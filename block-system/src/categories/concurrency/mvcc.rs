//! MVCC (Multi-Version Concurrency Control) Block
//!
//! Implements **snapshot isolation** via multi-version concurrency control.
//! Each write creates a new version of a record rather than overwriting in
//! place, allowing readers to see a consistent snapshot without blocking
//! writers.
//!
//! ## How it works
//!
//! Every record version carries a **creation timestamp** (`xmin`) and an
//! optional **deletion timestamp** (`xmax`). A transaction sees a version if:
//! - `xmin <= txn_timestamp` (version was created before the snapshot)
//! - `xmax` is None or `xmax > txn_timestamp` (version was not yet deleted)
//!
//! Garbage collection removes versions that are no longer visible to any
//! active transaction.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `versions_created` | Counter | New versions written |
//! | `versions_visible` | Gauge | Versions visible to latest snapshot |
//! | `versions_garbage` | Gauge | Versions eligible for GC |
//! | `gc_runs` | Counter | Garbage collection cycles |
//! | `gc_reclaimed` | Counter | Versions reclaimed by GC |
//! | `snapshot_reads` | Counter | Reads served from snapshot |
//! | `write_conflicts` | Counter | Write-write conflicts detected |
//! | `chain_length_avg` | Gauge | Average version chain length |

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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
// Internal version model
// ---------------------------------------------------------------------------

type Timestamp = u64;

#[derive(Debug, Clone)]
struct Version {
    data: JsonValue,
    xmin: Timestamp, // Created by this transaction
    xmax: Option<Timestamp>, // Deleted by this transaction (None = still live)
}

/// A version chain for a single key.
#[derive(Debug, Clone)]
struct VersionChain {
    versions: Vec<Version>, // Newest first
}

impl VersionChain {
    fn new() -> Self {
        Self {
            versions: Vec::new(),
        }
    }

    /// Add a new version at the head of the chain.
    fn add_version(&mut self, data: JsonValue, xmin: Timestamp) {
        self.versions.insert(
            0,
            Version {
                data,
                xmin,
                xmax: None,
            },
        );
    }

    /// Mark the latest live version as deleted.
    fn delete_latest(&mut self, xmax: Timestamp) -> bool {
        for v in &mut self.versions {
            if v.xmax.is_none() {
                v.xmax = Some(xmax);
                return true;
            }
        }
        false
    }

    /// Find the visible version for a given snapshot timestamp.
    fn visible_at(&self, ts: Timestamp) -> Option<&Version> {
        for v in &self.versions {
            if v.xmin <= ts && (v.xmax.is_none() || v.xmax.unwrap() > ts) {
                return Some(v);
            }
        }
        None
    }

    /// Count versions visible to no active transaction (all below min_active).
    fn garbage_versions(&self, min_active: Timestamp) -> usize {
        self.versions
            .iter()
            .filter(|v| {
                // A version is garbage if it was deleted before any active txn
                v.xmax.map_or(false, |xmax| xmax < min_active)
            })
            .count()
    }

    /// Remove garbage versions.
    fn gc(&mut self, min_active: Timestamp) -> usize {
        let before = self.versions.len();
        self.versions
            .retain(|v| !v.xmax.map_or(false, |xmax| xmax < min_active));
        before - self.versions.len()
    }
}

// ---------------------------------------------------------------------------
// MVCCBlock
// ---------------------------------------------------------------------------

pub struct MVCCBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    gc_threshold: usize,

    // Internal state
    /// Key → version chain.
    store: HashMap<String, VersionChain>,
    /// Current timestamp counter.
    current_ts: Timestamp,
    /// Active transactions: txn_ts → snapshot_ts.
    active_txns: HashMap<Timestamp, Timestamp>,
    /// Committed transactions: txn_ts → commit_ts.
    commit_times: HashMap<Timestamp, Timestamp>,

    // Counters
    versions_created: usize,
    gc_runs: usize,
    gc_reclaimed: usize,
    snapshot_reads: usize,
    write_conflicts: usize,
}

