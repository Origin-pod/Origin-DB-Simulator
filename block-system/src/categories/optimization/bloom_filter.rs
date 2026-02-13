//! Bloom Filter Block
//!
//! A probabilistic data structure used to test set membership. Returns either
//! "possibly in set" or "definitely not in set". Used by LSM-tree databases
//! (Cassandra, RocksDB, LevelDB) to avoid unnecessary disk reads.
//!
//! ## How it works
//!
//! A bit array of `m` bits with `k` hash functions. To add an element, hash it
//! with each function and set those bits. To test membership, check if all k
//! bits are set — if any is 0, the element is definitely absent.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `checks` | Counter | Total membership queries |
//! | `true_positives` | Counter | Queries correctly identified as present |
//! | `false_positives` | Counter | Queries incorrectly reported as present |
//! | `true_negatives` | Counter | Queries correctly identified as absent |
//! | `false_positive_rate` | Gauge | false_positives / (false_positives + true_negatives) |
//! | `bits_used` | Gauge | Number of set bits in the filter |

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
// BloomFilterBlock
// ---------------------------------------------------------------------------

pub struct BloomFilterBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    num_bits: usize,
    num_hash_fns: usize,

    // Internal state
    bits: Vec<bool>,
    /// Tracks actually inserted keys for ground-truth comparison
    inserted_keys: std::collections::HashSet<u64>,

    // Stats
    checks: usize,
    true_positives: usize,
    false_positives: usize,
    true_negatives: usize,
}

