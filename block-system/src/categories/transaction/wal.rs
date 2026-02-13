//! Write-Ahead Log (WAL) Transaction Block
//!
//! Implements a **write-ahead log** — the foundational technique for crash
//! recovery. Every modification is first written to an append-only log before
//! the actual data is changed, ensuring that committed transactions survive
//! crashes.
//!
//! ## How it works
//!
//! 1. Before any data modification, a **log record** is appended to the WAL.
//! 2. The log is **fsync'd** (simulated) to ensure durability before
//!    acknowledging the write.
//! 3. Periodically, a **checkpoint** flushes all dirty data and records the
//!    checkpoint LSN, allowing older log entries to be recycled.
//!
//! ## Log record types
//!
//! | Type | Fields | Purpose |
//! |------|--------|---------|
//! | INSERT | table, data | New record added |
//! | UPDATE | table, before, after | Record modified |
//! | DELETE | table, data | Record removed |
//! | COMMIT | txn_id | Transaction committed |
//! | CHECKPOINT | lsn | Recovery point |
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `log_entries` | Counter | Total log records written |
//! | `bytes_written` | Counter | Total bytes written to log |
//! | `fsyncs` | Counter | fsync operations (simulated) |
//! | `checkpoints` | Counter | Checkpoint operations |
//! | `log_size_bytes` | Gauge | Current log file size |
//! | `oldest_lsn` | Gauge | Oldest un-checkpointed LSN |

use async_trait::async_trait;
use std::collections::HashMap;

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
// Internal WAL model
// ---------------------------------------------------------------------------

/// Log Sequence Number — monotonically increasing.
type LSN = u64;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LogRecordType {
    Insert,
    Update,
    Delete,
    Commit,
    Checkpoint,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LogRecord {
    lsn: LSN,
    record_type: LogRecordType,
    size_bytes: usize,
}

// ---------------------------------------------------------------------------
// WALBlock
// ---------------------------------------------------------------------------

pub struct WALBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    fsync_interval: usize,      // Fsync every N log entries
    checkpoint_interval: usize, // Checkpoint every N log entries

    // Internal state
    log: Vec<LogRecord>,
    next_lsn: LSN,
    last_checkpoint_lsn: LSN,
    total_bytes: usize,

    // Counters
    fsync_count: usize,
    checkpoint_count: usize,
    entries_since_fsync: usize,
    entries_since_checkpoint: usize,
}