impl MVCCBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            gc_threshold: 100,
            store: HashMap::new(),
            current_ts: 1,
            active_txns: HashMap::new(),
            commit_times: HashMap::new(),
            versions_created: 0,
            gc_runs: 0,
            gc_reclaimed: 0,
            snapshot_reads: 0,
            write_conflicts: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "mvcc".into(),
            name: "MVCC".into(),
            category: BlockCategory::Concurrency,
            description: "Multi-Version Concurrency Control with snapshot isolation".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "MVCC (Multi-Version Concurrency Control) allows multiple transactions to read and \
                           write concurrently without blocking each other. Instead of overwriting data in place, \
                           writers create new versions of each record. Readers see a consistent snapshot of the \
                           database based on their start timestamp, completely unaffected by concurrent writers.\n\n\
                           MVCC is the concurrency control mechanism used by most modern databases including \
                           PostgreSQL, MySQL InnoDB, Oracle, and CockroachDB. It implements snapshot isolation, \
                           where each transaction sees the database as it existed at the moment the transaction \
                           began. This eliminates the need for read locks entirely, dramatically improving \
                           throughput for mixed read/write workloads.\n\n\
                           Think of MVCC like a document editing system with full version history. When someone \
                           edits a document, they create a new version rather than modifying the original. Anyone \
                           who opened the document before the edit still sees the old version. The system needs \
                           to periodically clean up old versions that nobody is reading anymore — this is garbage \
                           collection (VACUUM in PostgreSQL)."
                    .into(),
                algorithm: "WRITE(txn_ts, key, data):\n  \
                           1. Check for write-write conflict:\n     \
                              If another txn wrote this key and committed after our snapshot -> CONFLICT\n  \
                           2. Mark the current latest version as deleted (set xmax = txn_ts)\n  \
                           3. Create new version: Version { data, xmin=txn_ts, xmax=None }\n  \
                           4. Insert at head of version chain\n  \
                           5. If versions_created % gc_threshold == 0: trigger GC\n\n\
                           READ(snapshot_ts, key):\n  \
                           Walk version chain for key:\n    \
                             For each version v:\n      \
                               If v.xmin <= snapshot_ts AND (v.xmax is None OR v.xmax > snapshot_ts):\n        \
                                 Return v.data  (this version is visible)\n    \
                             Return None (key does not exist at this snapshot)\n\n\
                           GARBAGE_COLLECTION():\n  \
                           min_active = minimum timestamp among all active transactions\n  \
                           For each key's version chain:\n    \
                             Remove versions where xmax < min_active\n    \
                             (These versions are invisible to ALL current and future transactions)"
                    .into(),
                complexity: Complexity {
                    time: "Read O(v) where v = chain length, Write O(1), GC O(n × v)".into(),
                    space: "O(n × v) — one version per write per key".into(),
                },
                use_cases: vec![
                    "OLTP with mixed read/write workloads".into(),
                    "Snapshot isolation (PostgreSQL default)".into(),
                    "Long-running read queries alongside writes".into(),
                    "Analytical queries that need a consistent view without blocking writers".into(),
                    "Time-travel queries that read data as of a past timestamp".into(),
                ],
                tradeoffs: vec![
                    "Readers never block writers and vice versa".into(),
                    "Space overhead from multiple versions per key".into(),
                    "GC is necessary to reclaim old versions".into(),
                    "Write-write conflicts on the same key must be detected".into(),
                    "Version chain traversal slows reads when chains grow long (GC lag)".into(),
                    "MVCC provides snapshot isolation by default, but not true serializability without extra checks (SSI)".into(),
                ],
                examples: vec![
                    "PostgreSQL MVCC — uses xmin/xmax system columns on every tuple, VACUUM reclaims dead tuples".into(),
                    "MySQL InnoDB — stores old versions in undo log, reads reconstruct from undo chain".into(),
                    "Oracle Multiversion Read Consistency — uses undo tablespace for version storage".into(),
                    "CockroachDB — MVCC with timestamp ordering, built on RocksDB/Pebble storage".into(),
                ],
                motivation: "Without MVCC, databases must use locks to prevent readers from seeing partially \
                             updated data. This means a long-running analytical query would block all writes \
                             to the rows it is reading, or worse, a write would block all concurrent reads. \
                             In a busy OLTP system, this lock contention destroys throughput.\n\n\
                             MVCC solves this by letting readers and writers operate on different versions of \
                             the same data simultaneously. The cost is additional storage for old versions and \
                             the need for garbage collection. But the benefit — eliminating read-write contention \
                             entirely — is so significant that MVCC has become the dominant concurrency control \
                             strategy in modern databases."
                    .into(),
                parameter_guide: HashMap::from([
                    ("gc_threshold".into(),
                     "Controls how often garbage collection runs, measured in number of writes between GC \
                      cycles. A lower value (10-50) means GC runs frequently, keeping version chains short \
                      and memory usage low, but adds CPU overhead per write. A higher value (500-10000) \
                      reduces GC overhead but lets old versions accumulate, increasing memory usage and \
                      slowing reads that must traverse longer chains. In PostgreSQL, autovacuum is triggered \
                      by a similar threshold (autovacuum_vacuum_threshold + autovacuum_vacuum_scale_factor \
                      × table size). Recommended: 100 for balanced workloads, lower for write-heavy, higher \
                      for read-heavy with infrequent updates."
                        .into()),
                ]),
                alternatives: vec![
                    Alternative {
                        block_type: "row-lock-2pl".into(),
                        comparison: "Row-level locking (2PL) guarantees true serializability but blocks readers \
                                     when a writer holds an exclusive lock. MVCC provides snapshot isolation \
                                     without read-write blocking, at the cost of extra storage for versions \
                                     and the need for garbage collection. Choose 2PL for workloads requiring \
                                     strict serializability with short transactions. Choose MVCC for mixed \
                                     OLTP/OLAP workloads where long reads should not block writes."
                            .into(),
                    },
                ],
                suggested_questions: vec![
                    "How does PostgreSQL's VACUUM process work, and what happens if it falls behind?".into(),
                    "What is the difference between snapshot isolation and serializable isolation in MVCC?".into(),
                    "How do write-write conflicts get detected, and what should happen when one occurs?".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Paper,
                title: "An Empirical Evaluation of In-Memory MVCC".into(),
                url: None,
                citation: Some(
                    "Wu, Y. et al. (2017). PVLDB, 10(7), 781–792.".into(),
                ),
            }],
            icon: "git-compare".into(),
            color: "#A855F7".into(),
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
            description: "Records to process with MVCC versioning".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "visible".into(),
            name: "Visible Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records visible at the latest snapshot".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "gc_threshold".into(),
            name: "GC Threshold".into(),
            param_type: ParameterType::Number,
            description: "Run garbage collection every N writes".into(),
            default_value: ParameterValue::Integer(100),
            required: false,
            constraints: Some(
                ParameterConstraints::new()
                    .with_min(10.0)
                    .with_max(10000.0),
            ),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Slider)
                    .with_step(10.0)
                    .with_help_text("Lower = less space overhead, higher = less GC cost".into()),
            ),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "versions_created".into(),
                name: "Versions Created".into(),
                metric_type: MetricType::Counter,
                unit: "versions".into(),
                description: "Total new versions written".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "versions_visible".into(),
                name: "Visible Versions".into(),
                metric_type: MetricType::Gauge,
                unit: "versions".into(),
                description: "Versions visible at latest snapshot".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "versions_garbage".into(),
                name: "Garbage Versions".into(),
                metric_type: MetricType::Gauge,
                unit: "versions".into(),
                description: "Versions eligible for GC".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "gc_runs".into(),
                name: "GC Runs".into(),
                metric_type: MetricType::Counter,
                unit: "runs".into(),
                description: "Garbage collection cycles".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "gc_reclaimed".into(),
                name: "GC Reclaimed".into(),
                metric_type: MetricType::Counter,
                unit: "versions".into(),
                description: "Versions reclaimed by GC".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "snapshot_reads".into(),
                name: "Snapshot Reads".into(),
                metric_type: MetricType::Counter,
                unit: "reads".into(),
                description: "Reads served from snapshot".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "write_conflicts".into(),
                name: "Write Conflicts".into(),
                metric_type: MetricType::Counter,
                unit: "conflicts".into(),
                description: "Write-write conflicts detected".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "chain_length_avg".into(),
                name: "Avg Chain Length".into(),
                metric_type: MetricType::Gauge,
                unit: "versions".into(),
                description: "Average version chain length".into(),
                aggregations: vec![AggregationType::Avg],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Begin a transaction, returns its timestamp.
    pub fn begin_txn(&mut self) -> Timestamp {
        let ts = self.current_ts;
        self.current_ts += 1;
        self.active_txns.insert(ts, ts);
        ts
    }

    /// Write a new version of a key.
    pub fn write(&mut self, txn_ts: Timestamp, key: &str, data: JsonValue) -> bool {
        let chain = self
            .store
            .entry(key.to_string())
            .or_insert_with(VersionChain::new);

        // Check for write-write conflict: if another transaction wrote this key
        // and committed after our snapshot was taken, it's a conflict.
        if let Some(latest) = chain.versions.first() {
            if latest.xmin != txn_ts {
                if let Some(&commit_ts) = self.commit_times.get(&latest.xmin) {
                    // Writer committed after our snapshot → conflict
                    if commit_ts >= txn_ts {
                        self.write_conflicts += 1;
                        return false;
                    }
                } else {
                    // Writer hasn't committed yet → concurrent → conflict
                    self.write_conflicts += 1;
                    return false;
                }
            }
        }

        // Mark old version as deleted (if exists).
        chain.delete_latest(txn_ts);

        // Create new version.
        chain.add_version(data, txn_ts);
        self.versions_created += 1;

        // Maybe trigger GC.
        if self.versions_created % self.gc_threshold == 0 {
            self.run_gc();
        }

        true
    }

    /// Read the visible version of a key at a snapshot timestamp.
    pub fn read(&mut self, snapshot_ts: Timestamp, key: &str) -> Option<JsonValue> {
        self.snapshot_reads += 1;
        self.store
            .get(key)
            .and_then(|chain| chain.visible_at(snapshot_ts))
            .map(|v| v.data.clone())
    }

    /// Commit a transaction.
    pub fn commit(&mut self, txn_ts: Timestamp) {
        self.active_txns.remove(&txn_ts);
        let commit_ts = self.current_ts;
        self.current_ts += 1;
        self.commit_times.insert(txn_ts, commit_ts);
    }

    /// Run garbage collection.
    pub fn run_gc(&mut self) {
        let min_active = self
            .active_txns
            .keys()
            .copied()
            .min()
            .unwrap_or(self.current_ts);

        let mut reclaimed = 0;
        for chain in self.store.values_mut() {
            reclaimed += chain.gc(min_active);
        }

        // Remove empty chains.
        self.store.retain(|_, chain| !chain.versions.is_empty());

        self.gc_runs += 1;
        self.gc_reclaimed += reclaimed;
    }

    /// Count total versions across all chains.
    pub fn total_versions(&self) -> usize {
        self.store.values().map(|c| c.versions.len()).sum()
    }

    /// Count versions visible at a given timestamp.
    fn visible_count(&self, ts: Timestamp) -> usize {
        self.store
            .values()
            .filter(|c| c.visible_at(ts).is_some())
            .count()
    }

    /// Count garbage versions.
    fn garbage_count(&self) -> usize {
        let min_active = self
            .active_txns
            .keys()
            .copied()
            .min()
            .unwrap_or(self.current_ts);
        self.store
            .values()
            .map(|c| c.garbage_versions(min_active))
            .sum()
    }

    /// Average version chain length.
    fn avg_chain_length(&self) -> f64 {
        if self.store.is_empty() {
            return 0.0;
        }
        self.total_versions() as f64 / self.store.len() as f64
    }
}

