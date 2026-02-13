//! LRU Buffer Pool Block
//!
//! A fixed-size page cache that sits between the execution layer and storage.
//! Pages are evicted in **Least Recently Used** (LRU) order when the buffer
//! pool is full.
//!
//! ## How it works
//!
//! The buffer pool maintains a hash map from page IDs to page data, plus an
//! LRU ordering structure (a doubly-linked list simulated with a `VecDeque`).
//! On every `get_page` call:
//! - **Hit**: the page is moved to the most-recently-used position.
//! - **Miss**: the page is "fetched" (simulated) and inserted. If the pool is
//!   full, the least-recently-used page is evicted first.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `cache_hits` | Counter | Number of page requests served from cache |
//! | `cache_misses` | Counter | Number of page requests that missed |
//! | `hit_rate_pct` | Gauge | cache_hits / (hits + misses) * 100 |
//! | `evictions` | Counter | Pages evicted to make room |
//! | `current_size` | Gauge | Pages currently in the pool |

use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};

use crate::core::block::{
    Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue, Record};

// ---------------------------------------------------------------------------
// LRUBufferBlock
// ---------------------------------------------------------------------------

pub struct LRUBufferBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    pub(crate) capacity: usize, // max pages
    page_size: usize,

    // Internal state
    /// page_id → page data (simulated as a Vec<u8>)
    cache: HashMap<usize, Vec<u8>>,
    /// LRU order: front = least recently used, back = most recently used
    lru_order: VecDeque<usize>,

    // Stats
    hits: usize,
    misses: usize,
    evictions: usize,
}

impl LRUBufferBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            capacity: 1024,
            page_size: 8192,
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    // -- Metadata builders ---------------------------------------------------

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "lru-buffer-pool".into(),
            name: "LRU Buffer Pool".into(),
            category: BlockCategory::Buffer,
            description: "Fixed-size page cache with LRU eviction".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A buffer pool caches frequently accessed pages in memory to avoid \
                           repeated reads from storage. LRU eviction ensures the least \
                           recently accessed page is removed when the pool is full."
                    .into(),
                algorithm: "On access: if page is in cache (hit), move it to MRU position. \
                            If not (miss), evict LRU page if pool is full, then insert the \
                            new page at MRU position."
                    .into(),
                complexity: Complexity {
                    time: "O(n) per access in this VecDeque implementation; O(1) with a \
                           proper doubly-linked-list + hash map"
                        .into(),
                    space: "O(capacity) — at most `capacity` pages held in memory".into(),
                },
                use_cases: vec![
                    "Reducing storage I/O for hot data".into(),
                    "Caching index pages during lookups".into(),
                    "Shared buffer pool for multiple table/index accesses".into(),
                ],
                tradeoffs: vec![
                    "LRU can be fooled by sequential scans (see Clock or LRU-K)".into(),
                    "Larger pool = more memory, but better hit rate".into(),
                    "Simple LRU doesn't distinguish between scan and point access".into(),
                ],
                examples: vec![
                    "PostgreSQL shared_buffers".into(),
                    "MySQL InnoDB buffer pool".into(),
                    "SQLite page cache".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 5: Buffer Management"
                    .into(),
                url: None,
                citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
            }],
            icon: "layers".into(),
            color: "#F59E0B".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "requests".into(),
            name: "Page Requests".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records with a `_page_id` field identifying the requested page".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "pages".into(),
            name: "Served Pages".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records enriched with `_cache_hit` (bool) and `_page_data_size`".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "size".into(),
                name: "Pool Size".into(),
                param_type: ParameterType::Number,
                description: "Maximum number of pages to cache".into(),
                default_value: ParameterValue::Integer(1024),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(1.0).with_max(1_000_000.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(64.0)
                        .with_unit("pages".into()),
                ),
            },
            Parameter {
                id: "page_size".into(),
                name: "Page Size".into(),
                param_type: ParameterType::Number,
                description: "Size of each page in bytes (for memory accounting)".into(),
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
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "cache_hits".into(),
                name: "Cache Hits".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Page requests served from cache".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "cache_misses".into(),
                name: "Cache Misses".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Page requests that missed the cache".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "hit_rate_pct".into(),
                name: "Hit Rate".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Percentage of requests served from cache".into(),
                aggregations: vec![AggregationType::Avg],
            },
            MetricDefinition {
                id: "evictions".into(),
                name: "Evictions".into(),
                metric_type: MetricType::Counter,
                unit: "pages".into(),
                description: "Pages evicted from cache".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "current_size".into(),
                name: "Current Size".into(),
                metric_type: MetricType::Gauge,
                unit: "pages".into(),
                description: "Pages currently in the buffer pool".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Request a page. Returns `true` if it was a cache hit.
    pub fn get_page(&mut self, page_id: usize) -> bool {
        if self.cache.contains_key(&page_id) {
            // Hit — move to MRU position.
            self.touch(page_id);
            self.hits += 1;
            true
        } else {
            // Miss — possibly evict, then insert.
            if self.cache.len() >= self.capacity {
                self.evict();
            }
            // Simulate fetching the page (fill with zeros).
            self.cache.insert(page_id, vec![0u8; self.page_size]);
            self.lru_order.push_back(page_id);
            self.misses += 1;
            false
        }
    }

    /// Move a page to the most-recently-used position.
    fn touch(&mut self, page_id: usize) {
        if let Some(pos) = self.lru_order.iter().position(|&id| id == page_id) {
            self.lru_order.remove(pos);
        }
        self.lru_order.push_back(page_id);
    }

    /// Evict the least recently used page.
    fn evict(&mut self) {
        if let Some(victim) = self.lru_order.pop_front() {
            self.cache.remove(&victim);
            self.evictions += 1;
        }
    }

    /// Current number of cached pages.
    pub fn current_size(&self) -> usize {
        self.cache.len()
    }

    /// Hit rate as a percentage (0–100).
    pub fn hit_rate_pct(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    /// Total memory used by cached pages.
    pub fn memory_used(&self) -> usize {
        self.cache.len() * self.page_size
    }

    /// Check if a specific page is cached.
    pub fn contains(&self, page_id: usize) -> bool {
        self.cache.contains_key(&page_id)
    }

    /// Clear the entire buffer pool.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
    }
}