impl WALBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            fsync_interval: 1,
            checkpoint_interval: 100,
            log: Vec::new(),
            next_lsn: 1,
            last_checkpoint_lsn: 0,
            total_bytes: 0,
            fsync_count: 0,
            checkpoint_count: 0,
            entries_since_fsync: 0,
            entries_since_checkpoint: 0,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "wal".into(),
            name: "Write-Ahead Log".into(),
            category: BlockCategory::Transaction,
            description: "Append-only log for crash recovery and transaction durability".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "A Write-Ahead Log (WAL) ensures transaction durability by recording \
                           every modification before it is applied to the actual data. After a \
                           crash, the database replays the log to recover committed transactions \
                           and undo uncommitted ones."
                    .into(),
                algorithm: "Write: append log record with LSN, fsync. Checkpoint: flush dirty \
                            pages, write checkpoint record, advance the recovery starting point. \
                            Recovery: scan log from last checkpoint, redo committed txns, undo \
                            uncommitted ones (ARIES protocol)."
                    .into(),
                complexity: Complexity {
                    time: "O(1) per log append (sequential write), O(n) for recovery replay"
                        .into(),
                    space: "O(n) log entries between checkpoints".into(),
                },
                use_cases: vec![
                    "Crash recovery (every modern RDBMS uses WAL)".into(),
                    "Replication (ship log to replicas)".into(),
                    "Point-in-time recovery (replay log to target timestamp)".into(),
                ],
                tradeoffs: vec![
                    "Sequential writes are fast but log grows continuously".into(),
                    "Frequent fsyncs ensure durability but add latency".into(),
                    "Checkpoints reduce recovery time but pause normal operations briefly".into(),
                    "Group commit amortizes fsync cost across multiple transactions".into(),
                ],
                examples: vec![
                    "PostgreSQL WAL (pg_wal directory)".into(),
                    "MySQL InnoDB redo log".into(),
                    "SQLite WAL mode".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Paper,
                title: "ARIES: A Transaction Recovery Method".into(),
                url: None,
                citation: Some(
                    "Mohan, C. et al. (1992). ACM TODS, 17(1), 94–162.".into(),
                ),
            }],
            icon: "scroll-text".into(),
            color: "#F97316".into(),
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
            description: "Records representing write operations to log".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "logged".into(),
            name: "Logged Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records after being durably logged (with LSN)".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![
            Parameter {
                id: "fsync_interval".into(),
                name: "Fsync Interval".into(),
                param_type: ParameterType::Number,
                description: "Fsync every N log entries (1 = every entry, for max durability)"
                    .into(),
                default_value: ParameterValue::Integer(1),
                required: false,
                constraints: Some(
                    ParameterConstraints::new().with_min(1.0).with_max(1000.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(1.0)
                        .with_help_text("1 = safest, higher = better throughput".into()),
                ),
            },
            Parameter {
                id: "checkpoint_interval".into(),
                name: "Checkpoint Interval".into(),
                param_type: ParameterType::Number,
                description: "Checkpoint every N log entries".into(),
                default_value: ParameterValue::Integer(100),
                required: false,
                constraints: Some(
                    ParameterConstraints::new()
                        .with_min(10.0)
                        .with_max(100000.0),
                ),
                ui_hint: Some(
                    ParameterUIHint::new(WidgetType::Slider)
                        .with_step(10.0)
                        .with_help_text("Less frequent = faster writes, slower recovery".into()),
                ),
            },
        ]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "log_entries".into(),
                name: "Log Entries".into(),
                metric_type: MetricType::Counter,
                unit: "entries".into(),
                description: "Total log records written".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "bytes_written".into(),
                name: "Bytes Written".into(),
                metric_type: MetricType::Counter,
                unit: "bytes".into(),
                description: "Total bytes written to log".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "fsyncs".into(),
                name: "Fsyncs".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "fsync operations (simulated)".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "checkpoints".into(),
                name: "Checkpoints".into(),
                metric_type: MetricType::Counter,
                unit: "ops".into(),
                description: "Checkpoint operations performed".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "log_size_bytes".into(),
                name: "Log Size".into(),
                metric_type: MetricType::Gauge,
                unit: "bytes".into(),
                description: "Current total log size".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "oldest_lsn".into(),
                name: "Oldest LSN".into(),
                metric_type: MetricType::Gauge,
                unit: "lsn".into(),
                description: "Oldest un-checkpointed log sequence number".into(),
                aggregations: vec![AggregationType::Max],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Append a log record.
    pub fn append(&mut self, record_type: LogRecordType, data_size: usize) -> LSN {
        let lsn = self.next_lsn;
        let header_size = 32; // LSN + type + size + checksum
        let size_bytes = header_size + data_size;

        self.log.push(LogRecord {
            lsn,
            record_type,
            size_bytes,
        });

        self.next_lsn += 1;
        self.total_bytes += size_bytes;
        self.entries_since_fsync += 1;
        self.entries_since_checkpoint += 1;

        // Fsync if interval reached.
        if self.entries_since_fsync >= self.fsync_interval {
            self.fsync();
        }

        // Checkpoint if interval reached.
        if self.entries_since_checkpoint >= self.checkpoint_interval {
            self.checkpoint();
        }

        lsn
    }

    /// Simulate an fsync operation.
    fn fsync(&mut self) {
        self.fsync_count += 1;
        self.entries_since_fsync = 0;
    }

    /// Perform a checkpoint.
    pub fn checkpoint(&mut self) {
        let lsn = self.append_raw(LogRecordType::Checkpoint, 0);
        self.last_checkpoint_lsn = lsn;
        self.checkpoint_count += 1;
        self.entries_since_checkpoint = 0;
    }

    /// Internal append without triggering checkpoint (to avoid recursion).
    fn append_raw(&mut self, record_type: LogRecordType, data_size: usize) -> LSN {
        let lsn = self.next_lsn;
        let header_size = 32;
        let size_bytes = header_size + data_size;

        self.log.push(LogRecord {
            lsn,
            record_type,
            size_bytes,
        });

        self.next_lsn += 1;
        self.total_bytes += size_bytes;
        lsn
    }

    pub fn log_entry_count(&self) -> usize {
        self.log.len()
    }

    pub fn current_lsn(&self) -> LSN {
        self.next_lsn - 1
    }
}