impl BloomFilterBlock {
    pub fn new() -> Self {
        let num_bits = 10_000;
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            num_bits,
            num_hash_fns: 7,
            bits: vec![false; num_bits],
            inserted_keys: std::collections::HashSet::new(),
            checks: 0,
            true_positives: 0,
            false_positives: 0,
            true_negatives: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "bloom-filter".into(),
            name: "Bloom Filter".into(),
            category: BlockCategory::Optimization,
            description: "Probabilistic filter that prevents unnecessary disk reads".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A Bloom filter is a space-efficient probabilistic data structure that tests whether \
                           an element is a member of a set. It can tell you with certainty that an element is \
                           NOT in the set, but can only say that an element is PROBABLY in the set. False positives \
                           are possible, but false negatives are not.\n\n\
                           In database systems, Bloom filters are used as a first-pass check before expensive \
                           disk I/O. When a query looks up a key, the Bloom filter is checked first. If the \
                           filter says 'no', the database skips the disk read entirely — a huge performance win. \
                           If the filter says 'maybe yes', the database proceeds with the actual read, which \
                           might turn out to be a false positive (the key is not actually there).\n\n\
                           Think of a Bloom filter like a bouncer at a club with a guest list. The bouncer can \
                           instantly tell you 'you are definitely NOT on the list' (true negative). But sometimes \
                           the bouncer says 'you might be on the list, go check inside' — and when you get \
                           inside, it turns out you were not actually on the list (false positive). The bouncer \
                           never wrongly turns away someone who IS on the list (no false negatives)."
                    .into(),
                algorithm: "INSERT(key):\n  \
                           For i in 0..k (each hash function):\n    \
                             index = hash_i(key) % num_bits\n    \
                             bits[index] = 1\n\n\
                           QUERY(key):\n  \
                           For i in 0..k:\n    \
                             index = hash_i(key) % num_bits\n    \
                             If bits[index] == 0:\n      \
                               Return DEFINITELY_NOT_PRESENT\n  \
                           Return POSSIBLY_PRESENT\n\n\
                           FALSE_POSITIVE_RATE:\n  \
                           Theoretical: (1 - e^(-kn/m))^k\n  \
                           Where n=items inserted, m=total bits, k=hash functions\n  \
                           Optimal k = (m/n) * ln(2) ≈ 0.693 * (m/n)"
                    .into(),
                complexity: Complexity {
                    time: "O(k) per insert and query, where k is the number of hash functions"
                        .into(),
                    space: "O(m) bits, where m = -n·ln(p) / (ln2)² for n items at false-positive rate p"
                        .into(),
                },
                use_cases: vec![
                    "LSM-tree databases skip SSTables that don't contain the key".into(),
                    "Cassandra checks Bloom filters before reading from disk".into(),
                    "RocksDB uses per-SSTable Bloom filters to reduce read amplification".into(),
                    "Network routers use Bloom filters for packet filtering and deduplication".into(),
                    "Distributed caches use Bloom filters to avoid network round-trips for missing keys".into(),
                ],
                tradeoffs: vec![
                    "More bits = lower false positive rate but more memory".into(),
                    "More hash functions = lower false positives up to a point, then diminishing returns".into(),
                    "Cannot delete elements (use Counting Bloom filter for that)".into(),
                    "Optimal k = (m/n) · ln(2) ≈ 0.693 · (m/n)".into(),
                    "Filter must be rebuilt if the underlying data changes (no incremental delete)".into(),
                    "At high fill ratios (>50% bits set), false positive rate degrades rapidly".into(),
                ],
                examples: vec![
                    "Cassandra — per-SSTable Bloom filter configured via bloom_filter_fp_chance (default 0.01)".into(),
                    "RocksDB — configurable bits_per_key (default 10), full and partitioned filters".into(),
                    "LevelDB — uses Bloom filters to skip levels during point lookups".into(),
                    "Chrome — used Bloom filters for safe browsing URL checking before switching to prefix sets".into(),
                    "HBase — per-StoreFile Bloom filter to skip unnecessary block reads".into(),
                ],
                motivation: "Without a Bloom filter, a point lookup in an LSM-tree database might need to check \
                             every SSTable on disk to determine if a key exists. With thousands of SSTables, \
                             this means thousands of disk reads for a single query — each taking milliseconds. \
                             A Bloom filter for each SSTable allows the database to skip the vast majority of \
                             these disk reads with a single in-memory bit check.\n\n\
                             The impact is dramatic: RocksDB with 10 bits per key achieves roughly a 1% false \
                             positive rate, meaning 99% of unnecessary disk reads are eliminated. For a read-heavy \
                             workload with many non-existent keys, this can improve query latency by 10-100x."
                    .into(),
                parameter_guide: HashMap::from([
                    ("num_bits".into(),
                     "The total number of bits in the Bloom filter's bit array. More bits means lower false \
                      positive rates but higher memory usage. The relationship is: for n items with target \
                      false positive rate p, you need m = -n*ln(p)/(ln2)^2 bits. For example, 1000 items \
                      at 1% FP rate needs ~9585 bits (~1.2 KB). At 10 bits per item, expect ~1% FP rate. \
                      At 5 bits per item, expect ~5% FP rate. Recommended: 10,000-100,000 for typical \
                      SSTable sizes, scaling with the number of keys per SSTable."
                        .into()),
                    ("num_hash_functions".into(),
                     "The number of independent hash functions used to set and check bits. The optimal \
                      number is k = (m/n) * ln(2), where m is bits and n is items. Too few hash functions \
                      means higher false positive rates. Too many means the bit array fills up faster, also \
                      increasing false positives. For 10 bits per key, the optimal k is about 7. For 5 bits \
                      per key, optimal k is about 3-4. Recommended: 7 for the default 10,000 bits with \
                      ~1000 items; adjust based on your bits-per-key ratio."
                        .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "statistics-collector".into(),
                        comparison: "Bloom filters and statistics collectors serve different optimization \
                                     purposes. Bloom filters answer 'is this specific key present?' with a \
                                     quick probabilistic check, avoiding unnecessary I/O for point lookups. \
                                     Statistics collectors gather distribution data (cardinality, histograms) \
                                     to help the query planner choose efficient execution strategies. Use Bloom \
                                     filters for point lookup optimization in LSM-trees; use statistics for \
                                     query planning and join order selection."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "How do I calculate the optimal number of bits and hash functions for a target false positive rate?".into(),
                    "What is a Counting Bloom filter, and how does it support deletions?".into(),
                    "Why do LSM-tree databases need Bloom filters but B-tree databases generally do not?".into(),
                ],
            },
            references: vec![
                Reference {
                    ref_type: ReferenceType::Paper,
                    title: "Space/Time Trade-offs in Hash Coding with Allowable Errors — Burton Howard Bloom (1970)".into(),
                    url: None,
                    citation: Some("Bloom, B. H. (1970). Communications of the ACM, 13(7), 422–426.".into()),
                },
                Reference {
                    ref_type: ReferenceType::Book,
                    title: "Database Internals by Alex Petrov — Chapter 8: Log-Structured Storage".into(),
                    url: None,
                    citation: Some("Petrov, A. (2019). Database Internals. O'Reilly.".into()),
                },
            ],
            icon: "sparkles".into(),
            color: "#8B5CF6".into(),
        }
    }

