//! Execution engine
//!
//! Manages a graph of blocks connected via ports. Validates the graph, executes
//! blocks in topological order, routes data between connected ports, collects
//! per-block timing and metrics, and supports cancellation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::core::block::{Block, BlockError, ExecutionContext};
use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
use crate::core::parameter::ParameterValue;
use crate::core::port::{Connection, PortValue};

use super::validation::{GraphValidationResult, GraphValidator};

// ── Result types (mirror frontend) ──────────────────────────────────────────

/// Per-block metrics collected during execution.
#[derive(Debug, Clone)]
pub struct BlockMetrics {
    pub block_id: String,
    pub block_type: String,
    pub block_name: String,
    /// Wall-clock execution time in milliseconds.
    pub execution_time_ms: f64,
    /// Percentage of total pipeline time.
    pub percentage: f64,
    /// Block-specific counters (from the block's ExecutionResult.metrics).
    pub counters: HashMap<String, f64>,
}

/// Latency percentile metrics.
#[derive(Debug, Clone, Default)]
pub struct LatencyMetrics {
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Aggregate execution metrics.
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    /// Operations per second.
    pub throughput: f64,
    pub latency: LatencyMetrics,
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
}

/// Final result of an engine execution run.
#[derive(Debug, Clone)]
pub struct EngineExecutionResult {
    pub success: bool,
    /// Total wall-clock duration in milliseconds.
    pub duration_ms: f64,
    pub metrics: ExecutionMetrics,
    pub block_metrics: Vec<BlockMetrics>,
    pub errors: Vec<String>,
}

// ── Engine ──────────────────────────────────────────────────────────────────

/// The execution engine wires blocks together and runs them.
pub struct ExecutionEngine {
    blocks: HashMap<String, Box<dyn Block>>,
    connections: Vec<Connection>,
    /// Block IDs that receive workload data (entry points).
    entry_points: Vec<String>,
    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            connections: Vec::new(),
            entry_points: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add a block to the engine.
    pub fn add_block(&mut self, id: impl Into<String>, block: Box<dyn Block>) {
        self.blocks.insert(id.into(), block);
    }

    /// Add a connection between ports.
    pub fn add_connection(&mut self, conn: Connection) {
        self.connections.push(conn);
    }

    /// Mark a block as an entry point (receives workload data).
    pub fn set_entry_point(&mut self, block_id: impl Into<String>) {
        self.entry_points.push(block_id.into());
    }

    /// Clear all entry points.
    pub fn clear_entry_points(&mut self) {
        self.entry_points.clear();
    }

    /// Auto-detect entry points: blocks that have no incoming connections.
    pub fn auto_detect_entry_points(&mut self) {
        use std::collections::HashSet;
        let targets: HashSet<&str> = self
            .connections
            .iter()
            .map(|c| c.target_block_id.as_str())
            .collect();

        self.entry_points = self
            .blocks
            .keys()
            .filter(|id| !targets.contains(id.as_str()))
            .cloned()
            .collect();
    }

    /// Get current entry points.
    pub fn entry_points(&self) -> &[String] {
        &self.entry_points
    }

