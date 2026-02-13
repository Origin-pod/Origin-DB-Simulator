//! Clock Buffer Pool Block
//!
//! A fixed-size page cache using the **CLOCK** (second-chance) eviction algorithm.
//! The clock hand sweeps through pages: if the reference bit is set, it clears it
//! and moves on; if the reference bit is unset, that page is evicted.
//!
//! ## Why CLOCK over LRU?
//!
//! CLOCK approximates LRU with O(1) eviction (no list reordering on every access).
//! PostgreSQL uses a clock-sweep algorithm for its shared buffer pool because it
//! performs well under concurrent access without the overhead of maintaining a
//! strict LRU order.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `cache_hits` | Counter | Page requests served from cache |
//! | `cache_misses` | Counter | Page requests that missed |
//! | `hit_rate_pct` | Gauge | cache_hits / (hits + misses) * 100 |
//! | `evictions` | Counter | Pages evicted to make room |
//! | `clock_hand_sweeps` | Counter | Full rotations of the clock hand |
//! | `current_size` | Gauge | Pages currently in the pool |

use async_trait::async_trait;
use std::collections::HashMap;

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
    Complexity, ExecutionContext, ExecutionResult, Reference, ReferenceType,
};
use crate::core::constraint::{Constraint, Guarantee};
use crate::core::metrics::{AggregationType, MetricDefinition, MetricType};
use crate::core::parameter::{
    Parameter, ParameterConstraints, ParameterType, ParameterUIHint, ParameterValue,
    ValidationResult, WidgetType,
};
use crate::core::port::{Port, PortDirection, PortType, PortValue};

// ---------------------------------------------------------------------------
// ClockBufferBlock
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ClockEntry {
    page_id: usize,
    reference_bit: bool,
}

pub struct ClockBufferBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    capacity: usize,
    page_size: usize,

    // Internal state — circular buffer with clock hand
    pages: Vec<Option<ClockEntry>>,
    /// Maps page_id → slot index for O(1) lookup
    page_map: HashMap<usize, usize>,
    clock_hand: usize,

    // Stats
    hits: usize,
    misses: usize,
    evictions: usize,
    clock_hand_sweeps: usize,
}