impl Default for WALBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for WALBlock {
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
                GuaranteeType::Durability,
                "Committed transactions survive crashes via write-ahead logging",
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
        if let Some(val) = params.get("fsync_interval") {
            self.fsync_interval = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("fsync_interval must be an integer".into())
                })? as usize;
            if self.fsync_interval < 1 {
                return Err(BlockError::InvalidParameter(
                    "fsync_interval must be at least 1".into(),
                ));
            }
        }
        if let Some(val) = params.get("checkpoint_interval") {
            self.checkpoint_interval = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("checkpoint_interval must be an integer".into())
                })? as usize;
            if self.checkpoint_interval < 10 {
                return Err(BlockError::InvalidParameter(
                    "checkpoint_interval must be at least 10".into(),
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
            // Estimate record size from serialized data.
            let data_size = serde_json::to_string(&record.data)
                .map(|s| s.len())
                .unwrap_or(64);

            let lsn = self.append(LogRecordType::Insert, data_size);

            // Enrich output record with LSN.
            let mut out = record;
            let _ = out.insert("_lsn".into(), lsn as i64);
            output_records.push(out);
        }

        // Write a commit record for the batch.
        self.append(LogRecordType::Commit, 0);

        // Final fsync for safety.
        if self.entries_since_fsync > 0 {
            self.fsync();
        }

        context
            .metrics
            .record("log_entries", self.log.len() as f64);
        context
            .metrics
            .record("bytes_written", self.total_bytes as f64);
        context.metrics.record("fsyncs", self.fsync_count as f64);
        context
            .metrics
            .record("checkpoints", self.checkpoint_count as f64);
        context
            .metrics
            .record("log_size_bytes", self.total_bytes as f64);
        context
            .metrics
            .record("oldest_lsn", (self.last_checkpoint_lsn + 1) as f64);

        let mut outputs = HashMap::new();
        outputs.insert("logged".into(), PortValue::Stream(output_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("log_entries".into(), self.log.len() as f64);
        metrics_summary.insert("bytes_written".into(), self.total_bytes as f64);
        metrics_summary.insert("fsyncs".into(), self.fsync_count as f64);
        metrics_summary.insert("checkpoints".into(), self.checkpoint_count as f64);

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
                    ValidationResult::ok().with_warning("No records to log")
                }
                _ => ValidationResult::error("records port expects DataStream"),
            }
        } else {
            ValidationResult::ok().with_warning("records input not connected")
        }
    }

    fn get_state(&self) -> BlockState {
        let mut state = BlockState::new();
        let _ = state.insert("log_entries".into(), self.log.len());
        let _ = state.insert("total_bytes".into(), self.total_bytes);
        let _ = state.insert("current_lsn".into(), self.current_lsn() as usize);
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

    #[test]
    fn test_basic_append() {
        let mut wal = WALBlock::new();
        let lsn1 = wal.append(LogRecordType::Insert, 100);
        let lsn2 = wal.append(LogRecordType::Insert, 200);

        assert_eq!(lsn1, 1);
        assert_eq!(lsn2, 2);
        assert_eq!(wal.log_entry_count(), 2);
        assert!(wal.total_bytes > 0);
    }

    #[test]
    fn test_fsync_interval() {
        let mut wal = WALBlock::new();
        wal.fsync_interval = 3;

        for _ in 0..9 {
            wal.append(LogRecordType::Insert, 50);
        }

        assert_eq!(wal.fsync_count, 3, "Should fsync every 3 entries");
    }

    #[test]
    fn test_checkpoint() {
        let mut wal = WALBlock::new();
        wal.checkpoint_interval = 5;

        for _ in 0..10 {
            wal.append(LogRecordType::Insert, 50);
        }

        assert!(
            wal.checkpoint_count > 0,
            "Should have at least one checkpoint"
        );
        assert!(wal.last_checkpoint_lsn > 0);
    }

    #[test]
    fn test_lsn_monotonic() {
        let mut wal = WALBlock::new();
        wal.checkpoint_interval = 1000; // Don't trigger checkpoint

        let mut prev_lsn = 0;
        for _ in 0..100 {
            let lsn = wal.append(LogRecordType::Insert, 10);
            assert!(lsn > prev_lsn, "LSN must be monotonically increasing");
            prev_lsn = lsn;
        }
    }

    #[test]
    fn test_metadata() {
        let wal = WALBlock::new();
        assert_eq!(wal.metadata().id, "wal");
        assert_eq!(wal.metadata().category, BlockCategory::Transaction);
        assert_eq!(wal.inputs().len(), 1);
        assert_eq!(wal.outputs().len(), 1);
        assert_eq!(wal.parameters().len(), 2);
    }

    #[tokio::test]
    async fn test_initialize_with_params() {
        let mut wal = WALBlock::new();
        let mut params = HashMap::new();
        params.insert("fsync_interval".into(), ParameterValue::Integer(5));
        params.insert("checkpoint_interval".into(), ParameterValue::Integer(50));

        wal.initialize(params).await.unwrap();
        assert_eq!(wal.fsync_interval, 5);
        assert_eq!(wal.checkpoint_interval, 50);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut wal = WALBlock::new();
        wal.fsync_interval = 5;

        let records: Vec<Record> = (0..20)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("data".into(), format!("value_{}", i)).unwrap();
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

        let result = wal.execute(ctx).await.unwrap();
        assert!(result.errors.is_empty());

        let logged = result.outputs.get("logged").unwrap();
        assert_eq!(logged.len(), 20);
        assert!(*result.metrics.get("log_entries").unwrap() > 20.0); // 20 inserts + 1 commit
        assert!(*result.metrics.get("fsyncs").unwrap() > 0.0);
    }
}