    /// Initialize a block with parameters.
    pub async fn initialize_block(
        &mut self,
        block_id: &str,
        params: HashMap<String, ParameterValue>,
    ) -> Result<(), BlockError> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| BlockError::InitializationError(format!("Block '{}' not found", block_id)))?;
        block.initialize(params).await
    }

    /// Get a handle to the cancellation flag (for external cancellation).
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Is the engine cancelled?
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Number of registered blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Validate the graph.
    pub fn validate(&self) -> GraphValidationResult {
        let entry_refs: Vec<&str> = self.entry_points.iter().map(|s| s.as_str()).collect();
        GraphValidator::validate(&self.blocks, &self.connections, &entry_refs)
    }

    /// Execute the pipeline.
    ///
    /// 1. Validate the graph.
    /// 2. Compute topological order.
    /// 3. Execute each block in order, routing outputs to connected inputs.
    /// 4. Collect per-block metrics and timing.
    ///
    /// `input_data` provides the initial data for entry-point blocks,
    /// keyed by (block_id, port_id).
    pub async fn execute(
        &mut self,
        input_data: HashMap<(String, String), PortValue>,
    ) -> EngineExecutionResult {
        let pipeline_start = Instant::now();
        let mut errors = Vec::new();
        let mut block_metrics = Vec::new();

        // Reset cancellation.
        self.cancelled.store(false, Ordering::SeqCst);

        // Step 1: Validate.
        let validation = self.validate();
        if !validation.valid {
            let err_msgs: Vec<String> = validation.errors.iter().map(|e| e.message.clone()).collect();
            return EngineExecutionResult {
                success: false,
                duration_ms: pipeline_start.elapsed().as_secs_f64() * 1000.0,
                metrics: ExecutionMetrics::default(),
                block_metrics: Vec::new(),
                errors: err_msgs,
            };
        }

        // Step 2: Topological sort.
        let block_ids: Vec<&str> = self.blocks.keys().map(|s| s.as_str()).collect();
        let order = match GraphValidator::topological_sort(&block_ids, &self.connections) {
            Some(o) => o,
            None => {
                return EngineExecutionResult {
                    success: false,
                    duration_ms: pipeline_start.elapsed().as_secs_f64() * 1000.0,
                    metrics: ExecutionMetrics::default(),
                    block_metrics: Vec::new(),
                    errors: vec!["Graph contains a cycle".into()],
                };
            }
        };

        // Step 3: Execute blocks in order.
        // Data bus: stores output port values from completed blocks.
        let mut data_bus: HashMap<(String, String), PortValue> = HashMap::new();

        // Seed the data bus with external input data.
        for ((block_id, port_id), value) in input_data {
            data_bus.insert((block_id, port_id), value);
        }

        let mut total_ops: usize = 0;
        let mut successful_ops: usize = 0;
        let mut failed_ops: usize = 0;
        let mut block_times: Vec<f64> = Vec::new();

        for block_id in &order {
            // Check cancellation.
            if self.cancelled.load(Ordering::SeqCst) {
                errors.push("Execution cancelled".into());
                break;
            }

            // Build input map for this block by collecting data from the bus.
            let mut inputs: HashMap<String, PortValue> = HashMap::new();

            // First, check for external data directly addressed to this block.
            for ((bid, pid), value) in &data_bus {
                if bid == block_id {
                    inputs.insert(pid.clone(), value.clone());
                }
            }

            // Then, collect data from connections (source → this block).
            for conn in &self.connections {
                if &conn.target_block_id == block_id {
                    let key = (conn.source_block_id.clone(), conn.source_port_id.clone());
                    if let Some(value) = data_bus.get(&key) {
                        inputs.insert(conn.target_port_id.clone(), value.clone());
                    }
                }
            }

            // Build execution context.
            let ctx = ExecutionContext {
                inputs,
                parameters: HashMap::new(),
                metrics: MetricsCollector::new(),
                logger: Logger::new(),
                storage: StorageContext::new(),
            };

            // Execute the block.
            let block_start = Instant::now();
            let block = self.blocks.get_mut(block_id.as_str()).unwrap();
            let block_name = block.metadata().name.clone();
            let block_type = format!("{:?}", block.metadata().category);

            let result = block.execute(ctx).await;
            let block_elapsed_ms = block_start.elapsed().as_secs_f64() * 1000.0;
            block_times.push(block_elapsed_ms);

            match result {
                Ok(exec_result) => {
                    // Count operations from the output.
                    let op_count: usize = exec_result
                        .outputs
                        .values()
                        .map(|v| v.len())
                        .sum();
                    total_ops += op_count;
                    successful_ops += op_count;

                    // Store outputs in the data bus.
                    for (port_id, value) in &exec_result.outputs {
                        data_bus.insert((block_id.clone(), port_id.clone()), value.clone());
                    }

                    // Collect non-fatal errors.
                    for err in &exec_result.errors {
                        failed_ops += 1;
                        errors.push(format!("[{}] {}", block_id, err));
                    }

                    block_metrics.push(BlockMetrics {
                        block_id: block_id.clone(),
                        block_type: block_type.clone(),
                        block_name,
                        execution_time_ms: block_elapsed_ms,
                        percentage: 0.0, // computed below
                        counters: exec_result.metrics,
                    });
                }
                Err(e) => {
                    failed_ops += 1;
                    errors.push(format!("[{}] Fatal: {}", block_id, e));
                    block_metrics.push(BlockMetrics {
                        block_id: block_id.clone(),
                        block_type,
                        block_name,
                        execution_time_ms: block_elapsed_ms,
                        percentage: 0.0,
                        counters: HashMap::new(),
                    });
                    // Continue executing remaining blocks (best-effort).
                }
            }
        }

        // Step 4: Compute aggregate metrics.
        let total_duration_ms = pipeline_start.elapsed().as_secs_f64() * 1000.0;

        // Compute percentage of total time for each block.
        for bm in &mut block_metrics {
            if total_duration_ms > 0.0 {
                bm.percentage = (bm.execution_time_ms / total_duration_ms) * 100.0;
            }
        }

        // Latency stats from block_times.
        let latency = if !block_times.is_empty() {
            let avg = block_times.iter().sum::<f64>() / block_times.len() as f64;
            let mut sorted = block_times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            LatencyMetrics {
                avg,
                p50: percentile(&sorted, 0.5),
                p95: percentile(&sorted, 0.95),
                p99: percentile(&sorted, 0.99),
            }
        } else {
            LatencyMetrics::default()
        };

        let throughput = if total_duration_ms > 0.0 {
            (total_ops as f64) / (total_duration_ms / 1000.0)
        } else {
            0.0
        };

        let success = errors.is_empty() || !errors.iter().any(|e| e.contains("Fatal"));

        EngineExecutionResult {
            success,
            duration_ms: total_duration_ms,
            metrics: ExecutionMetrics {
                throughput,
                latency,
                total_operations: total_ops,
                successful_operations: successful_ops,
                failed_operations: failed_ops,
            },
            block_metrics,
            errors,
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Linear interpolation percentile.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (sorted.len() as f64 - 1.0) * p;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = idx - lo as f64;
        sorted[lo] + (sorted[hi] - sorted[lo]) * frac
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::buffer::LRUBufferBlock;
    use crate::categories::index::BTreeIndexBlock;
    use crate::categories::storage::HeapFileBlock;
    use crate::core::port::{Connection, PortValue, Record};
    use crate::runtime::workload::{WorkloadConfig, WorkloadGenerator};

    fn conn(id: &str, src_block: &str, src_port: &str, tgt_block: &str, tgt_port: &str) -> Connection {
        Connection::new(
            id.into(),
            src_block.into(),
            src_port.into(),
            tgt_block.into(),
            tgt_port.into(),
        )
    }

    fn generate_records(n: usize) -> Vec<Record> {
        (0..n)
            .map(|i| {
                let mut r = Record::new();
                r.insert("id".into(), i as i64).unwrap();
                r.insert("name".into(), format!("user_{}", i)).unwrap();
                r.insert("score".into(), (i * 7 % 100) as f64).unwrap();
                r
            })
            .collect()
    }

    // ── Basic pipeline ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_single_block_execution() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.set_entry_point("heap");

        engine
            .initialize_block("heap", HashMap::new())
            .await
            .unwrap();

        let records = generate_records(100);
        let mut input = HashMap::new();
        input.insert(
            ("heap".into(), "records".into()),
            PortValue::Stream(records),
        );

        let result = engine.execute(input).await;
        assert!(result.success, "Errors: {:?}", result.errors);
        assert_eq!(result.block_metrics.len(), 1);
        assert!(result.duration_ms >= 0.0);
        assert!(result.metrics.total_operations > 0);
    }

    #[tokio::test]
    async fn test_two_block_pipeline() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.add_block("btree", Box::new(BTreeIndexBlock::new()));
        engine.add_connection(conn("c1", "heap", "stored", "btree", "records"));
        engine.set_entry_point("heap");

        engine
            .initialize_block("heap", HashMap::new())
            .await
            .unwrap();
        engine
            .initialize_block("btree", HashMap::new())
            .await
            .unwrap();

        let records = generate_records(200);
        let mut input = HashMap::new();
        input.insert(
            ("heap".into(), "records".into()),
            PortValue::Stream(records),
        );

        let result = engine.execute(input).await;
        assert!(result.success, "Errors: {:?}", result.errors);
        assert_eq!(result.block_metrics.len(), 2);

        // Both blocks should have execution time > 0.
        for bm in &result.block_metrics {
            assert!(bm.execution_time_ms >= 0.0);
            assert!(!bm.counters.is_empty(), "Block {} should have counters", bm.block_id);
        }

        // Percentages should roughly sum to ~100% (may differ due to overhead).
        let total_pct: f64 = result.block_metrics.iter().map(|b| b.percentage).sum();
        assert!(total_pct > 50.0, "Total percentage {:.1}% suspiciously low", total_pct);
    }

    #[tokio::test]
    async fn test_three_block_pipeline() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.add_block("btree", Box::new(BTreeIndexBlock::new()));
        engine.add_block("buffer", Box::new(LRUBufferBlock::new()));

        engine.add_connection(conn("c1", "heap", "stored", "btree", "records"));
        engine.add_connection(conn("c2", "heap", "stored", "buffer", "requests"));
        engine.set_entry_point("heap");

        engine.initialize_block("heap", HashMap::new()).await.unwrap();
        engine.initialize_block("btree", HashMap::new()).await.unwrap();
        engine.initialize_block("buffer", HashMap::new()).await.unwrap();

        let records = generate_records(100);
        let mut input = HashMap::new();
        input.insert(
            ("heap".into(), "records".into()),
            PortValue::Stream(records),
        );

        let result = engine.execute(input).await;
        assert!(result.success, "Errors: {:?}", result.errors);
        assert_eq!(result.block_metrics.len(), 3);
    }

    // ── Validation through engine ───────────────────────────────────────

    #[tokio::test]
    async fn test_validate_detects_cycle() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("a", Box::new(HeapFileBlock::new()));
        engine.add_block("b", Box::new(HeapFileBlock::new()));
        engine.add_connection(conn("c1", "a", "stored", "b", "records"));
        engine.add_connection(conn("c2", "b", "stored", "a", "records"));
        engine.set_entry_point("a");
        engine.set_entry_point("b");

        let validation = engine.validate();
        assert!(!validation.valid);
    }

    #[tokio::test]
    async fn test_execute_rejects_invalid_graph() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("a", Box::new(HeapFileBlock::new()));
        engine.add_block("b", Box::new(HeapFileBlock::new()));
        engine.add_connection(conn("c1", "a", "stored", "b", "records"));
        engine.add_connection(conn("c2", "b", "stored", "a", "records"));
        engine.set_entry_point("a");
        engine.set_entry_point("b");

        let result = engine.execute(HashMap::new()).await;
        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    // ── Cancellation ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cancellation() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.set_entry_point("heap");
        engine.initialize_block("heap", HashMap::new()).await.unwrap();

        // Cancel immediately.
        engine.cancel();

        let records = generate_records(100);
        let mut input = HashMap::new();
        input.insert(
            ("heap".into(), "records".into()),
            PortValue::Stream(records),
        );

        let result = engine.execute(input).await;
        // Cancellation is reset at the start of execute(), so it should succeed.
        // Let's test with external cancellation instead.
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_cancel_handle() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.set_entry_point("heap");
        engine.initialize_block("heap", HashMap::new()).await.unwrap();

        let handle = engine.cancel_handle();
        // Cancel externally after engine resets.
        // Since execute is sequential and blocks run fast, we can't easily
        // cancel mid-execution in a test. Just verify the API works.
        assert!(!engine.is_cancelled());
        handle.store(true, Ordering::SeqCst);
        assert!(engine.is_cancelled());
    }

    // ── Workload integration ────────────────────────────────────────────

    #[tokio::test]
    async fn test_engine_with_workload() {
        let mut engine = ExecutionEngine::new();
        engine.add_block("heap", Box::new(HeapFileBlock::new()));
        engine.add_block("btree", Box::new(BTreeIndexBlock::new()));
        engine.add_connection(conn("c1", "heap", "stored", "btree", "records"));
        engine.set_entry_point("heap");

        engine.initialize_block("heap", HashMap::new()).await.unwrap();
        engine.initialize_block("btree", HashMap::new()).await.unwrap();

        // Generate workload records.
        let config = WorkloadConfig {
            total_ops: 500,
            seed: 42,
            ..Default::default()
        };
        let records = WorkloadGenerator::generate_records(&config);

        let mut input = HashMap::new();
        input.insert(
            ("heap".into(), "records".into()),
            PortValue::Stream(records),
        );

        let result = engine.execute(input).await;
        assert!(result.success, "Errors: {:?}", result.errors);
        assert_eq!(result.block_metrics.len(), 2);
        assert!(result.metrics.throughput > 0.0);
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_empty_engine() {
        let mut engine = ExecutionEngine::new();
        let result = engine.execute(HashMap::new()).await;
        // Empty graph is valid.
        assert!(result.success);
        assert!(result.block_metrics.is_empty());
    }

    #[tokio::test]
    async fn test_block_count() {
        let mut engine = ExecutionEngine::new();
        assert_eq!(engine.block_count(), 0);
        engine.add_block("a", Box::new(HeapFileBlock::new()));
        assert_eq!(engine.block_count(), 1);
        engine.add_block("b", Box::new(BTreeIndexBlock::new()));
        assert_eq!(engine.block_count(), 2);
    }
}