impl ClockBufferBlock {
    pub fn new() -> Self {
        let capacity = 1024;
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            capacity,
            page_size: 8192,
            pages: vec![None; capacity],
            page_map: HashMap::new(),
            clock_hand: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            clock_hand_sweeps: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "clock-buffer-pool".into(),
            name: "Clock Buffer Pool".into(),
            category: BlockCategory::Buffer,
            description: "Page cache with CLOCK (second-chance) eviction — used by PostgreSQL".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "The CLOCK algorithm (also known as second-chance) is a page \
                           replacement policy that approximates LRU behavior without the \
                           overhead of maintaining an explicit access-order list. Instead \
                           of tracking the exact order of accesses, each page gets a single \
                           reference bit that is set whenever the page is accessed.\n\n\
                           When eviction is needed, a clock hand sweeps around a circular \
                           buffer. If a page's reference bit is set, the algorithm gives it \
                           a 'second chance' by clearing the bit and moving on. If the bit \
                           is already clear, the page is evicted. This means recently \
                           accessed pages survive at least one full sweep before eviction.\n\n\
                           PostgreSQL chose CLOCK over strict LRU for its shared buffer pool \
                           because setting a bit is an atomic operation that requires no lock \
                           contention, while moving a node in a linked list requires exclusive \
                           access. Under hundreds of concurrent connections, this difference \
                           is significant."
                    .into(),
                algorithm: "CLOCK Buffer Pool Algorithm:\n\
                            \n\
                            STRUCTURE: Circular array of slots, each with (page_id, ref_bit)\n\
                            STATE: clock_hand pointing to current slot\n\
                            \n\
                            FUNCTION get_page(page_id):\n  \
                              IF page_id IN page_map:\n    \
                                // Cache HIT — just set the reference bit\n    \
                                slot = page_map[page_id]\n    \
                                slots[slot].ref_bit = true\n    \
                                RETURN page_data\n  \
                              ELSE:\n    \
                                // Cache MISS — need to find a slot\n    \
                                IF pool is full:\n      \
                                  CALL evict_one()\n    \
                                slot = find_empty_slot()\n    \
                                slots[slot] = (page_id, ref_bit=true)\n    \
                                RETURN fetched page_data\n\
                            \n\
                            FUNCTION evict_one():\n  \
                              LOOP:\n    \
                                entry = slots[clock_hand]\n    \
                                IF entry.ref_bit == true:\n      \
                                  entry.ref_bit = false   // second chance\n      \
                                  advance clock_hand\n    \
                                ELSE:\n      \
                                  EVICT entry.page_id\n      \
                                  advance clock_hand\n      \
                                  RETURN"
                    .into(),
                complexity: Complexity {
                    time: "O(1) amortized per access; worst case O(n) sweep on eviction".into(),
                    space: "O(capacity) — fixed-size circular buffer".into(),
                },
                use_cases: vec![
                    "PostgreSQL shared_buffers replacement policy".into(),
                    "High-concurrency buffer pools where LRU reordering is expensive".into(),
                    "Systems with mixed scan and point-lookup workloads".into(),
                    "Embedded databases that need a simple, low-overhead cache policy".into(),
                    "Operating system virtual memory page replacement".into(),
                ],
                tradeoffs: vec![
                    "Less precise than true LRU but much cheaper under concurrency".into(),
                    "Sequential scans can pollute the cache (same as LRU)".into(),
                    "Second-chance mechanism helps retain hot pages during scans".into(),
                    "The clock hand sweep can take O(n) in the worst case if all pages \
                     have their reference bit set, causing a temporary latency spike".into(),
                    "Cannot distinguish between a page accessed once and a page accessed \
                     many times — both just have ref_bit=true".into(),
                ],
                examples: vec![
                    "PostgreSQL uses clock-sweep for its shared buffer pool — the \
                     bgwriter process advances the clock hand proactively".into(),
                    "OS page replacement algorithms — Linux uses a two-list variant \
                     (active/inactive) inspired by CLOCK concepts".into(),
                    "Apache Derby uses a CLOCK-based buffer manager".into(),
                ],
                motivation: "Maintaining a strict LRU list requires updating a data structure \
                             on every single page access. In a high-concurrency database with \
                             thousands of queries running simultaneously, this creates severe \
                             lock contention on the LRU list. CLOCK solves this by replacing \
                             the list update with a single bit-set operation, which can be done \
                             atomically without locking.\n\n\
                             Without CLOCK (or a similar approximation), the buffer manager \
                             becomes a bottleneck under concurrent workloads. The slight loss \
                             in eviction quality compared to true LRU is a worthwhile trade \
                             for dramatically better throughput."
                    .into(),
                parameter_guide: HashMap::from([
                    ("size".into(), "The number of slots in the circular buffer. More slots \
                                     mean more pages can be cached, improving hit rates. However, \
                                     each slot consumes memory for the page data. Start with 1024 \
                                     and observe the hit rate and clock_hand_sweeps metrics. If \
                                     sweeps are frequent, the pool is too small for the workload. \
                                     Aim for a hit rate above 90% for OLTP workloads.".into()),
                    ("page_size".into(), "Size of each cached page in bytes. Should match the \
                                          storage layer's page size. The default of 8192 bytes \
                                          (8 KB) matches PostgreSQL. Larger pages (16 KB, like \
                                          MySQL InnoDB) hold more rows per page but waste space \
                                          when accessing individual rows. Smaller pages (4 KB) \
                                          are better for point lookups on small records.".into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "lru-buffer-pool".into(),
                        comparison: "LRU maintains exact access order, giving more precise \
                                     eviction decisions. Choose LRU when concurrency is low \
                                     and eviction quality matters (e.g., single-threaded \
                                     embedded databases). Choose CLOCK when the system has \
                                     many concurrent readers and the overhead of list \
                                     reordering on every access is unacceptable. In practice, \
                                     CLOCK achieves hit rates within 1-2% of true LRU.".into(),
                    },
                ],
                suggested_questions: vec![
                    "How many times does the clock hand need to sweep before evicting a \
                     page that was just accessed?".into(),
                    "What is the worst-case scenario for CLOCK eviction latency, and how \
                     can PostgreSQL's bgwriter mitigate it?".into(),
                    "How would you modify CLOCK to give more chances to frequently accessed \
                     pages (hint: look up CLOCK-Pro or GCLOCK)?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Book,
                title: "Database Internals by Alex Petrov — Chapter 5: Buffer Management".into(),
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
                description: "Size of each page in bytes".into(),
                default_value: ParameterValue::Integer(8192),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(512.0).with_max(65536.0),
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
                id: "clock_hand_sweeps".into(),
                name: "Clock Sweeps".into(),
                metric_type: MetricType::Counter,
                unit: "rotations".into(),
                description: "Full rotations of the clock hand".into(),
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
        if let Some(&slot) = self.page_map.get(&page_id) {
            // Hit — set reference bit.
            if let Some(entry) = &mut self.pages[slot] {
                entry.reference_bit = true;
            }
            self.hits += 1;
            true
        } else {
            // Miss — find a slot using clock sweep.
            if self.page_map.len() >= self.capacity {
                self.evict_one();
            }
            // Find an empty slot (there must be one after eviction).
            let slot = self.find_empty_slot();
            self.pages[slot] = Some(ClockEntry {
                page_id,
                reference_bit: true,
            });
            self.page_map.insert(page_id, slot);
            self.misses += 1;
            false
        }
    }

    /// Clock sweep: advance hand, clearing reference bits until we find one to evict.
    fn evict_one(&mut self) {
        let n = self.pages.len();
        loop {
            if let Some(entry) = &mut self.pages[self.clock_hand] {
                if entry.reference_bit {
                    // Second chance: clear bit and move on.
                    entry.reference_bit = false;
                } else {
                    // Evict this page.
                    let victim_id = entry.page_id;
                    self.page_map.remove(&victim_id);
                    self.pages[self.clock_hand] = None;
                    self.evictions += 1;
                    self.advance_hand(n);
                    return;
                }
            }
            self.advance_hand(n);
        }
    }

    fn advance_hand(&mut self, n: usize) {
        self.clock_hand = (self.clock_hand + 1) % n;
        if self.clock_hand == 0 {
            self.clock_hand_sweeps += 1;
        }
    }

    fn find_empty_slot(&self) -> usize {
        self.pages
            .iter()
            .position(|p| p.is_none())
            .unwrap_or(0)
    }

    pub fn current_size(&self) -> usize {
        self.page_map.len()
    }

    pub fn hit_rate_pct(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    pub fn contains(&self, page_id: usize) -> bool {
        self.page_map.contains_key(&page_id)
    }
}

impl Default for ClockBufferBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for ClockBufferBlock {
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
                return Err(BlockError::InvalidParameter("size must be at least 1".into()));
            }
            self.pages = vec![None; self.capacity];
            self.page_map.clear();
            self.clock_hand = 0;
        }
        if let Some(val) = params.get("page_size") {
            self.page_size = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("page_size must be an integer".into()))?
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

        context.metrics.record("hit_rate_pct", self.hit_rate_pct());
        context.metrics.record("evictions", self.evictions as f64);
        context.metrics.record("clock_hand_sweeps", self.clock_hand_sweeps as f64);
        context.metrics.record("current_size", self.current_size() as f64);

        let mut outputs = HashMap::new();
        outputs.insert("pages".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("cache_hits".into(), self.hits as f64);
        metrics_summary.insert("cache_misses".into(), self.misses as f64);
        metrics_summary.insert("hit_rate_pct".into(), self.hit_rate_pct());
        metrics_summary.insert("evictions".into(), self.evictions as f64);
        metrics_summary.insert("clock_hand_sweeps".into(), self.clock_hand_sweeps as f64);
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
                PortValue::None => ValidationResult::ok().with_warning("No page requests provided"),
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
        let mut pool = ClockBufferBlock::new();
        pool.capacity = 4;
        pool.pages = vec![None; 4];

        assert!(!pool.get_page(1));
        assert_eq!(pool.misses, 1);

        assert!(pool.get_page(1));
        assert_eq!(pool.hits, 1);
    }

    #[test]
    fn test_eviction_second_chance() {
        let mut pool = ClockBufferBlock::new();
        pool.capacity = 3;
        pool.pages = vec![None; 3];

        // Fill pool: pages 1, 2, 3
        pool.get_page(1); // miss
        pool.get_page(2); // miss
        pool.get_page(3); // miss
        assert_eq!(pool.current_size(), 3);

        // All pages have reference bits set. Access page 1 again (still set).
        pool.get_page(1); // hit

        // Insert page 4 — clock sweep clears ref bits until it finds one to evict.
        // Hand starts at 0: page 1 has ref=true → clear, advance
        // Hand at 1: page 2 has ref=true → clear, advance
        // Hand at 2: page 3 has ref=true → clear, advance
        // Hand wraps to 0: page 1 has ref=false (was re-set by hit then cleared by sweep)
        // Actually after the hit on page 1, ref bit was set. Then sweep:
        //   slot 0 (page 1): ref=true → clear → advance
        //   slot 1 (page 2): ref=true → clear → advance
        //   slot 2 (page 3): ref=true → clear → advance (sweep++)
        //   slot 0 (page 1): ref=false → EVICT
        pool.get_page(4); // miss, evicts page 1
        assert_eq!(pool.evictions, 1);
        assert!(!pool.contains(1), "Page 1 should have been evicted");
        assert!(pool.contains(4));
    }

    #[test]
    fn test_hit_rate() {
        let mut pool = ClockBufferBlock::new();
        pool.capacity = 100;
        pool.pages = vec![None; 100];

        for i in 0..10 {
            pool.get_page(i);
        }
        for i in 0..10 {
            pool.get_page(i);
        }

        assert_eq!(pool.hits, 10);
        assert_eq!(pool.misses, 10);
        assert!((pool.hit_rate_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_metadata() {
        let pool = ClockBufferBlock::new();
        assert_eq!(pool.metadata().id, "clock-buffer-pool");
        assert_eq!(pool.metadata().category, BlockCategory::Buffer);
        assert_eq!(pool.inputs().len(), 1);
        assert_eq!(pool.outputs().len(), 1);
    }
}