impl Default for LRUBufferBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for LRUBufferBlock {
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
        if let Some(val) = params.get("size") {
            self.capacity = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("size must be an integer".into()))?
                as usize;
            if self.capacity == 0 {
                return Err(BlockError::InvalidParameter(
                    "size must be at least 1".into(),
                ));
            }
        }
        if let Some(val) = params.get("page_size") {
            self.page_size = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("page_size must be an integer".into())
                })?
                as usize;
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context
            .inputs
            .get("requests")
            .cloned()
            .unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => {
                return Err(BlockError::InvalidInput(
                    "Expected DataStream, Batch, or Single".into(),
                ))
            }
        };

        let mut output_records = Vec::with_capacity(records.len());

        for record in records {
            let page_id = record
                .get::<usize>("_page_id")
                .ok()
                .flatten()
                .unwrap_or(0);

            let hit = self.get_page(page_id);

            if hit {
                context.metrics.increment("cache_hits");
            } else {
                context.metrics.increment("cache_misses");
            }

            let mut out = record;
            let _ = out.insert("_cache_hit".into(), hit);
            let _ = out.insert("_page_data_size".into(), self.page_size);
            output_records.push(out);
        }

        // Record gauges.
        context
            .metrics
            .record("hit_rate_pct", self.hit_rate_pct());
        context
            .metrics
            .record("evictions", self.evictions as f64);
        context
            .metrics
            .record("current_size", self.current_size() as f64);

        let mut outputs = HashMap::new();
        outputs.insert("pages".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("cache_hits".into(), self.hits as f64);
        metrics_summary.insert("cache_misses".into(), self.misses as f64);
        metrics_summary.insert("hit_rate_pct".into(), self.hit_rate_pct());
        metrics_summary.insert("evictions".into(), self.evictions as f64);
        metrics_summary.insert("current_size".into(), self.current_size() as f64);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("requests") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => {
                    ValidationResult::ok()
                }
                PortValue::None => {
                    ValidationResult::ok().with_warning("No page requests provided")
                }
                _ => ValidationResult::error("requests port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("requests input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("capacity".into(), self.capacity);
        let _ = state.insert("page_size".into(), self.page_size);
        let _ = state.insert("current_size".into(), self.current_size());
        let _ = state.insert("hits".into(), self.hits);
        let _ = state.insert("misses".into(), self.misses);
        let _ = state.insert("evictions".into(), self.evictions);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(c)) = state.get::<usize>("capacity") {
            self.capacity = c;
        }
        if let Ok(Some(ps)) = state.get::<usize>("page_size") {
            self.page_size = ps;
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

    #[test]
    fn test_basic_hit_and_miss() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 4;

        // First access is always a miss.
        assert!(!pool.get_page(1));
        assert_eq!(pool.misses, 1);

        // Second access to the same page is a hit.
        assert!(pool.get_page(1));
        assert_eq!(pool.hits, 1);
    }

    #[test]
    fn test_100_pct_hit_rate() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 10;

        // Load pages 0..5
        for i in 0..5 {
            pool.get_page(i);
        }
        // All misses so far.
        assert_eq!(pool.misses, 5);
        assert_eq!(pool.hits, 0);

        // Access same pages again — all hits.
        for i in 0..5 {
            assert!(pool.get_page(i));
        }
        assert_eq!(pool.hits, 5);
        // hit_rate = 5 / 10 = 50% (across entire lifetime)
        assert!((pool.hit_rate_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_eviction_lru_order() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 3;

        // Fill the pool: pages 1, 2, 3
        pool.get_page(1); // miss
        pool.get_page(2); // miss
        pool.get_page(3); // miss
        assert_eq!(pool.current_size(), 3);
        assert_eq!(pool.evictions, 0);

        // Access page 1 to make it MRU. LRU order: 2, 3, 1
        pool.get_page(1); // hit

        // Insert page 4 — should evict page 2 (LRU).
        pool.get_page(4); // miss, evicts 2
        assert_eq!(pool.evictions, 1);
        assert!(!pool.contains(2), "Page 2 should have been evicted");
        assert!(pool.contains(1));
        assert!(pool.contains(3));
        assert!(pool.contains(4));
    }

    #[test]
    fn test_eviction_cascading() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 2;

        pool.get_page(1); // miss
        pool.get_page(2); // miss
        pool.get_page(3); // miss, evicts 1
        pool.get_page(4); // miss, evicts 2

        assert_eq!(pool.evictions, 2);
        assert!(!pool.contains(1));
        assert!(!pool.contains(2));
        assert!(pool.contains(3));
        assert!(pool.contains(4));
    }

    #[test]
    fn test_hit_rate_accuracy() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 100;

        // 100 unique misses.
        for i in 0..100 {
            pool.get_page(i);
        }
        // 100 hits on the same pages.
        for i in 0..100 {
            pool.get_page(i);
        }

        assert_eq!(pool.hits, 100);
        assert_eq!(pool.misses, 100);
        assert!((pool.hit_rate_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_clear() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 10;

        for i in 0..5 {
            pool.get_page(i);
        }
        assert_eq!(pool.current_size(), 5);

        pool.clear();
        assert_eq!(pool.current_size(), 0);
    }

    #[test]
    fn test_memory_used() {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 10;
        pool.page_size = 4096;

        pool.get_page(0);
        pool.get_page(1);
        pool.get_page(2);

        assert_eq!(pool.memory_used(), 3 * 4096);
    }

    #[test]
    fn test_metadata() {
        let pool = LRUBufferBlock::new();
        assert_eq!(pool.metadata().id, "lru-buffer-pool");
        assert_eq!(pool.metadata().category, BlockCategory::Buffer);
        assert_eq!(pool.inputs().len(), 1);
        assert_eq!(pool.outputs().len(), 1);
        assert_eq!(pool.parameters().len(), 2);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut pool = LRUBufferBlock::new();
        pool.capacity = 4;

        // Create requests: pages 0, 1, 2, 0, 1, 3 (0 and 1 should hit on second access)
        let page_ids = vec![0, 1, 2, 0, 1, 3];
        let records: Vec<Record> = page_ids
            .iter()
            .map(|&pid| {
                let mut r = Record::new();
                r.insert("_page_id".into(), pid as usize).unwrap();
                r
            })
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("requests".into(), PortValue::Stream(records));

        let ctx = ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        };

        let result = pool.execute(ctx).await.unwrap();
        assert!(result.errors.is_empty());

        // 3 misses (0, 1, 2 first time) + 1 miss (3) = 4 misses, 2 hits (0, 1 second time)
        assert_eq!(*result.metrics.get("cache_hits").unwrap(), 2.0);
        assert_eq!(*result.metrics.get("cache_misses").unwrap(), 4.0);

        let pages_output = result.outputs.get("pages").unwrap();
        assert_eq!(pages_output.len(), 6);
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut pool = LRUBufferBlock::new();
        let mut params = HashMap::new();
        params.insert("size".into(), ParameterValue::Integer(256));
        params.insert("page_size".into(), ParameterValue::Integer(4096));

        pool.initialize(params).await.unwrap();
        assert_eq!(pool.capacity, 256);
        assert_eq!(pool.page_size, 4096);
    }

    #[tokio::test]
    async fn test_initialize_rejects_zero_size() {
        let mut pool = LRUBufferBlock::new();
        let mut params = HashMap::new();
        params.insert("size".into(), ParameterValue::Integer(0));
        assert!(pool.initialize(params).await.is_err());
    }
}
