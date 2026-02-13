//! LSM Tree Storage Block
//!
//! A Log-Structured Merge-Tree that buffers writes in a sorted in-memory
//! **memtable** and flushes to on-disk **SSTables** organized into levels.
//! Compaction merges SSTables to reclaim space and reduce read amplification.
//!
//! ## How it works
//!
//! 1. **Write path**: Records are appended to the active memtable (a sorted
//!    structure). When the memtable reaches `memtable_size`, it is frozen and
//!    flushed as a new SSTable in Level 0.
//! 2. **Compaction**: When Level 0 accumulates too many SSTables, they are
//!    merged (size-tiered) into Level 1. The same process cascades upward.
//! 3. **Read path**: Point lookups check the memtable first, then Level 0
//!    SSTables (newest first), then higher levels. A **Bloom filter** on each
//!    SSTable lets us skip tables that definitely don't contain the key.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `memtable_size` | Gauge | Entries in the active memtable |
//! | `total_sstables` | Gauge | Total SSTables across all levels |
//! | `level_count` | Gauge | Deepest non-empty level |
//! | `compactions` | Counter | Compaction operations performed |
//! | `flushes` | Counter | Memtable flushes to Level 0 |
//! | `bloom_true_negatives` | Counter | Reads skipped by bloom filter |
//! | `bloom_false_positives` | Counter | Bloom filter said yes but key absent |
//! | `write_amplification` | Gauge | Total bytes written / user bytes |
//! | `read_amplification` | Gauge | SSTables checked per point lookup |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};

use crate::core::block::{
    Alternative, Block, BlockCategory, BlockDocumentation, BlockError, BlockMetadata, BlockState,
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
// Internal SSTable model
// ---------------------------------------------------------------------------

/// A simple Bloom filter for probabilistic key membership testing.
#[derive(Debug, Clone)]
struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
}

impl BloomFilter {
    fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let bits_count = if expected_items == 0 {
            64
        } else {
            let n = expected_items as f64;
            let m = -(n * false_positive_rate.ln()) / (2.0_f64.ln().powi(2));
            (m as usize).max(64)
        };
        let num_hashes = if expected_items == 0 {
            3
        } else {
            let k = (bits_count as f64 / expected_items as f64) * 2.0_f64.ln();
            (k as usize).max(1).min(10)
        };
        Self {
            bits: vec![false; bits_count],
            num_hashes,
        }
    }

    fn insert(&mut self, key: &str) {
        for i in 0..self.num_hashes {
            let idx = self.hash(key, i) % self.bits.len();
            self.bits[idx] = true;
        }
    }

    fn might_contain(&self, key: &str) -> bool {
        for i in 0..self.num_hashes {
            let idx = self.hash(key, i) % self.bits.len();
            if !self.bits[idx] {
                return false;
            }
        }
        true
    }

    /// Simple hash: FNV-1a variant with seed.
    fn hash(&self, key: &str, seed: usize) -> usize {
        let mut h: u64 = 14695981039346656037u64.wrapping_add(seed as u64 * 2654435761);
        for b in key.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        h as usize
    }
}

/// A sorted string table — an immutable, sorted collection of key-value pairs.
#[derive(Debug, Clone)]
struct SSTable {
    /// Entries sorted by key.
    entries: Vec<(String, JsonValue)>,
    /// Bloom filter for fast negative lookups.
    bloom: BloomFilter,
    /// Approximate byte size of this SSTable.
    size_bytes: usize,
}

impl SSTable {
    fn from_entries(mut entries: Vec<(String, JsonValue)>) -> Self {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut bloom = BloomFilter::new(entries.len(), 0.01);
        let mut size_bytes = 0;
        for (k, v) in &entries {
            bloom.insert(k);
            size_bytes += k.len() + v.to_string().len() + 16; // overhead
        }
        Self {
            entries,
            bloom,
            size_bytes,
        }
    }

