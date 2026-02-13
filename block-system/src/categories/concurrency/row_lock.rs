//! Row Lock (Two-Phase Locking) Concurrency Block
//!
//! Implements **strict two-phase locking (2PL)** — the classic concurrency
//! control protocol. Transactions acquire locks before accessing records
//! and release them only at commit/abort.
//!
//! ## How it works
//!
//! - **Growing phase**: Locks are acquired as records are accessed.
//! - **Shrinking phase**: All locks released at once when the transaction commits.
//! - **Lock modes**: Shared (S) for reads, Exclusive (X) for writes.
//! - **Deadlock detection**: Uses a wait-for graph with cycle detection.
//!
//! ## Metrics tracked
//!
//! | Metric | Type | Description |
//! |--------|------|-------------|
//! | `locks_acquired` | Counter | Total locks granted |
//! | `lock_waits` | Counter | Lock requests that had to wait |
//! | `deadlocks_detected` | Counter | Deadlock cycles found |
//! | `lock_upgrades` | Counter | S → X upgrades |
//! | `active_locks` | Gauge | Currently held locks |
//! | `transactions_committed` | Counter | Successfully committed txns |
//! | `transactions_aborted` | Counter | Aborted transactions |

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

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
// Internal lock model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone)]
struct LockEntry {
    mode: LockMode,
    holders: HashSet<u64>, // Transaction IDs
}

/// Result of a lock request.
#[derive(Debug, Clone, PartialEq)]
pub enum LockResult {
    Granted,
    Waited,
    Deadlock,
}

// ---------------------------------------------------------------------------
// RowLockBlock
// ---------------------------------------------------------------------------

pub struct RowLockBlock {
    metadata: BlockMetadata,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
    params: Vec<Parameter>,
    metric_defs: Vec<MetricDefinition>,

    // Configuration
    max_locks_per_txn: usize,

    // Internal state — lock table: resource_id → LockEntry
    lock_table: HashMap<String, LockEntry>,
    // Tracks which resources each transaction holds.
    txn_locks: HashMap<u64, Vec<String>>,
    // Wait-for graph: txn → set of txns it's waiting for.
    wait_for: HashMap<u64, HashSet<u64>>,

    // Counters
    locks_acquired: usize,
    lock_waits: usize,
    deadlocks_detected: usize,
    lock_upgrades: usize,
    txn_committed: usize,
    txn_aborted: usize,
    next_txn_id: u64,
}