impl Default for MVCCBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for MVCCBlock {
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
            vec![Guarantee::strict(
                GuaranteeType::Consistency,
                "Snapshot isolation — readers see a consistent point-in-time view",
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
        if let Some(val) = params.get("gc_threshold") {
            self.gc_threshold = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("gc_threshold must be an integer".into())
                })? as usize;
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

        // Simulate: each record is a write in its own transaction.
        for record in &records {
            let txn = self.begin_txn();
            let key = record
                .data
                .get("id")
                .map(|v| v.to_string())
                .unwrap_or_else(|| format!("key_{}", txn));
            let data = serde_json::to_value(&record.data).unwrap_or(JsonValue::Null);
            self.write(txn, &key, data);
            self.commit(txn);
        }

        // Read snapshot at current time.
        let snap_ts = self.current_ts;
        let visible = self.visible_count(snap_ts);

        context
            .metrics
            .record("versions_created", self.versions_created as f64);
        context
            .metrics
            .record("versions_visible", visible as f64);
        context
            .metrics
            .record("versions_garbage", self.garbage_count() as f64);
        context.metrics.record("gc_runs", self.gc_runs as f64);
        context
            .metrics
            .record("gc_reclaimed", self.gc_reclaimed as f64);
        context
            .metrics
            .record("snapshot_reads", self.snapshot_reads as f64);
        context
            .metrics
            .record("write_conflicts", self.write_conflicts as f64);
        context
            .metrics
            .record("chain_length_avg", self.avg_chain_length());

        let mut outputs = HashMap::new();
        outputs.insert("visible".into(), PortValue::Stream(records.to_vec()));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("versions_created".into(), self.versions_created as f64);
        metrics_summary.insert("versions_visible".into(), visible as f64);
        metrics_summary.insert("gc_runs".into(), self.gc_runs as f64);
        metrics_summary.insert("gc_reclaimed".into(), self.gc_reclaimed as f64);
        metrics_summary.insert("chain_length_avg".into(), self.avg_chain_length());

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
                    ValidationResult::ok().with_warning("No records to process")
                }
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("total_versions".into(), self.total_versions());
        let _ = state.insert("versions_created".into(), self.versions_created);
        state
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
    use serde_json::json;