    fn build_inputs() -> Vec<Port> {
        vec![Port {
            id: "requests".into(),
            name: "Lookup Requests".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Input,
            required: true,
            multiple: false,
            description: "Records to check against the filter. Uses `_key` field for membership test.".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "filtered".into(),
            name: "Filtered Results".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records enriched with `_bloom_hit` (bool) — true if possibly present".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "num_bits".into(),
                name: "Filter Size".into(),
                param_type: ParameterType::Number,
                description: "Number of bits in the filter (more bits = fewer false positives)".into(),
                default_value: ParameterValue::Integer(10000),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(64.0).with_max(1_000_000.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1000.0)
                        .with_unit("bits".into()),
                ),
            },
            Parameter {
                id: "num_hash_functions".into(),
                name: "Hash Functions".into(),
                param_type: ParameterType::Number,
                description: "Number of hash functions (optimal ≈ 0.693 × bits/items)".into(),
                default_value: ParameterValue::Integer(7),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(1.0).with_max(20.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider).with_step(1.0),
                ),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "checks".into(),
                name: "Total Checks".into(),
                metric_type: MetricType::Counter,
                unit: "queries".into(),
                description: "Total membership queries against the filter".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "true_positives".into(),
                name: "True Positives".into(),
                metric_type: MetricType::Counter,
                unit: "queries".into(),
                description: "Queries correctly identified as present".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "false_positives".into(),
                name: "False Positives".into(),
                metric_type: MetricType::Counter,
                unit: "queries".into(),
                description: "Queries incorrectly reported as present (wasted reads)".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "true_negatives".into(),
                name: "True Negatives".into(),
                metric_type: MetricType::Counter,
                unit: "queries".into(),
                description: "Queries correctly filtered out (saved reads)".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "false_positive_rate".into(),
                name: "False Positive Rate".into(),
                metric_type: MetricType::Gauge,
                unit: "%".into(),
                description: "Percentage of negative queries that were false positives".into(),
                aggregations: vec![AggregationType::Avg],
            },
            MetricDefinition {
                id: "bits_used".into(),
                name: "Bits Used".into(),
                metric_type: MetricType::Gauge,
                unit: "bits".into(),
                description: "Number of set bits in the filter".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Simple hash function using FNV-1a-like mixing.
    fn hash(&self, key: u64, seed: usize) -> usize {
        let mut h = key.wrapping_add(seed as u64).wrapping_mul(6364136223846793005);
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        (h as usize) % self.num_bits
    }

    /// Insert a key into the filter.
    pub fn insert(&mut self, key: u64) {
        self.inserted_keys.insert(key);
        for i in 0..self.num_hash_fns {
            let idx = self.hash(key, i);
            self.bits[idx] = true;
        }
    }

    /// Check if a key might be in the set.
    pub fn might_contain(&mut self, key: u64) -> bool {
        self.checks += 1;
        let bloom_says_yes = (0..self.num_hash_fns).all(|i| {
            let idx = self.hash(key, i);
            self.bits[idx]
        });

        let actually_present = self.inserted_keys.contains(&key);

        if bloom_says_yes {
            if actually_present {
                self.true_positives += 1;
            } else {
                self.false_positives += 1;
            }
        } else {
            // Bloom filter says no — guaranteed correct.
            self.true_negatives += 1;
        }

        bloom_says_yes
    }

    pub fn false_positive_rate(&self) -> f64 {
        let negatives = self.false_positives + self.true_negatives;
        if negatives == 0 {
            return 0.0;
        }
        (self.false_positives as f64 / negatives as f64) * 100.0
    }

    pub fn bits_used(&self) -> usize {
        self.bits.iter().filter(|&&b| b).count()
    }
}

impl Default for BloomFilterBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for BloomFilterBlock {
    fn metadata(&self) -> &BlockMetadata { &self.metadata }
    fn inputs(&self) -> &[Port] { &self.input_ports }
    fn outputs(&self) -> &[Port] { &self.output_ports }
    fn parameters(&self) -> &[Parameter] { &self.params }
    fn requires(&self) -> &[Constraint] { &[] }
    fn guarantees(&self) -> &[Guarantee] { &[] }
    fn metrics(&self) -> &[MetricDefinition] { &self.metric_defs }

    async fn initialize(
        &mut self,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        if let Some(val) = params.get("num_bits") {
            self.num_bits = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("num_bits must be an integer".into()))?
                as usize;
            self.bits = vec![false; self.num_bits];
        }
        if let Some(val) = params.get("num_hash_functions") {
            self.num_hash_fns = val
                .as_integer()
                .ok_or_else(|| BlockError::InvalidParameter("num_hash_functions must be an integer".into()))?
                as usize;
        }
        Ok(())
    }