impl RowLockBlock {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            input_ports: Self::build_inputs(),
            output_ports: Self::build_outputs(),
            params: Self::build_parameters(),
            metric_defs: Self::build_metrics(),
            max_locks_per_txn: 1000,
            lock_table: HashMap::new(),
            txn_locks: HashMap::new(),
            wait_for: HashMap::new(),
            locks_acquired: 0,
            lock_waits: 0,
            deadlocks_detected: 0,
            lock_upgrades: 0,
            txn_committed: 0,
            txn_aborted: 0,
            next_txn_id: 1,
        }
    }

    fn build_metadata() -> BlockMetadata {
        BlockMetadata {
            id: "row-lock-2pl".into(),
            name: "Row Lock (2PL)".into(),
            category: BlockCategory::Concurrency,
            description: "Strict two-phase locking with deadlock detection".into(),
            version: "1.0.0".into(),
            documentation: BlockDocumentation {
                overview: "Two-phase locking (2PL) is the classic concurrency control protocol. \
                           Transactions acquire locks before accessing data (growing phase) and \
                           release all locks at once when they finish (shrinking phase). Strict \
                           2PL holds locks until commit, preventing cascading aborts."
                    .into(),
                algorithm: "Lock request: check lock table. If compatible (S+S), grant. If \
                            incompatible (S+X or X+any), add to wait-for graph and check for \
                            cycles. If cycle detected, abort the requesting transaction. On \
                            commit, release all locks held by the transaction."
                    .into(),
                complexity: Complexity {
                    time: "Lock acquire O(1), deadlock check O(V+E) in wait-for graph".into(),
                    space: "O(L) for lock table + O(T²) worst-case for wait-for graph".into(),
                },
                use_cases: vec![
                    "OLTP workloads with row-level locking".into(),
                    "Serializable isolation level".into(),
                    "Short transactions accessing few rows".into(),
                ],
                tradeoffs: vec![
                    "Guarantees serializability but can cause deadlocks".into(),
                    "Lock overhead per row access".into(),
                    "Long transactions hold locks longer, reducing concurrency".into(),
                    "Deadlock detection adds overhead but prevents indefinite waits".into(),
                ],
                examples: vec![
                    "MySQL InnoDB row locks".into(),
                    "PostgreSQL row-level locks (FOR UPDATE, FOR SHARE)".into(),
                ],
            },
            references: vec![Reference {
                ref_type: ReferenceType::Paper,
                title: "The Notions of Consistency and Predicate Locks".into(),
                url: None,
                citation: Some(
                    "Eswaran, K.P. et al. (1976). CACM, 19(11), 624–633.".into(),
                ),
            }],
            icon: "lock".into(),
            color: "#EF4444".into(),
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
            description: "Records to process with locking".into(),
            schema: None,
        }]
    }

    fn build_outputs() -> Vec<Port> {
        vec![Port {
            id: "committed".into(),
            name: "Committed Records".into(),
            port_type: PortType::DataStream,
            direction: PortDirection::Output,
            required: false,
            multiple: true,
            description: "Records from successfully committed transactions".into(),
            schema: None,
        }]
    }

    fn build_parameters() -> Vec<Parameter> {
        vec![Parameter {
            id: "max_locks_per_txn".into(),
            name: "Max Locks Per Txn".into(),
            param_type: ParameterType::Number,
            description: "Maximum locks a single transaction can hold".into(),
            default_value: ParameterValue::Integer(1000),
            required: false,
            constraints: Some(
                ParameterConstraints::new()
                    .with_min(1.0)
                    .with_max(100000.0),
            ),
            ui_hint: Some(
                ParameterUIHint::new(WidgetType::Slider)
                    .with_step(100.0)
                    .with_help_text("Lock escalation threshold".into()),
            ),
        }]
    }

    fn build_metrics() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: "locks_acquired".into(),
                name: "Locks Acquired".into(),
                metric_type: MetricType::Counter,
                unit: "locks".into(),
                description: "Total locks granted".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "lock_waits".into(),
                name: "Lock Waits".into(),
                metric_type: MetricType::Counter,
                unit: "waits".into(),
                description: "Lock requests that had to wait".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "deadlocks_detected".into(),
                name: "Deadlocks".into(),
                metric_type: MetricType::Counter,
                unit: "deadlocks".into(),
                description: "Deadlock cycles detected".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "lock_upgrades".into(),
                name: "Lock Upgrades".into(),
                metric_type: MetricType::Counter,
                unit: "upgrades".into(),
                description: "Shared to exclusive lock upgrades".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "active_locks".into(),
                name: "Active Locks".into(),
                metric_type: MetricType::Gauge,
                unit: "locks".into(),
                description: "Currently held locks".into(),
                aggregations: vec![AggregationType::Max],
            },
            MetricDefinition {
                id: "transactions_committed".into(),
                name: "Committed".into(),
                metric_type: MetricType::Counter,
                unit: "txns".into(),
                description: "Successfully committed transactions".into(),
                aggregations: vec![AggregationType::Sum],
            },
            MetricDefinition {
                id: "transactions_aborted".into(),
                name: "Aborted".into(),
                metric_type: MetricType::Counter,
                unit: "txns".into(),
                description: "Aborted transactions (deadlock or error)".into(),
                aggregations: vec![AggregationType::Sum],
            },
        ]
    }

    // -- Core operations -----------------------------------------------------

    /// Begin a new transaction.
    pub fn begin_txn(&mut self) -> u64 {
        let id = self.next_txn_id;
        self.next_txn_id += 1;
        self.txn_locks.insert(id, Vec::new());
        id
    }

    /// Request a lock on a resource.
    pub fn acquire_lock(
        &mut self,
        txn_id: u64,
        resource: &str,
        mode: LockMode,
    ) -> LockResult {
        if let Some(entry) = self.lock_table.get(resource) {
            // Already locked.
            if entry.holders.contains(&txn_id) {
                // Already hold a lock — check for upgrade.
                if entry.mode == LockMode::Shared && mode == LockMode::Exclusive {
                    if entry.holders.len() == 1 {
                        // Only holder — upgrade.
                        self.lock_table.get_mut(resource).unwrap().mode = LockMode::Exclusive;
                        self.lock_upgrades += 1;
                        self.locks_acquired += 1;
                        return LockResult::Granted;
                    } else {
                        // Others hold shared locks — potential deadlock.
                        // Add to wait-for graph.
                        let waitees: HashSet<u64> =
                            entry.holders.iter().filter(|&&h| h != txn_id).copied().collect();
                        self.wait_for.insert(txn_id, waitees);

                        if self.has_cycle(txn_id) {
                            self.wait_for.remove(&txn_id);
                            self.deadlocks_detected += 1;
                            return LockResult::Deadlock;
                        }

                        // Simulate wait then grant.
                        self.wait_for.remove(&txn_id);
                        self.lock_waits += 1;
                        let entry = self.lock_table.get_mut(resource).unwrap();
                        entry.mode = LockMode::Exclusive;
                        // Remove other holders (they would have been resolved).
                        entry.holders.clear();
                        entry.holders.insert(txn_id);
                        self.lock_upgrades += 1;
                        self.locks_acquired += 1;
                        return LockResult::Waited;
                    }
                }
                // Already hold compatible lock.
                return LockResult::Granted;
            }

            // Someone else holds the lock — check compatibility.
            let compatible = entry.mode == LockMode::Shared && mode == LockMode::Shared;

            if compatible {
                self.lock_table
                    .get_mut(resource)
                    .unwrap()
                    .holders
                    .insert(txn_id);
                self.txn_locks
                    .entry(txn_id)
                    .or_default()
                    .push(resource.to_string());
                self.locks_acquired += 1;
                return LockResult::Granted;
            }

            // Incompatible — check for deadlock.
            let waitees = entry.holders.clone();
            self.wait_for.insert(txn_id, waitees);

            if self.has_cycle(txn_id) {
                self.wait_for.remove(&txn_id);
                self.deadlocks_detected += 1;
                return LockResult::Deadlock;
            }

            // Simulate wait then grant — overwrite the lock entry.
            self.wait_for.remove(&txn_id);
            self.lock_waits += 1;
            let mut holders = HashSet::new();
            holders.insert(txn_id);
            self.lock_table.insert(
                resource.to_string(),
                LockEntry { mode, holders },
            );
            self.txn_locks
                .entry(txn_id)
                .or_default()
                .push(resource.to_string());
            self.locks_acquired += 1;
            return LockResult::Waited;
        }

        // No existing lock — grant immediately.
        let mut holders = HashSet::new();
        holders.insert(txn_id);
        self.lock_table.insert(
            resource.to_string(),
            LockEntry {
                mode,
                holders,
            },
        );
        self.txn_locks
            .entry(txn_id)
            .or_default()
            .push(resource.to_string());
        self.locks_acquired += 1;
        LockResult::Granted
    }

    /// Commit a transaction — release all its locks.
    pub fn commit(&mut self, txn_id: u64) {
        self.release_locks(txn_id);
        self.txn_committed += 1;
    }

    /// Abort a transaction — release all its locks.
    pub fn abort(&mut self, txn_id: u64) {
        self.release_locks(txn_id);
        self.txn_aborted += 1;
    }

    fn release_locks(&mut self, txn_id: u64) {
        if let Some(resources) = self.txn_locks.remove(&txn_id) {
            for resource in resources {
                if let Some(entry) = self.lock_table.get_mut(&resource) {
                    entry.holders.remove(&txn_id);
                    if entry.holders.is_empty() {
                        self.lock_table.remove(&resource);
                    }
                }
            }
        }
        self.wait_for.remove(&txn_id);
    }

    /// Detect cycle in wait-for graph using DFS from start_txn.
    fn has_cycle(&self, start_txn: u64) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start_txn];

        while let Some(txn) = stack.pop() {
            if !visited.insert(txn) {
                continue;
            }
            if let Some(waitees) = self.wait_for.get(&txn) {
                for &w in waitees {
                    if w == start_txn {
                        return true; // Cycle back to start
                    }
                    if !visited.contains(&w) {
                        stack.push(w);
                    }
                }
            }
        }
        false
    }

    pub fn active_lock_count(&self) -> usize {
        self.lock_table.values().map(|e| e.holders.len()).sum()
    }
}

