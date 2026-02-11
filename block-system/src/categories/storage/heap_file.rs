//! Heap File Storage Block
//!
//! A heap file stores records in unordered, fixed-size pages. It is the
//! simplest and most common storage layout used by databases (e.g. PostgreSQL's
//! default table storage).
//!
//! ## How it works
//!
//! Records are appended to whichever page has enough free space. A **free-space
//! map** tracks how much room each page has so inserts don't have to scan every
//! page. Deletes mark slots as dead rather than physically removing data; a
//! future VACUUM-like compaction pass would reclaim the space.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `pages_read` | Counter | Pages accessed for reads/scans |
//! | `pages_written` | Counter | Pages written (inserts/deletes) |
//! | `records_inserted` | Counter | Total records inserted |
//! | `records_deleted` | Counter | Total records soft-deleted |
//! | `sequential_scans` | Counter | Number of full-table scans |
//! | `total_pages` | Gauge | Current page count |
//! | `total_live_records` | Gauge | Live (non-dead) records |
//! | `fragmentation_pct` | Gauge | Dead records / total records |

use async_trait::async_trait;
use std::collections::HashMap;

use crate::categories::TupleId;
use crate::core::block::{
    Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee, GuaranteeType};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// Internal page model
// ---------------------------------------------------------------------------

/// A slot on a page. Each slot holds one record.
#[derive(Debug, Clone)]
struct Slot {
    record: Record,
    is_dead: bool,
}

/// A fixed-size page containing a number of record slots.
#[derive(Debug, Clone)]
struct Page {
    page_id: usize,
    slots: Vec<Slot>,
    /// Estimated bytes used by live records on this page.
    used_bytes: usize,
}

impl Page {
    fn new(page_id: usize) -> Self {
        Self {
            page_id,
            slots: Vec::new(),
            used_bytes: 0,
        }
    }

    fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_dead).count()
    }

    fn dead_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_dead).count()
    }
}

// ---------------------------------------------------------------------------
// HeapFileBlock
// ---------------------------------------------------------------------------

/// Heap file storage block.
///
/// Stores records in fixed-size pages with a free-space map for efficient
/// insert placement.
pub struct HeapFileBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration (set during initialize)
    page_size: usize,
    fill_factor: f64,

    // Internal state
    pages: Vec<Page>,
    /// Estimated record size in bytes (computed from first insert).
    estimated_record_size: Option<usize>,
}