    async fn execute(
        &mut self,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, BlockError> {
        let input = context.inputs.get("requests").cloned().unwrap_or(PortValue::None);

        let records = match input {
            PortValue::Stream(r) => r,
            PortValue::Batch(r) => r,
            PortValue::Single(r) => vec![r],
            PortValue::None => Vec::new(),
            _ => return Err(BlockError::InvalidInput("Expected DataStream".into())),
        };

        let mut output_records = Vec::with_capacity(records.len());

        // Phase 1: Insert operations (writes) — records with _op_type == "INSERT"
        // Phase 2: Query operations (reads) — everything else
        let (inserts, queries): (Vec<_>, Vec<_>) = records.into_iter().partition(|r| {
            r.get::<String>("_op_type")
                .ok()
                .flatten()
                .map(|s| s == "INSERT" || s == "insert")
                .unwrap_or(false)
        });

        for record in &inserts {
            let key = record.get::<u64>("_key").ok().flatten().unwrap_or(0);
            self.insert(key);
        }

        for record in queries {
            let key = record.get::<u64>("_key").ok().flatten().unwrap_or(0);
            let hit = self.might_contain(key);

            if hit {
                context.metrics.increment("bloom_hit");
            } else {
                context.metrics.increment("bloom_miss");
            }

            let mut out = record;
            let _ = out.insert("_bloom_hit".into(), hit);
            output_records.push(out);
        }

        // Also pass through inserts with bloom_hit = true
        for record in inserts {
            let mut out = record;
            let _ = out.insert("_bloom_hit".into(), true);
            output_records.push(out);
        }

        context.metrics.record("false_positive_rate", self.false_positive_rate());
        context.metrics.record("bits_used", self.bits_used() as f64);

        let mut outputs = HashMap::new();
        outputs.insert("filtered".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("checks".into(), self.checks as f64);
        metrics_summary.insert("true_positives".into(), self.true_positives as f64);
        metrics_summary.insert("false_positives".into(), self.false_positives as f64);
        metrics_summary.insert("true_negatives".into(), self.true_negatives as f64);
        metrics_summary.insert("false_positive_rate".into(), self.false_positive_rate());
        metrics_summary.insert("bits_used".into(), self.bits_used() as f64);

        Ok(ExecutionResult {
            outputs,
            metrics: metrics_summary,
            errors: vec![],
        })
    }

    fn validate(&self, inputs: &HashMap<String, PortValue>) -> ValidationResult {
        if let Some(input) = inputs.get("requests") {
            match input {
                PortValue::Stream(_) | PortValue::Batch(_) | PortValue::Single(_) => ValidationResult::ok(),
                PortValue::None => ValidationResult::ok().with_warning("No requests provided"),
                _ => ValidationResult::error("requests port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("requests input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("num_bits".into(), self.num_bits);
        let _ = state.insert("num_hash_functions".into(), self.num_hash_fns);
        let _ = state.insert("checks".into(), self.checks);
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(n)) = state.get::<usize>("num_bits") { self.num_bits = n; }
        if let Ok(Some(k)) = state.get::<usize>("num_hash_functions") { self.num_hash_fns = k; }
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
    fn test_no_false_negatives() {
        let mut bf = BloomFilterBlock::new();
        bf.num_bits = 10_000;
        bf.num_hash_fns = 7;
        bf.bits = vec![false; 10_000];

        // Insert keys 0..100
        for i in 0..100u64 {
            bf.insert(i);
        }

        // All inserted keys must be found (no false negatives).
        for i in 0..100u64 {
            assert!(bf.might_contain(i), "Key {} should be found", i);
        }
    }

    #[test]
    fn test_false_positive_rate_reasonable() {
        let mut bf = BloomFilterBlock::new();
        bf.num_bits = 100_000;
        bf.num_hash_fns = 7;
        bf.bits = vec![false; 100_000];

        for i in 0..1000u64 {
            bf.insert(i);
        }

        // Check 1000 keys that were NOT inserted.
        let mut fp = 0;
        for i in 10_000..11_000u64 {
            if bf.might_contain(i) {
                fp += 1;
            }
        }

        // With 100k bits and 1000 items, FP rate should be very low (< 5%).
        assert!(fp < 50, "False positive rate too high: {}/1000", fp);
    }

    #[test]
    fn test_metadata() {
        let bf = BloomFilterBlock::new();
        assert_eq!(bf.metadata().id, "bloom-filter");
        assert_eq!(bf.metadata().category, BlockCategory::Optimization);
    }
}