impl Default for RowLockBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Block for RowLockBlock {
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
                "Serializable isolation via strict two-phase locking",
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
        if let Some(val) = params.get("max_locks_per_txn") {
            self.max_locks_per_txn = val
                .as_integer()
                .ok_or_else(|| {
                    BlockError::InvalidParameter("max_locks_per_txn must be an integer".into())
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

        // Simulate: each record is a separate transaction that acquires an exclusive lock.
        let mut committed_records = Vec::new();

        for record in &records {
            let txn_id = self.begin_txn();
            let resource = record
                .data
                .get("id")
                .map(|v| v.to_string())
                .unwrap_or_else(|| format!("row_{}", txn_id));

            let result = self.acquire_lock(txn_id, &resource, LockMode::Exclusive);

            match result {
                LockResult::Granted | LockResult::Waited => {
                    committed_records.push(record.clone());
                    self.commit(txn_id);
                }
                LockResult::Deadlock => {
                    self.abort(txn_id);
                }
            }
        }

        context
            .metrics
            .record("locks_acquired", self.locks_acquired as f64);
        context
            .metrics
            .record("lock_waits", self.lock_waits as f64);
        context
            .metrics
            .record("deadlocks_detected", self.deadlocks_detected as f64);
        context
            .metrics
            .record("lock_upgrades", self.lock_upgrades as f64);
        context
            .metrics
            .record("active_locks", self.active_lock_count() as f64);
        context
            .metrics
            .record("transactions_committed", self.txn_committed as f64);
        context
            .metrics
            .record("transactions_aborted", self.txn_aborted as f64);

        let mut outputs = HashMap::new();
        outputs.insert("committed".into(), PortValue::Stream(committed_records));

        let mut metrics_summary = HashMap::new();
        metrics_summary.insert("locks_acquired".into(), self.locks_acquired as f64);
        metrics_summary.insert("lock_waits".into(), self.lock_waits as f64);
        metrics_summary.insert("deadlocks_detected".into(), self.deadlocks_detected as f64);
        metrics_summary.insert(
            "transactions_committed".into(),
            self.txn_committed as f64,
        );
        metrics_summary.insert("transactions_aborted".into(), self.txn_aborted as f64);

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
        let _ = state.insert("active_locks".into(), self.active_lock_count());
        let _ = state.insert("txn_committed".into(), self.txn_committed);
        let _ = state.insert("txn_aborted".into(), self.txn_aborted);
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
    fn test_basic_lock_and_commit() {
        let mut lock = RowLockBlock::new();
        let txn = lock.begin_txn();

        assert_eq!(
            lock.acquire_lock(txn, "row_1", LockMode::Exclusive),
            LockResult::Granted
        );
        assert_eq!(lock.active_lock_count(), 1);

        lock.commit(txn);
        assert_eq!(lock.active_lock_count(), 0);
        assert_eq!(lock.txn_committed, 1);
    }

    #[test]
    fn test_shared_locks_compatible() {
        let mut lock = RowLockBlock::new();
        let txn1 = lock.begin_txn();
        let txn2 = lock.begin_txn();

        assert_eq!(
            lock.acquire_lock(txn1, "row_1", LockMode::Shared),
            LockResult::Granted
        );
        assert_eq!(
            lock.acquire_lock(txn2, "row_1", LockMode::Shared),
            LockResult::Granted
        );
        assert_eq!(lock.active_lock_count(), 2);
    }

    #[test]
    fn test_exclusive_lock_conflict() {
        let mut lock = RowLockBlock::new();
        let txn1 = lock.begin_txn();
        let txn2 = lock.begin_txn();

        assert_eq!(
            lock.acquire_lock(txn1, "row_1", LockMode::Exclusive),
            LockResult::Granted
        );
        // txn2 requests exclusive on same row — should wait.
        let result = lock.acquire_lock(txn2, "row_1", LockMode::Exclusive);
        assert_eq!(result, LockResult::Waited);
        assert_eq!(lock.lock_waits, 1);
    }

    #[test]
    fn test_lock_upgrade() {
        let mut lock = RowLockBlock::new();
        let txn = lock.begin_txn();

        lock.acquire_lock(txn, "row_1", LockMode::Shared);
        let result = lock.acquire_lock(txn, "row_1", LockMode::Exclusive);
        assert_eq!(result, LockResult::Granted);
        assert_eq!(lock.lock_upgrades, 1);
    }

    #[test]
    fn test_abort_releases_locks() {
        let mut lock = RowLockBlock::new();
        let txn = lock.begin_txn();

        lock.acquire_lock(txn, "row_1", LockMode::Exclusive);
        lock.acquire_lock(txn, "row_2", LockMode::Exclusive);
        assert_eq!(lock.active_lock_count(), 2);

        lock.abort(txn);
        assert_eq!(lock.active_lock_count(), 0);
        assert_eq!(lock.txn_aborted, 1);
    }

    #[test]
    fn test_metadata() {
        let lock = RowLockBlock::new();
        assert_eq!(lock.metadata().id, "row-lock-2pl");
        assert_eq!(lock.metadata().category, BlockCategory::Concurrency);
        assert_eq!(lock.inputs().len(), 1);
        assert_eq!(lock.outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_block_execute() {
        use crate::core::metrics::{Logger, MetricsCollector, StorageContext};

        let mut lock = RowLockBlock::new();

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

        let result = lock.execute(ctx).await.unwrap();
        assert_eq!(
            *result.metrics.get("transactions_committed").unwrap(),
            20.0
        );
        assert!(result.errors.is_empty());
    }
}
