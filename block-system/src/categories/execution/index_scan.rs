//! Index Scan Execution Block
//!
//! Uses an index to look up matching records, then fetches the full rows from
//! storage. This avoids reading every page — only the pages containing matching
//! records are accessed.
//!
//! ## How it works
//!
//! 1. Receive indexed lookup results (TupleIds from an index block).
//! 2. For each TupleId, fetch the full record from the storage input.
//! 3. Return only the fetched records.
//!
//! In this simulation, the index scan block receives both index results and
//! stored data, then matches them by `_page_id` and `_slot_id` fields.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `index_hits` | Counter | Records found via index |
//! | `pages_read` | Counter | Distinct pages accessed |
//! | `rows_returned` | Counter | Rows returned to caller |
//! | `random_ios` | Counter | Simulated random I/O operations |

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// IndexScanBlock
// ---------------------------------------------------------------------------

pub struct IndexScanBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    limit: Option<usize>,
}

impl IndexScanBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            limit: None,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "index-scan".into(),
            name: "Index Scan".into(),
            category: BlockCategory::Execution,
            description: "Uses an index to fetch only matching records from storage".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "An index scan uses a secondary index (such as a B-tree or hash index) \
                           to find the exact locations of matching records, then fetches only \
                           those records from the storage layer. Instead of reading every page \
                           in the table, it reads only the pages that contain matching rows.\n\n\
                           In a database system, the query optimizer chooses an index scan when \
                           it estimates that the query is selective enough — meaning only a small \
                           fraction of rows match the predicate. The index provides a shortcut: \
                           it maps search keys to TupleIds (page_id, slot_id), which tell the \
                           storage layer exactly where to find each matching row.\n\n\
                           Think of it like using the index at the back of a textbook: instead \
                           of reading every page to find references to 'B-tree', you look up \
                           'B-tree' in the index, get the page numbers, and flip directly to \
                           those pages. This is fast when there are few references, but slower \
                           than reading straight through if almost every page contains the term."
                    .into(),
                algorithm: "Index Scan Algorithm:\n\
                            \n\
                            FUNCTION index_scan(index_results, storage, limit):\n  \
                              // Phase 1: Collect TupleIds from index\n  \
                              index_set = SET of (page_id, slot_id) from index_results\n  \
                              pages_accessed = empty SET\n  \
                              matched = []\n  \
                              \n  \
                              // Phase 2: Fetch matching records from storage\n  \
                              FOR EACH record IN storage:\n    \
                                pid = record._page_id\n    \
                                sid = record._slot_id\n    \
                                IF (pid, sid) IN index_set:\n      \
                                  pages_accessed.add(pid)\n      \
                                  matched.append(record)\n      \
                                  IF limit > 0 AND matched.len >= limit:\n        \
                                    BREAK\n  \
                              \n  \
                              random_ios = pages_accessed.size  // each distinct page = 1 I/O\n  \
                              RETURN matched"
                    .into(),
                complexity: Complexity {
                    time: "O(k × log n) for k results using a B-tree index, O(k) with hash index"
                        .into(),
                    space: "O(k) — only matching records buffered".into(),
                },
                use_cases: vec![
                    "Selective queries (WHERE id = ?, WHERE price < 100)".into(),
                    "Queries that return a small fraction of the table".into(),
                    "Covering index queries".into(),
                    "Primary key lookups (WHERE id = 42) — the most common index scan".into(),
                    "Range queries on indexed columns (WHERE date BETWEEN '2024-01-01' AND '2024-01-31')".into(),
                ],
                tradeoffs: vec![
                    "Fast for selective queries but slower than seq scan for non-selective ones".into(),
                    "Random I/O pattern (one page fetch per matched record) is less cache-friendly".into(),
                    "Requires a compatible index to exist".into(),
                    "Each matched row may be on a different page, causing many small random reads \
                     rather than a few large sequential reads — the cost per row is higher".into(),
                    "Index-only scans (covering indexes) can avoid the heap fetch entirely, \
                     returning data directly from the index leaf pages".into(),
                ],
                examples: vec![
                    "PostgreSQL Index Scan / Index Only Scan — the planner estimates row count \
                     and chooses index scan when selectivity is low enough".into(),
                    "MySQL InnoDB secondary index lookup + clustered index fetch — secondary \
                     index leaves contain the primary key, requiring a second B-tree traversal".into(),
                    "SQLite index scan — walks the index B-tree, then fetches rows by rowid".into(),
                    "Oracle index range scan — efficiently handles BETWEEN and inequality predicates".into(),
                ],
                motivation: "Without index scans, every query would require a full table scan, \
                             reading every page even when only a handful of rows match. For a \
                             table with millions of rows, finding a single row by ID would mean \
                             reading the entire table — an O(n) operation that could take seconds \
                             or minutes.\n\n\
                             Index scans turn selective lookups into O(log n) or O(1) operations \
                             by using a prebuilt data structure to jump directly to the matching \
                             rows. They are the reason database queries can return results in \
                             milliseconds even on tables with billions of rows."
                    .into(),
                parameter_guide: HashMap::from([
                    ("limit".into(), "Maximum number of records to return, simulating a SQL LIMIT \
                                      clause. Set to 0 for unlimited. When set, the scan stops \
                                      early after finding enough matches, which can dramatically \
                                      reduce the number of page reads. This is especially powerful \
                                      with sorted indexes: 'SELECT * FROM users ORDER BY created_at \
                                      DESC LIMIT 10' only needs to read the last 10 index entries. \
                                      Try setting limit to 1, 10, and 100 to see how it affects \
                                      pages_read and random_ios.".into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "sequential-scan".into(),
                        comparison: "A sequential scan reads every page in order — O(n) but with \
                                     efficient sequential I/O. An index scan reads only matching \
                                     pages — O(k) but with expensive random I/O. The crossover \
                                     point is typically 5-15% selectivity: if more than ~10% of \
                                     rows match, the sequential scan's efficient I/O pattern often \
                                     wins. Databases choose automatically based on cost estimation.".into(),
                    },
                ],
                suggested_questions: vec![
                    "Why does MySQL InnoDB require a 'double lookup' for secondary index scans, \
                     and how do covering indexes avoid this?".into(),
                    "What is the difference between an Index Scan and an Index Only Scan in \
                     PostgreSQL, and when can the planner use each?".into(),
                    "How does the correlation between index order and physical row order affect \
                     index scan performance?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 5: Query Processing".into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "search".into(),
            color: "#14B8A6".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![
            Port {
                id: "records".into(),
                name: "Storage Records".into(),
                port_type: PortType::DataStream,
                direction: PortDirection::Input,
                required: true,
                multiple: false,
                description: "Full records from storage (with _page_id, _slot_id)".into(),
                schema: None,
            },
            Port {
                id: "index_results".into(),
                name: "Index Results".into(),
                port_type: PortType::DataStream,
                direction: PortDirection::Input,
                required: false,
                multiple: false,
                description: "TupleIds from index lookup (with _page_id, _slot_id)".into(),
                schema: None,
            },
        ]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "results".into(),
            name: "Fetched Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records fetched via index lookup".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "limit".into(),
            name: "Limit".into(),
            param_type: ParameterType::Number,
            description: "Maximum number of records to return (0 = unlimited)".into(),
            default_value: ParameterValue::Integer(0),
            required: false,
            constraints: None,
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Input)
                    .with_help_text("Simulates LIMIT clause".into()),
            ),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "index_hits".into(),
                name: "Index Hits".into(),
                metric_type: MetricType::Counter,
                unit: "records".into(),
                description: "Records found via index".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "pages_read".into(),
                name: "Pages Read".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Distinct pages accessed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "rows_returned".into(),
                name: "Rows Returned".into(),
                metric_type: MetricType::Counter,
                unit: "rows".into(),
                description: "Rows returned to caller".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "random_ios".into(),
                name: "Random I/Os".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Random I/O operations (page fetches)".into(),
                aggregations: vec![AggregationType::Sum],
            },
        ]
    }
}