    #[test]
    fn test_basic_write_and_read() {
        let mut mvcc = MVCCBlock::new();
        let txn = mvcc.begin_txn();

        mvcc.write(txn, "key1", json!({"name": "Alice"}));
        mvcc.commit(txn);

        let txn2 = mvcc.begin_txn();
        let result = mvcc.read(txn2, "key1");
        assert_eq!(result, Some(json!({"name": "Alice"})));
        assert_eq!(mvcc.read(txn2, "key2"), None);
    }

    #[test]
    fn test_snapshot_isolation() {
        let mut mvcc = MVCCBlock::new();

        // txn1 writes key1.
        let txn1 = mvcc.begin_txn();
        mvcc.write(txn1, "key1", json!({"version": 1}));
        mvcc.commit(txn1);

        // txn2 takes a snapshot (sees version 1).
        let txn2 = mvcc.begin_txn();

        // txn3 updates key1 to version 2 after txn2's snapshot.
        let txn3 = mvcc.begin_txn();
        mvcc.write(txn3, "key1", json!({"version": 2}));
        mvcc.commit(txn3);

        // txn2 should still see version 1 (snapshot isolation).
        let visible = mvcc.read(txn2, "key1");
        assert_eq!(visible, Some(json!({"version": 1})));

        // A new txn should see version 2.
        let txn4 = mvcc.begin_txn();
        let latest = mvcc.read(txn4, "key1");
        assert_eq!(latest, Some(json!({"version": 2})));
    }