impl HeapFileBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            page_size: 8192,
            fill_factor: 0.9,
            pages: Vec::new(),
            estimated_record_size: None,
        }
    }

    // -- Metadata builders ---------------------------------------------------

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "heap-file-storage".into(),
            name: "Heap File Storage".into(),
            category: BlockCategory::Storage,
            description: "Stores records in unordered, fixed-size pages with a free-space map"
                .into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A heap file is the simplest table storage structure. Records are \
                           appended to pages in insertion order with no particular sorting."
                    .into(),
                algorithm: "Insert: find a page with space via free-space map, append slot. \
                            Scan: iterate all pages, skip dead slots. Delete: mark slot dead."
                    .into(),
                complexity: Complexity {
                    time: "Insert O(1) amortized, Scan O(n), Point lookup O(n) without index"
                        .into(),
                    space: "O(n) — one slot per record".into(),
                },
                use_cases: vec![
                    "Default table storage when no specific ordering is needed".into(),
                    "Write-heavy workloads where insert speed matters most".into(),
                    "Staging area before records are indexed".into(),
                ],
                tradeoffs: vec![
                    "Fast inserts but slow point lookups without an index".into(),
                    "Deletes cause fragmentation over time".into(),
                    "Sequential scans read dead tuples until vacuumed".into(),
                ],
                examples: vec![
                    "PostgreSQL heap tables".into(),
                    "MySQL InnoDB clustered index leaf pages (conceptually similar)".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 3: File Formats".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "database".into(),
            color: "#3B82F6".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "records".into(),
            name: "Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Stream of records to store".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "stored".into(),
            name: "Stored Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records after storage, enriched with _tuple_id".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "page_size".into(),
                name: "Page Size".into(),
                param_type: ParameterType::Number,
                description: "Size of each page in bytes".into(),
                default_value: ParameterValue::Integer(8192),
                required: false,
                constraints: Some(
                    ParameterConstraints::new()
                        .with_min(512.0)
                        .with_max(65536.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(512.0)
                        .with_unit("bytes".into()),
                ),
            },
            Parameter {
                id: "fill_factor".into(),
                name: "Fill Factor".into(),
                param_type: ParameterType::Number,
                description: "Fraction of page space to fill before allocating a new page".into(),
                default_value: ParameterValue::Number(0.9),
                required: false,
                constraints: Some(
                    ParameterConstraints::new()
                        .with_min(0.1)
                        .with_max(1.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(0.05)
                        .with_help_text("Lower values leave room for future updates".into()),
                ),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "pages_read".into(),
                name: "Pages Read".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Number of page reads".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "pages_written".into(),
                name: "Pages Written".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Number of page writes".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "records_inserted".into(),
                name: "Records Inserted".into(),
                metric_type: MetricType::Counter,
                unit: "records".into(),
                description: "Total records inserted".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "records_deleted".into(),
                name: "Records Deleted".into(),
                metric_type: MetricType::Counter,
                unit: "records".into(),
                description: "Total records soft-deleted".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "sequential_scans".into(),
                name: "Sequential Scans".into(),
                metric_type: MetricType::Counter,
                unit: "scans".into(),
                description: "Number of full sequential scans".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "total_pages".into(),
                name: "Total Pages".into(),
                metric_type: MetricType::Gauge,
                unit: "pages".into(),
                description: "Current number of pages".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "total_live_records".into(),
                name: "Live Records".into(),
                metric_type: MetricType::Gauge,
                unit: "records".into(),
                description: "Number of live (non-deleted) records".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "fragmentation_pct".into(),
                name: "Fragmentation".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Percentage of dead slots".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Estimate the byte size of a record (for page capacity simulation).
    fn estimate_record_size(record: &Record) -> usize {
        // Use JSON serialization length as a rough proxy for on-disk size.
        // Add a 16-byte slot header overhead.
        let json_size = serde_json::to_string(&record.data)
            .map(|s| s.len())
            .unwrap_or(64);
        json_size + 16
    }

    /// Maximum usable bytes on a page given fill_factor.
    fn usable_page_bytes(&self) -> usize {
        // Reserve a 24-byte page header.
        ((self.page_size - 24) as f64 * self.fill_factor) as usize
    }

    /// Find a page with enough free space, or allocate a new one.
    fn find_page_for_insert(&mut self, record_size: usize) -> usize {
        let usable = self.usable_page_bytes();
        // Free-space map scan — find first page with room.
        for page in &self.pages {
            if page.used_bytes + record_size <= usable {
                return page.page_id;
            }
        }
        // No page with room — allocate a new one.
        let new_id = self.pages.len();
        self.pages.push(Page::new(new_id));
        new_id
    }

    /// Insert a single record. Returns the TupleId.
    pub fn insert(&mut self, record: Record) -> TupleId {
        let rec_size = self
            .estimated_record_size
            .unwrap_or_else(|| Self::estimate_record_size(&record));
        if self.estimated_record_size.is_none() {
            self.estimated_record_size = Some(rec_size);
        }

        let page_id = self.find_page_for_insert(rec_size);
        let page = &mut self.pages[page_id];
        let slot_id = page.slots.len();
        page.slots.push(Slot {
            record,
            is_dead: false,
        });
        page.used_bytes += rec_size;

        TupleId::new(page_id, slot_id)
    }

    /// Get a record by TupleId. Returns None if out of range or dead.
    pub fn get(&self, tid: TupleId) -> Option<&Record> {
        let page = self.pages.get(tid.page_id)?;
        let slot = page.slots.get(tid.slot_id)?;
        if slot.is_dead {
            None
        } else {
            Some(&slot.record)
        }
    }

    /// Sequential scan — returns all live records with their TupleIds.
    pub fn scan(&self) -> Vec<(TupleId, &Record)> {
        let mut results = Vec::new();
        for page in &self.pages {
            for (slot_idx, slot) in page.slots.iter().enumerate() {
                if !slot.is_dead {
                    results.push((TupleId::new(page.page_id, slot_idx), &slot.record));
                }
            }
        }
        results
    }

    /// Soft-delete a record. Returns true if the record existed and was live.
    pub fn delete(&mut self, tid: TupleId) -> bool {
        if let Some(page) = self.pages.get_mut(tid.page_id) {
            if let Some(slot) = page.slots.get_mut(tid.slot_id) {
                if !slot.is_dead {
                    slot.is_dead = true;
                    return true;
                }
            }
        }
        false
    }

    /// Total number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Total live records across all pages.
    pub fn live_record_count(&self) -> usize {
        self.pages.iter().map(|p| p.live_count()).sum()
    }

    /// Fragmentation: dead / total slots as a percentage.
    pub fn fragmentation_pct(&self) -> f64 {
        let total: usize = self.pages.iter().map(|p| p.slots.len()).sum();
        if total == 0 {
            return 0.0;
        }
        let dead: usize = self.pages.iter().map(|p| p.dead_count()).sum();
        (dead as f64 / total as f64) * 100.0
    }
}

impl Default for HeapFileBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for HeapFileBlock {
    fn metadata(&self) -> &BlockMetadata {
        &self.metadata
    }

    fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.params
    }

    fn requires(&self) -> &[Constraint] {
        &[]
    }

    fn guarantees(&self) -> &[Guarantee] {
        // Declared as a static-lifetime slice so we can return a reference.
        static GUARANTEES: std::sync::LazyLock<Vec<Guarantee>> = std::sync::LazyLock::new(|| {
            vec![Guarantee::best_effort(
                GuaranteeType::Durability,
                "Records persist for the lifetime of the simulation",
            )]
        });
        &GUARANTEES
    }

    fn metrics(&self) -> &[MetricDefinition] {
        &self.metric_defs
    }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("page_size") {
            self.page_size = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("page_size must be an integer".into()))?
                as usize;
            if self.page_size < 512 || self.page_size > 65536 {
                return Err(BlockError::InvalidParameter(
                    "page_size must be between 512 and 65536".into(),
                ));
            }
        }
        if let Some(val) = params.get("fill_factor") {
            self.fill_factor = val
                .as_number()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("fill_factor must be a number".into())
                })?;
            if !(0.1..=1.0).contains(&self.fill_factor) {
                return Err(BlockError::InvalidParameter(
                    "fill_factor must be between 0.1 and 1.0".into(),
                ));
            }
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context
            .inputs
            .get("records")
            .cloned()
            .unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(recs) => recs,
            PortValue::Batch(recs) => recs,
            PortValue::Single(rec) => vec![rec],
            PortValue::None => Vec::new(),
            _ => {
                return Err(BlockError::InvalidInput(
                    "Expected DataStream, Batch, or Single record input".into(),
                ));
            }
        };

        let mut output_records = Vec::with_capacity(records.len());

        for record in records {
            let tid = self.insert(record.clone());
            context.metrics.increment("pages_written");
            context.metrics.increment("records_inserted");

            // Enrich the output record with the assigned tuple id.
            let mut out = record;
            let _ = out.insert("_page_id".into(), tid.page_id);
            let _ = out.insert("_slot_id".into(), tid.slot_id);
            output_records.push(out);
        }

        // Record gauges.
        context
            .metrics
            .record("total_pages", self.page_count() as f64);
        context
            .metrics
            .record("total_live_records", self.live_record_count() as f64);
        context
            .metrics
            .record("fragmentation_pct", self.fragmentation_pct());

        let mut outputs = HashMap::new();
        outputs.insert("stored".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert(
            "records_inserted".into(),
            context
                .metrics
                .aggregate("records_inserted", AggregationType::Sum)
                .unwrap_or(0.0),
        );
        metrics_summary.insert("total_pages".into(), self.page_count() as f64);
        metrics_summary.insert(
            "total_live_records".into(),
            self.live_record_count() as f64,
        );
        metrics_summary.insert("fragmentation_pct".into(), self.fragmentation_pct());

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("records") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => {
                    ValidationResult::ok()
                }
                PortValue::None => ValidationResult::ok()
                    .with_warning("No records provided — nothing to store"),
                _ => ValidationResult::error("records port expects DataStream, Batch, or Single"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("page_size".into(), self.page_size);
        let _ = state.insert("fill_factor".into(), self.fill_factor);
        let _ = state.insert("page_count".into(), self.page_count());
        let _ = state.insert("live_records".into(), self.live_record_count());
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(ps)) = state.get::<usize>("page_size") {
            self.page_size = ps;
        }
        if let Ok(Some(ff)) = state.get::<f64>("fill_factor") {
            self.fill_factor = ff;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: i64, name: &str) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), id).unwrap();
        r.insert("name".into(), name).unwrap();
        r
    }

    #[test]
    fn test_insert_and_get() {
        let mut heap = HeapFileBlock::new();
        let tid = heap.insert(make_record(1, "Alice"));

        let rec = heap.get(tid).expect("record should exist");
        assert_eq!(rec.get::<i64>("id").unwrap(), Some(1));
        assert_eq!(rec.get::<String>("name").unwrap(), Some("Alice".into()));
    }

    #[test]
    fn test_page_allocation() {
        let mut heap = HeapFileBlock::new();
        // With default 8 KB pages and ~50-byte records,
        // many records should fit on the first page.
        for i in 0..10 {
            heap.insert(make_record(i, "user"));
        }
        assert_eq!(heap.page_count(), 1, "10 small records should fit on one page");

        // Force tiny pages to trigger multi-page allocation.
        let mut tiny = HeapFileBlock::new();
        tiny.page_size = 512;
        tiny.fill_factor = 0.5;
        for i in 0..100 {
            tiny.insert(make_record(i, "user"));
        }
        assert!(
            tiny.page_count() > 1,
            "100 records in 512-byte pages should span multiple pages"
        );
    }

    #[test]
    fn test_sequential_scan() {
        let mut heap = HeapFileBlock::new();
        for i in 0..50 {
            heap.insert(make_record(i, &format!("user_{}", i)));
        }

        let results = heap.scan();
        assert_eq!(results.len(), 50);
    }

    #[test]
    fn test_delete_marks_dead() {
        let mut heap = HeapFileBlock::new();
        let _t1 = heap.insert(make_record(1, "Alice"));
        let t2 = heap.insert(make_record(2, "Bob"));
        let _t3 = heap.insert(make_record(3, "Charlie"));

        assert!(heap.delete(t2));
        assert!(heap.get(t2).is_none(), "deleted record should not be returned");

        let scan = heap.scan();
        assert_eq!(scan.len(), 2, "scan should skip dead records");

        // Double-delete returns false.
        assert!(!heap.delete(t2));
    }

    #[test]
    fn test_fragmentation() {
        let mut heap = HeapFileBlock::new();
        let mut tids = Vec::new();
        for i in 0..10 {
            tids.push(heap.insert(make_record(i, "user")));
        }
        assert_eq!(heap.fragmentation_pct(), 0.0);

        heap.delete(tids[0]);
        heap.delete(tids[5]);
        // 2 dead out of 10 = 20%
        assert!((heap.fragmentation_pct() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_fill_factor_respected() {
        let mut heap = HeapFileBlock::new();
        heap.page_size = 512;
        heap.fill_factor = 0.5;
        // Usable bytes = (512 - 24) * 0.5 = 244 bytes
        // Each record ~50 bytes → ~4 records per page
        for i in 0..20 {
            heap.insert(make_record(i, "x"));
        }
        // With 4 records/page, 20 records → ~5 pages
        assert!(
            heap.page_count() >= 3,
            "fill factor should cause earlier page splits"
        );
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut heap = HeapFileBlock::new();
        heap.initialize(HashMap::new()).await.unwrap();

        let records: Vec<Record> = (0..5)
            .map(|i| make_record(i, &format!("user_{}", i)))
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = heap.execute(ctx).await.unwrap();
        assert!(result.errors.is_empty());

        let stored = result.outputs.get("stored").unwrap();
        assert_eq!(stored.len(), 5);

        assert_eq!(*result.metrics.get("total_live_records").unwrap(), 5.0);
    }

    #[test]
    fn test_metadata() {
        let heap = HeapFileBlock::new();
        assert_eq!(heap.metadata().id, "heap-file-storage");
        assert_eq!(heap.metadata().category, BlockCategory::Storage);
        assert_eq!(heap.inputs().len(), 1);
        assert_eq!(heap.outputs().len(), 1);
        assert_eq!(heap.parameters().len(), 2);
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut heap = HeapFileBlock::new();
        let mut params = HashMap::new();
        params.insert("page_size".into(), ParameterValue::Integer(4096));
        params.insert("fill_factor".into(), ParameterValue::Number(0.75));

        heap.initialize(params).await.unwrap();
        assert_eq!(heap.page_size, 4096);
        assert!((heap.fill_factor - 0.75).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_initialize_rejects_bad_params() {
        let mut heap = HeapFileBlock::new();

        let mut params = HashMap::new();
        params.insert("page_size".into(), ParameterValue::Integer(100));
        assert!(heap.initialize(params).await.is_err());

        let mut params2 = HashMap::new();
        params2.insert("fill_factor".into(), ParameterValue::Number(1.5));
        assert!(heap.initialize(params2).await.is_err());
    }
}