impl Default for IndexScanBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for IndexScanBlock {
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
        &[]
    }

    fn metrics(&self) -> &[MetricDefinition] {
        &self.metric_defs
    }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("limit") {
            let v = val.as_integer().ok_or_else(|| {
                BlockError::InvalidParameter("limit must be an integer".into())
            })? as usize;
            if v > 0 {
                self.limit = Some(v);
            }
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        // Get storage records.
        let storage_records = match context.inputs.get("records").cloned().unwrap_or(PortValue::None)
        {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => {
                return Err(BlockError::InvalidInput(
                    "Expected DataStream for records".into(),
                ))
            }
        };

        // Get index results (if connected).
        let index_records =
            match context.inputs.get("index_results").cloned().unwrap_or(PortValue::None) {
                PortValue::Stream(r) => r,
                PortValue::Batch(r) => r,
                PortValue::Single(r) => vec![r],
                PortValue::None => Vec::new(),
                _ => Vec::new(),
            };

        let results;
        let mut pages_accessed = HashSet::new();

        if index_records.is_empty() {
            // No index input — pass through all storage records (like a seq scan fallback).
            for rec in &storage_records {
                if let Ok(Some(pid)) = rec.get::<usize>("_page_id") {
                    pages_accessed.insert(pid);
                }
            }
            results = storage_records;
        } else {
            // Build a lookup set from index results: (page_id, slot_id).
            let mut index_set: HashSet<(usize, usize)> = HashSet::new();
            for idx_rec in &index_records {
                let page_id = idx_rec.get::<usize>("_page_id").ok().flatten().unwrap_or(0);
                let slot_id = idx_rec.get::<usize>("_slot_id").ok().flatten().unwrap_or(0);
                index_set.insert((page_id, slot_id));
            }

            // Match storage records against index set.
            let mut matched = Vec::new();
            for rec in &storage_records {
                let page_id = rec.get::<usize>("_page_id").ok().flatten().unwrap_or(0);
                let slot_id = rec.get::<usize>("_slot_id").ok().flatten().unwrap_or(0);
                if index_set.contains(&(page_id, slot_id)) {
                    pages_accessed.insert(page_id);
                    matched.push(rec.clone());
                    if let Some(lim) = self.limit {
                        if matched.len() >= lim {
                            break;
                        }
                    }
                }
            }
            results = matched;
        }

        let index_hits = results.len();
        let distinct_pages = pages_accessed.len();

        context.metrics.record("index_hits", index_hits as f64);
        context
            .metrics
            .record("pages_read", distinct_pages as f64);
        context
            .metrics
            .record("rows_returned", results.len() as f64);
        context
            .metrics
            .record("random_ios", distinct_pages as f64);

        let mut outputs = HashMap::new();
        outputs.insert("results".into(), PortValue::Stream(results));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("index_hits".into(), index_hits as f64);
        metrics_summary.insert("pages_read".into(), distinct_pages as f64);
        metrics_summary.insert("rows_returned".into(), index_hits as f64);
        metrics_summary.insert("random_ios".into(), distinct_pages as f64);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if inputs.get("records").is_none() {
            ValidationResult::ok().with_warning("storage records input not connected")
        } else {
            ValidationResult::ok()
        }
    }

    fn get_state(&self) -> BlockState {
        BlockState::new()
    }

    fn set_state(&mut self, _state: BlockState) -> Result<(), BlockError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_storage_records(n: usize) -> Vec<Record> {
        (0..n)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("name".into(), format!("user_{}", i)).unwrap();
                r.insert("_page_id".into(), (i / 10) as usize).unwrap();
                r.insert("_slot_id".into(), i as usize).unwrap();
                r
            })
            .collect()
    }

    fn make_index_results(ids: &[usize]) -> Vec<Record> {
        ids.iter()
            .map(|&i| {
                let mut r = Record::new();
                r.insert("_page_id".into(), (i / 10) as usize).unwrap();
                r.insert("_slot_id".into(), i as usize).unwrap();
                r
            })
            .collect()
    }

    #[tokio::test]
    async fn test_index_scan_with_results() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut scan = IndexScanBlock::new();
        scan.initialize(HashMap::new()).await.unwrap();

        let storage = make_storage_records(100);
        let index = make_index_results(&[5, 15, 25, 50, 99]);

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(storage));
        inputs.insert("index_results".into(), PortValue::Stream(index));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("index_hits").unwrap(), 5.0);
        // Pages: 5→page 0, 15→page 1, 25→page 2, 50→page 5, 99→page 9 = 5 distinct
        assert_eq!(*result.metrics.get("pages_read").unwrap(), 5.0);
    }

    #[tokio::test]
    async fn test_index_scan_no_index_input() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut scan = IndexScanBlock::new();

        let storage = make_storage_records(50);
        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(storage));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        // Falls back to returning all records.
        assert_eq!(*result.metrics.get("rows_returned").unwrap(), 50.0);
    }

    #[tokio::test]
    async fn test_index_scan_with_limit() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut scan = IndexScanBlock::new();
        scan.limit = Some(2);

        let storage = make_storage_records(100);
        let index = make_index_results(&[10, 20, 30, 40, 50]);

        let mut inputs = HashMap::new();
        inputs.insert("records".into(), PortValue::Stream(storage));
        inputs.insert("index_results".into(), PortValue::Stream(index));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = scan.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("rows_returned").unwrap(), 2.0);
    }

    #[test]
    fn test_metadata() {
        let scan = IndexScanBlock::new();
        assert_eq!(scan.metadata().id, "index-scan");
        assert_eq!(scan.metadata().category, BlockCategory::Execution);
        assert_eq!(scan.inputs().len(), 2);
        assert_eq!(scan.outputs().len(), 1);
    }
}