    #[test]
    fn test_write_conflict() {
        let mut mvcc = MVCCBlock::new();
        mvcc.gc_threshold = 10000; // Prevent GC during test

        let txn1 = mvcc.begin_txn();
        let txn2 = mvcc.begin_txn();

        // Both try to write the same key.
        assert!(mvcc.write(txn1, "key1", json!(1)));
        mvcc.commit(txn1);

        // txn2 should detect a conflict (txn1 wrote after txn2's start).
        assert!(!mvcc.write(txn2, "key1", json!(2)));
        assert_eq!(mvcc.write_conflicts, 1);
    }

    #[test]
    fn test_garbage_collection() {
        let mut mvcc = MVCCBlock::new();
        mvcc.gc_threshold = 10000; // Manual GC

        // Create multiple versions of the same key.
        for i in 0..5 {
            let txn = mvcc.begin_txn();
            mvcc.write(txn, "key1", json!(i));
            mvcc.commit(txn);
        }

        assert!(mvcc.total_versions() > 1, "Should have multiple versions");

        // Run GC — all old versions should be reclaimable since no active txns.
        mvcc.run_gc();
        assert_eq!(
            mvcc.total_versions(),
            1,
            "GC should keep only the latest version"
        );
        assert!(mvcc.gc_reclaimed > 0);
    }

    #[test]
    fn test_version_chain_length() {
        let mut mvcc = MVCCBlock::new();
        mvcc.gc_threshold = 10000;

        // Write 3 versions of same key without GC.
        for i in 0..3 {
            let txn = mvcc.begin_txn();
            mvcc.write(txn, "key1", json!(i));
            mvcc.commit(txn);
        }

        assert!(mvcc.avg_chain_length() >= 1.0);
    }

    #[test]
    fn test_metadata() {
        let mvcc = MVCCBlock::new();
        assert_eq!(mvcc.metadata().id, "mvcc");
        assert_eq!(mvcc.metadata().category, BlockCategory::Concurrency);
        assert_eq!(mvcc.inputs().len(), 1);
        assert_eq!(mvcc.outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut mvcc = MVCCBlock::new();

        let records: Vec<Record> = (0..20)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
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

        let result = mvcc.execute(ctx).await.unwrap();
        assert_eq!(*result.metrics.get("versions_created").unwrap(), 20.0);
        assert!(result.errors.is_empty());
    }
}