    fn lookup(&self, key: &str) -> Option<&JsonValue> {
        // Binary search in sorted entries.
        self.entries
            .binary_search_by(|(k, _)| k.as_str().cmp(key))
            .ok()
            .map(|idx| &self.entries[idx].1)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// LSMTreeBlock
// ---------------------------------------------------------------------------

pub struct LSMTreeBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    memtable_size: usize,
    level0_compaction_trigger: usize,
    size_ratio: usize,
    bloom_fp_rate: f64,

    // Internal state
    /// Active memtable — sorted by key via BTreeMap.
    memtable: BTreeMap<String, JsonValue>,
    /// Levels of SSTables. Level 0 has the newest, unsorted-among-tables data.
    levels: Vec<Vec<SSTable>>,

    // Counters
    flush_count: usize,
    compaction_count: usize,
    bloom_true_negatives: usize,
    bloom_false_positives: usize,
    total_bytes_written: usize,
    user_bytes_written: usize,
}

impl LSMTreeBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            memtable_size: 1000,
            level0_compaction_trigger: 4,
            size_ratio: 10,
            bloom_fp_rate: 0.01,
            memtable: BTreeMap::new(),
            levels: vec![Vec::new(); 4], // L0..L3
            flush_count: 0,
            compaction_count: 0,
            bloom_true_negatives: 0,
            bloom_false_positives: 0,
            total_bytes_written: 0,
            user_bytes_written: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "lsm-tree-storage".into(),
            name: "LSM Tree".into(),
            category: BlockCategory::Storage,
            description: "Log-Structured Merge-Tree with memtable, SSTables, and compaction".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "An LSM Tree (Log-Structured Merge-Tree) optimizes write throughput by \
                           buffering all writes in a fast, in-memory sorted structure called a \
                           memtable. When the memtable reaches a configured size threshold, it is \
                           frozen and flushed to disk as an immutable Sorted String Table (SSTable). \
                           Background compaction merges SSTables across levels to bound read \
                           amplification and reclaim space from obsolete entries.\n\n\
                           In a database system, the LSM tree sits at the storage engine layer and \
                           is the dominant architecture for write-optimized key-value stores. Unlike \
                           B-trees that update data in-place (requiring random I/O), LSM trees \
                           convert random writes into sequential writes by batching them in memory \
                           and writing sorted runs. This makes them ideal for modern SSDs and \
                           spinning disks alike.\n\n\
                           Think of an LSM tree like a mail sorting system: incoming letters \
                           (writes) pile up in your inbox (memtable). Periodically you sort the \
                           inbox and file it into a cabinet drawer (Level 0 SSTable). When a drawer \
                           gets too full, you merge it with the next larger filing cabinet \
                           (compaction to Level 1). Finding a specific letter means checking the \
                           inbox first, then the most recent drawer, working backward."
                    .into(),
                algorithm: "WRITE (put key, value):\n  \
                           1. Insert (key, value) into the memtable (BTreeMap, O(log n))\n  \
                           2. Track user_bytes_written for amplification metrics\n  \
                           3. If memtable.len() >= memtable_size:\n    \
                              a. Freeze the memtable\n    \
                              b. Sort entries and write as a new SSTable at Level 0\n    \
                              c. Create a Bloom filter for the new SSTable\n    \
                              d. Increment flush_count\n    \
                              e. If Level 0 has >= level0_compaction_trigger SSTables:\n      \
                                 Trigger compaction of Level 0 into Level 1\n\n\
                           COMPACTION (level L):\n  \
                           1. Collect all entries from all SSTables at level L\n  \
                           2. Merge with all entries at level L+1\n  \
                           3. Sort by key, deduplicate (keep newest value)\n  \
                           4. Write as a new SSTable at level L+1\n  \
                           5. Track total_bytes_written for amplification\n  \
                           6. Check if L+1 also needs compaction (cascading)\n\n\
                           READ (get key):\n  \
                           1. Check memtable — return if found (newest data)\n  \
                           2. For each level, newest SSTable first:\n    \
                              a. Check Bloom filter — skip if definitely absent\n    \
                              b. Binary search SSTable entries\n    \
                              c. Return if found\n  \
                           3. Return None if not found in any level"
                    .into(),
                complexity: Complexity {
                    time: "Write O(log n) amortized, Point read O(L * log n) where L = levels"
                        .into(),
                    space: "O(n) with write amplification factor ~size_ratio per level".into(),
                },
                use_cases: vec![
                    "Write-heavy workloads (logging, time-series, event ingestion)".into(),
                    "Key-value stores (RocksDB, LevelDB, Cassandra)".into(),
                    "Workloads where reads can tolerate some amplification".into(),
                    "Append-heavy ingestion pipelines where data arrives faster than a B-tree can index it".into(),
                    "Time-series databases where recent data is queried most often".into(),
                ],
                tradeoffs: vec![
                    "Excellent write throughput but reads check multiple levels".into(),
                    "Write amplification from compaction (each record rewritten ~size_ratio times per level)".into(),
                    "Space amplification before compaction catches up".into(),
                    "Bloom filters reduce read cost but use memory".into(),
                    "Compaction can cause latency spikes if not properly scheduled or rate-limited".into(),
                    "Range scans are more expensive than B-trees since data is spread across levels".into(),
                ],
                examples: vec![
                    "RocksDB (Meta) — the most widely used LSM engine, used in MySQL (MyRocks), CockroachDB, TiKV".into(),
                    "LevelDB (Google) — the original leveled compaction implementation, inspiration for RocksDB".into(),
                    "Apache Cassandra SSTables — distributed LSM storage with size-tiered compaction".into(),
                    "ScyllaDB — high-performance Cassandra-compatible LSM engine written in C++".into(),
                ],
                motivation: "Traditional B-tree storage engines must perform random I/O for every \
                             write because they update data in-place on disk. When write throughput \
                             is the bottleneck — such as ingesting millions of events per second, \
                             logging, or time-series data — this random I/O becomes the limiting \
                             factor.\n\n\
                             The LSM tree solves this by converting random writes into sequential \
                             writes. Instead of seeking to the right page on disk for each insert, \
                             it batches writes in memory and periodically flushes them as a large, \
                             sorted, sequential write. This can achieve 10-100x higher write \
                             throughput compared to B-tree engines, at the cost of more expensive \
                             reads and background compaction work."
                    .into(),
                parameter_guide: HashMap::from([
                    ("memtable_size".into(),
                     "Controls how many entries accumulate in memory before being flushed to \
                      disk as an SSTable. Larger memtable (e.g., 10000-100000) means fewer \
                      flushes, larger SSTables, and better write throughput — but uses more \
                      RAM and risks more data loss on crash (unless a WAL is used). Smaller \
                      memtable (e.g., 10-100) flushes frequently, creating many small SSTables \
                      that increase read amplification and trigger more compactions. \
                      Recommended: 1000-10000 for general workloads. Default is 1000."
                         .into()),
                    ("level0_compaction_trigger".into(),
                     "Number of SSTables at Level 0 before compaction merges them into Level 1. \
                      Higher values (e.g., 8-20) delay compaction, which improves write throughput \
                      but makes reads slower because more L0 SSTables must be checked. Lower \
                      values (e.g., 2-4) compact sooner, keeping reads fast but increasing write \
                      amplification. Recommended: 4-8 for balanced workloads, 2-3 for read-heavy, \
                      10+ for write-heavy. Default is 4."
                         .into()),
                    ("size_ratio".into(),
                     "The size multiplier between adjacent levels. With size_ratio=10, Level 1 \
                      can hold 10x more data than Level 0, Level 2 holds 100x, etc. Higher \
                      values (e.g., 10-20) mean fewer levels and fewer compactions, but each \
                      compaction moves more data (higher write amplification per compaction). \
                      Lower values (e.g., 2-4) create more levels, meaning more SSTables to \
                      check during reads. Recommended: 10 for most workloads (matches LevelDB/RocksDB \
                      defaults). Range: 2-20."
                         .into()),
                    ("key_column".into(),
                     "The column name used as the key for the LSM tree. Each record must have \
                      this column. The key determines how records are sorted within SSTables \
                      and how duplicates are resolved (latest value wins). Choose the column \
                      that you will most frequently look up by. Default is 'id'."
                         .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "heap-file-storage".into(),
                        comparison: "Heap files append records to unordered pages with no memtable \
                                     or compaction overhead. They are simpler and have no write \
                                     amplification, but offer no key-based organization. Choose heap \
                                     for simple row storage with separate B-tree indexes. Choose LSM \
                                     when write throughput is the primary concern and you need built-in \
                                     key-value semantics."
                            .into(),
                    },
                    Alternative {
                        block_type: "clustered-storage".into(),
                        comparison: "Clustered storage sorts records by a key on disk for fast range \
                                     scans but suffers from page splits on random inserts. Choose \
                                     clustered when range scan performance on the sort key matters \
                                     most. Choose LSM when write throughput is more important than \
                                     range scan speed."
                            .into(),
                    },
                    Alternative {
                        block_type: "columnar-storage".into(),
                        comparison: "Columnar storage organizes data by column for analytical queries. \
                                     It excels at scanning few columns across many rows but is poor \
                                     at point lookups and single-row writes. Choose columnar for \
                                     OLAP/warehouse workloads. Choose LSM for OLTP key-value workloads \
                                     with high write rates."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "How does write amplification change as the size_ratio increases from 2 to 20?".into(),
                    "Why do Bloom filters help read performance, and what happens if the false positive rate is too high?".into(),
                    "What is the difference between size-tiered and leveled compaction strategies?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Paper,
                title: "The Log-Structured Merge-Tree (LSM-Tree)".into(),
                url: None,
                citation: Some(
                    "O'Neil, P. et al. (1996). Acta Informatica, 33(4), 351–385.".into(),
                ),
            }],
            icon: "layers".into(),
            color: "#8B5CF6".into(),
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
            description: "Stream of records to store (must have a key column)".into(),
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
            description: "Records after storage with level metadata".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "memtable_size".into(),
                name: "Memtable Size".into(),
                param_type: ParameterType::Number,
                description: "Max entries in the memtable before flushing to L0".into(),
                default_value: ParameterValue::Integer(1000),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(10.0).with_max(100000.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(100.0)
                        .with_unit("entries".into()),
                ),
            },
            Parameter {
                id: "level0_compaction_trigger".into(),
                name: "L0 Compaction Trigger".into(),
                param_type: ParameterType::Number,
                description: "Number of L0 SSTables before triggering compaction".into(),
                default_value: ParameterValue::Integer(4),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(2.0).with_max(20.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1.0)
                        .with_help_text("More = fewer compactions but slower reads".into()),
                ),
            },
            Parameter {
                id: "size_ratio".into(),
                name: "Size Ratio".into(),
                param_type: ParameterType::Number,
                description: "Size multiplier between adjacent levels".into(),
                default_value: ParameterValue::Integer(10),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(2.0).with_max(20.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1.0)
                        .with_help_text("Higher = fewer levels but more write amplification".into()),
                ),
            },
            Parameter {
                id: "key_column".into(),
                name: "Key Column".into(),
                param_type: ParameterType::String,
                description: "Name of the column to use as the key".into(),
                default_value: ParameterValue::String("id".into()),
                required: true,
                constraints: None,
                ui_hint: Some(ParameterUIHint::new(WidgetType::Input)),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "memtable_entries".into(),
                name: "Memtable Entries".into(),
                metric_type: MetricType::Gauge,
                unit: "entries".into(),
                description: "Current entries in the active memtable".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "total_sstables".into(),
                name: "Total SSTables".into(),
                metric_type: MetricType::Gauge,
                unit: "tables".into(),
                description: "Total SSTables across all levels".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "level_count".into(),
                name: "Level Count".into(),
                metric_type: MetricType::Gauge,
                unit: "levels".into(),
                description: "Number of non-empty levels".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "flushes".into(),
                name: "Flushes".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Memtable flushes to Level 0".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "compactions".into(),
                name: "Compactions".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Compaction operations performed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "bloom_true_negatives".into(),
                name: "Bloom True Negatives".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Reads skipped by bloom filter (correctly)".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "bloom_false_positives".into(),
                name: "Bloom False Positives".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Bloom filter said yes but key was absent".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "write_amplification".into(),
                name: "Write Amplification".into(),
                metric_type: MetricType::Gauge,
                unit: "x".into(),
                description: "Total bytes written / user bytes written".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Insert a key-value pair into the memtable.
    pub fn put(&mut self, key: String, value: JsonValue) {
        let entry_size = key.len() + value.to_string().len() + 16;
        self.user_bytes_written += entry_size;
        self.memtable.insert(key, value);

        if self.memtable.len() >= self.memtable_size {
            self.flush_memtable();
        }
    }

    /// Point lookup — checks memtable, then L0 (newest first), then higher levels.
    pub fn get(&mut self, key: &str) -> Option<JsonValue> {
        // 1. Check memtable
        if let Some(v) = self.memtable.get(key) {
            return Some(v.clone());
        }

        // 2. Check each level, newest SSTables first
        let mut tables_checked = 0;
        for level in &self.levels {
            for sst in level.iter().rev() {
                if !sst.bloom.might_contain(key) {
                    self.bloom_true_negatives += 1;
                    continue;
                }
                tables_checked += 1;
                if let Some(v) = sst.lookup(key) {
                    return Some(v.clone());
                } else {
                    self.bloom_false_positives += 1;
                }
            }
        }
        let _ = tables_checked;
        None
    }

    /// Flush the active memtable to Level 0 as a new SSTable.
    fn flush_memtable(&mut self) {
        if self.memtable.is_empty() {
            return;
        }

        let entries: Vec<(String, JsonValue)> = self.memtable.drain_filter_compat();
        let sst = SSTable::from_entries(entries);
        self.total_bytes_written += sst.size_bytes;
        self.levels[0].push(sst);
        self.flush_count += 1;

        // Check if L0 needs compaction.
        if self.levels[0].len() >= self.level0_compaction_trigger {
            self.compact_level(0);
        }
    }

    /// Merge all SSTables at the given level into the next level.
    fn compact_level(&mut self, level: usize) {
        if level + 1 >= self.levels.len() {
            // Add a new level if needed.
            self.levels.push(Vec::new());
        }

        // Collect all entries from this level.
        let mut all_entries: Vec<(String, JsonValue)> = Vec::new();
        for sst in self.levels[level].drain(..) {
            all_entries.extend(sst.entries);
        }

        // Also merge with existing entries at the next level.
        for sst in self.levels[level + 1].drain(..) {
            all_entries.extend(sst.entries);
        }

        // Sort and deduplicate (keep latest value for duplicate keys).
        all_entries.sort_by(|a, b| a.0.cmp(&b.0));
        all_entries.dedup_by(|a, b| a.0 == b.0);

        // Create a new SSTable at the next level.
        let sst = SSTable::from_entries(all_entries);
        self.total_bytes_written += sst.size_bytes;
        self.levels[level + 1].push(sst);
        self.compaction_count += 1;

        // Check if next level also needs compaction.
        let max_tables = self.level0_compaction_trigger * self.size_ratio.pow(level as u32 + 1);
        let next_total_entries: usize = self.levels[level + 1].iter().map(|s| s.len()).sum();
        if next_total_entries > max_tables * self.memtable_size {
            self.compact_level(level + 1);
        }
    }

    /// Total number of SSTables across all levels.
    pub fn total_sstables(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Number of non-empty levels.
    pub fn non_empty_levels(&self) -> usize {
        self.levels.iter().filter(|l| !l.is_empty()).count()
    }

    /// Total entries across memtable and all SSTables.
    pub fn total_entries(&self) -> usize {
        let sst_entries: usize = self
            .levels
            .iter()
            .flat_map(|l| l.iter())
            .map(|s| s.len())
            .sum();
        self.memtable.len() + sst_entries
    }

    /// Write amplification factor.
    pub fn write_amplification(&self) -> f64 {
        if self.user_bytes_written == 0 {
            1.0
        } else {
            self.total_bytes_written as f64 / self.user_bytes_written as f64
        }
    }
}

/// Compatibility helper — BTreeMap doesn't have drain_filter in stable Rust.
trait DrainFilterCompat<K, V> {
    fn drain_filter_compat(&mut self) -> Vec<(K, V)>;
}

impl<K: Ord + Clone, V: Clone> DrainFilterCompat<K, V> for BTreeMap<K, V> {
    fn drain_filter_compat(&mut self) -> Vec<(K, V)> {
        let entries: Vec<(K, V)> = self.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        self.clear();
        entries
    }
}

impl Default for LSMTreeBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Block trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Block for LSMTreeBlock {
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
        if let Some(val) = params.get("memtable_size") {
            let v = val.as_integer().ok_or_else(|| {
                BlockError::InvalidParameter("memtable_size must be an integer".into())
            })? as usize;
            if v < 10 || v > 100000 {
                return Err(BlockError::InvalidParameter(
                    "memtable_size must be between 10 and 100000".into(),
                ));
            }
            self.memtable_size = v;
        }
        if let Some(val) = params.get("level0_compaction_trigger") {
            let v = val.as_integer().ok_or_else(|| {
                BlockError::InvalidParameter("level0_compaction_trigger must be an integer".into())
            })? as usize;
            if v < 2 || v > 20 {
                return Err(BlockError::InvalidParameter(
                    "level0_compaction_trigger must be between 2 and 20".into(),
                ));
            }
            self.level0_compaction_trigger = v;
        }
        if let Some(val) = params.get("size_ratio") {
            let v = val.as_integer().ok_or_else(|| {
                BlockError::InvalidParameter("size_ratio must be an integer".into())
            })? as usize;
            if v < 2 || v > 20 {
                return Err(BlockError::InvalidParameter(
                    "size_ratio must be between 2 and 20".into(),
                ));
            }
            self.size_ratio = v;
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
            // Use the "id" field (or first field) as key.
            let key = record
                .data
                .get("id")
                .map(|v| v.to_string())
                .unwrap_or_else(|| format!("key_{}", self.total_entries()));

            let value = serde_json::to_value(&record.data).unwrap_or(JsonValue::Null);
            self.put(key, value);

            context.metrics.increment("records_written");
            output_records.push(record);
        }

        // Flush any remaining memtable entries.
        self.flush_memtable();

        // Record gauges.
        context
            .metrics
            .record("memtable_entries", self.memtable.len() as f64);
        context
            .metrics
            .record("total_sstables", self.total_sstables() as f64);
        context
            .metrics
            .record("level_count", self.non_empty_levels() as f64);
        context
            .metrics
            .record("flushes", self.flush_count as f64);
        context
            .metrics
            .record("compactions", self.compaction_count as f64);
        context.metrics.record(
            "bloom_true_negatives",
            self.bloom_true_negatives as f64,
        );
        context.metrics.record(
            "bloom_false_positives",
            self.bloom_false_positives as f64,
        );
        context
            .metrics
            .record("write_amplification", self.write_amplification());

        let mut outputs = HashMap::new();
        outputs.insert("stored".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("total_sstables".into(), self.total_sstables() as f64);
        metrics_summary.insert("level_count".into(), self.non_empty_levels() as f64);
        metrics_summary.insert("flushes".into(), self.flush_count as f64);
        metrics_summary.insert("compactions".into(), self.compaction_count as f64);
        metrics_summary.insert("write_amplification".into(), self.write_amplification());

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
                PortValue::None => {
                    ValidationResult::ok().with_warning("No records provided — nothing to store")
                }
                _ => ValidationResult::error("records port expects DataStream, Batch, or Single"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("memtable_size".into(), self.memtable_size);
        let _ = state.insert("memtable_entries".into(), self.memtable.len());
        let _ = state.insert("total_sstables".into(), self.total_sstables());
        let _ = state.insert("total_entries".into(), self.total_entries());
        state
    }

    fn set_state(&mut self, state: BlockState) -> Result<(), BlockError> {
        if let Ok(Some(ms)) = state.get::<usize>("memtable_size") {
            self.memtable_size = ms;
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
    use serde_json::json;

    #[test]
    fn test_basic_put_and_get() {
        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 100;

        lsm.put("key1".into(), json!({"name": "Alice"}));
        lsm.put("key2".into(), json!({"name": "Bob"}));

        assert_eq!(lsm.get("key1"), Some(json!({"name": "Alice"})));
        assert_eq!(lsm.get("key2"), Some(json!({"name": "Bob"})));
        assert_eq!(lsm.get("key3"), None);
    }

    #[test]
    fn test_memtable_flush() {
        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 10;

        for i in 0..25 {
            lsm.put(format!("key_{:04}", i), json!(i));
        }

        assert!(lsm.flush_count > 0, "Should have flushed at least once");
        assert!(lsm.levels[0].len() > 0 || lsm.levels[1].len() > 0,
            "Should have SSTables");

        // All keys should still be readable.
        for i in 0..25 {
            assert!(
                lsm.get(&format!("key_{:04}", i)).is_some(),
                "Key {} should be readable after flush",
                i
            );
        }
    }

    #[test]
    fn test_compaction_triggered() {
        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 5;
        lsm.level0_compaction_trigger = 3;

        // Insert enough to trigger multiple flushes and at least one compaction.
        for i in 0..50 {
            lsm.put(format!("key_{:04}", i), json!(i));
        }
        lsm.flush_memtable();

        assert!(
            lsm.compaction_count > 0,
            "Should have triggered at least one compaction"
        );
    }

    #[test]
    fn test_bloom_filter_basic() {
        let mut bloom = BloomFilter::new(100, 0.01);
        bloom.insert("hello");
        bloom.insert("world");

        assert!(bloom.might_contain("hello"));
        assert!(bloom.might_contain("world"));
        // "missing" might occasionally false-positive, but usually not
    }

    #[test]
    fn test_bloom_filter_false_positive_rate() {
        let n = 1000;
        let mut bloom = BloomFilter::new(n, 0.01);
        for i in 0..n {
            bloom.insert(&format!("key_{}", i));
        }

        let mut false_positives = 0;
        let test_count = 10000;
        for i in n..(n + test_count) {
            if bloom.might_contain(&format!("key_{}", i)) {
                false_positives += 1;
            }
        }

        let fp_rate = false_positives as f64 / test_count as f64;
        assert!(
            fp_rate < 0.05,
            "False positive rate {} is too high (expected < 5%)",
            fp_rate
        );
    }

    #[test]
    fn test_sstable_sorted_lookup() {
        let entries = vec![
            ("c".into(), json!(3)),
            ("a".into(), json!(1)),
            ("b".into(), json!(2)),
        ];
        let sst = SSTable::from_entries(entries);

        assert_eq!(sst.lookup("a"), Some(&json!(1)));
        assert_eq!(sst.lookup("b"), Some(&json!(2)));
        assert_eq!(sst.lookup("c"), Some(&json!(3)));
        assert_eq!(sst.lookup("d"), None);
    }

    #[test]
    fn test_write_amplification() {
        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 10;
        lsm.level0_compaction_trigger = 2;

        for i in 0..100 {
            lsm.put(format!("key_{:04}", i), json!(i));
        }
        lsm.flush_memtable();

        let wa = lsm.write_amplification();
        assert!(
            wa >= 1.0,
            "Write amplification should be >= 1.0, got {}",
            wa
        );
    }

    #[test]
    fn test_overwrite_key() {
        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 100;

        lsm.put("key1".into(), json!({"version": 1}));
        lsm.put("key1".into(), json!({"version": 2}));

        // Should return the latest value.
        assert_eq!(lsm.get("key1"), Some(json!({"version": 2})));
    }

    #[test]
    fn test_metadata() {
        let lsm = LSMTreeBlock::new();
        assert_eq!(lsm.metadata().id, "lsm-tree-storage");
        assert_eq!(lsm.metadata().category, BlockCategory::Storage);
        assert_eq!(lsm.inputs().len(), 1);
        assert_eq!(lsm.outputs().len(), 1);
        assert_eq!(lsm.parameters().len(), 4);
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut lsm = LSMTreeBlock::new();
        let mut params = HashMap::new();
        params.insert("memtable_size".into(), ParameterValue::Integer(500));
        params.insert(
            "level0_compaction_trigger".into(),
            ParameterValue::Integer(6),
        );
        params.insert("size_ratio".into(), ParameterValue::Integer(5));

        lsm.initialize(params).await.unwrap();
        assert_eq!(lsm.memtable_size, 500);
        assert_eq!(lsm.level0_compaction_trigger, 6);
        assert_eq!(lsm.size_ratio, 5);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut lsm = LSMTreeBlock::new();
        lsm.memtable_size = 10;

        let records: Vec<Record> = (0..20)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("name".into(), format!("user_{}", i)).unwrap();
                r
            })
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

        let result = lsm.execute(ctx).await.unwrap();
        assert!(result.errors.is_empty());

        let stored = result.outputs.get("stored").unwrap();
        assert_eq!(stored.len(), 20);
        assert!(*result.metrics.get("flushes").unwrap() > 0.0);
    }
}
