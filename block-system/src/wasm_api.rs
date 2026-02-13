//! WASM API — `#[wasm_bindgen]` exports matching the frontend's `WASMModule` interface.
//!
//! This module is only compiled when targeting `wasm32`. It provides:
//! - `init_runtime` / `destroy_runtime` — lifecycle
//! - `register_block` / `create_connection` — graph construction
//! - `validate` — graph validation
//! - `execute` — run workload with progress reporting
//! - `cancel_execution` — cooperative cancellation
//! - `get_metrics` / `get_block_types` — discovery and results

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::categories::buffer::LRUBufferBlock;
use crate::categories::concurrency::{MVCCBlock, RowLockBlock};
use crate::categories::execution::{
    FilterBlock, HashJoinBlock, IndexScanBlock, SequentialScanBlock, SortBlock,
};
use crate::categories::index::{BTreeIndexBlock, CoveringIndexBlock, HashIndexBlock};
use crate::categories::storage::{
    ClusteredStorageBlock, ColumnarStorageBlock, HeapFileBlock, LSMTreeBlock,
};
use crate::categories::transaction::WALBlock;
use crate::core::block::Block;
use crate::core::parameter::ParameterValue;
use crate::core::port::{Connection, PortValue};
use crate::runtime::engine::{EngineExecutionResult, ExecutionEngine};
use crate::runtime::workload::{
    Distribution, OperationConfig, OperationType, WorkloadConfig, WorkloadGenerator,
};

// ── Trivial async executor for WASM ─────────────────────────────────────────
//
// Our async Block trait doesn't do real I/O — every future completes
// immediately. A noop-waker executor drives them to completion.

fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    let mut fut = core::pin::pin!(fut);
    let waker = noop_waker();
    let mut cx = core::task::Context::from_waker(&waker);
    match fut.as_mut().poll(&mut cx) {
        core::task::Poll::Ready(result) => result,
        core::task::Poll::Pending => panic!("async block yielded Pending in WASM context"),
    }
}

fn noop_waker() -> core::task::Waker {
    use std::task::{RawWaker, RawWakerVTable};

    fn no_op(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);

    unsafe { core::task::Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

// ── Global state ────────────────────────────────────────────────────────────

struct WasmRuntime {
    engine: ExecutionEngine,
    last_result: Option<EngineExecutionResult>,
}

thread_local! {
    static RUNTIME: RefCell<Option<WasmRuntime>> = RefCell::new(None);
}

fn with_runtime<R>(f: impl FnOnce(&mut WasmRuntime) -> R) -> Result<R, String> {
    RUNTIME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        match borrow.as_mut() {
            Some(rt) => Ok(f(rt)),
            None => Err("Runtime not initialized. Call init_runtime() first.".into()),
        }
    })
}

// ── JSON interchange types ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct BlockConfigJson {
    #[serde(rename = "type")]
    block_type: String,
    id: String,
    #[serde(default)]
    parameters: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ConnectionJson {
    source_block_id: String,
    source_port_id: String,
    target_block_id: String,
    target_port_id: String,
    #[serde(default)]
    backpressure: bool,
    #[serde(default)]
    buffer_size: Option<usize>,
}

#[derive(Deserialize)]
struct WorkloadJson {
    #[serde(default)]
    operations: Vec<OperationJson>,
    #[serde(default = "default_distribution")]
    distribution: String,
    #[serde(default = "default_total_ops", rename = "totalOps")]
    total_ops: usize,
    #[serde(default)]
    concurrency: usize,
}

fn default_distribution() -> String {
    "uniform".into()
}
fn default_total_ops() -> usize {
    1000
}

#[derive(Deserialize)]
struct OperationJson {
    #[serde(rename = "type")]
    op_type: String,
    weight: u32,
    #[serde(default)]
    template: String,
}

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OkResponse {
    id: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct ValidationResponse {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ExecutionResponse {
    success: bool,
    duration: f64,
    metrics: MetricsResponse,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

#[derive(Serialize)]
struct MetricsResponse {
    throughput: f64,
    latency: LatencyResponse,
    #[serde(rename = "totalOperations")]
    total_operations: usize,
    #[serde(rename = "successfulOperations")]
    successful_operations: usize,
    #[serde(rename = "failedOperations")]
    failed_operations: usize,
    #[serde(rename = "blockMetrics")]
    block_metrics: Vec<BlockMetricsResponse>,
}

#[derive(Serialize)]
struct LatencyResponse {
    avg: f64,
    p50: f64,
    p95: f64,
    p99: f64,
}

#[derive(Serialize)]
struct BlockMetricsResponse {
    #[serde(rename = "blockId")]
    block_id: String,
    #[serde(rename = "blockType")]
    block_type: String,
    #[serde(rename = "blockName")]
    block_name: String,
    #[serde(rename = "executionTime")]
    execution_time: f64,
    percentage: f64,
    counters: HashMap<String, f64>,
}

#[derive(Serialize)]
struct ProgressResponse {
    progress: f64,
    phase: String,
    #[serde(rename = "currentBlockId")]
    current_block_id: Option<String>,
    message: String,
}

#[derive(Serialize)]
struct BlockTypeInfo {
    #[serde(rename = "type")]
    block_type: String,
    name: String,
    category: String,
    description: String,
}

// ── Block factory ───────────────────────────────────────────────────────────

fn create_block(block_type: &str) -> Result<Box<dyn Block>, String> {
    match block_type {
        "heap_storage" | "heap_file" => Ok(Box::new(HeapFileBlock::new())),
        "lsm_tree" | "lsm_storage" => Ok(Box::new(LSMTreeBlock::new())),
        "clustered_storage" | "clustered" => Ok(Box::new(ClusteredStorageBlock::new())),
        "columnar_storage" | "columnar" => Ok(Box::new(ColumnarStorageBlock::new())),
        "btree_index" | "b_tree_index" => Ok(Box::new(BTreeIndexBlock::new())),
        "hash_index" => Ok(Box::new(HashIndexBlock::new())),
        "covering_index" => Ok(Box::new(CoveringIndexBlock::new())),
        "lru_buffer" | "lru_cache" => Ok(Box::new(LRUBufferBlock::new())),
        "sequential_scan" | "seq_scan" => Ok(Box::new(SequentialScanBlock::new())),
        "index_scan" => Ok(Box::new(IndexScanBlock::new())),
        "filter" => Ok(Box::new(FilterBlock::new())),
        "sort" => Ok(Box::new(SortBlock::new())),
        "hash_join" => Ok(Box::new(HashJoinBlock::new())),
        "row_lock" | "row_lock_2pl" => Ok(Box::new(RowLockBlock::new())),
        "mvcc" => Ok(Box::new(MVCCBlock::new())),
        "wal" | "write_ahead_log" => Ok(Box::new(WALBlock::new())),
        _ => Err(format!(
            "Unknown block type: '{}'. Available: heap_storage, lsm_tree, clustered_storage, \
             columnar_storage, btree_index, hash_index, covering_index, lru_buffer, \
             sequential_scan, index_scan, filter, sort, hash_join, row_lock, mvcc, wal",
            block_type
        )),
    }
}

fn convert_parameters(raw: &HashMap<String, serde_json::Value>) -> HashMap<String, ParameterValue> {
    raw.iter()
        .filter_map(|(k, v)| {
            let pv = match v {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Some(ParameterValue::Integer(i))
                    } else {
                        n.as_f64().map(ParameterValue::Number)
                    }
                }
                serde_json::Value::String(s) => Some(ParameterValue::String(s.clone())),
                serde_json::Value::Bool(b) => Some(ParameterValue::Boolean(*b)),
                _ => None,
            };
            pv.map(|val| (k.clone(), val))
        })
        .collect()
}

fn parse_distribution(s: &str) -> Distribution {
    match s.to_lowercase().as_str() {
        "zipfian" | "zipf" => Distribution::Zipfian,
        "latest" | "recent" => Distribution::Latest,
        _ => Distribution::Uniform,
    }
}

fn parse_op_type(s: &str) -> OperationType {
    match s.to_uppercase().as_str() {
        "SELECT" | "READ" | "GET" => OperationType::Select,
        "UPDATE" | "MODIFY" => OperationType::Update,
        "DELETE" | "REMOVE" => OperationType::Delete,
        _ => OperationType::Insert,
    }
}

fn json_ok(id: &str) -> String {
    serde_json::to_string(&OkResponse { id: id.into() }).unwrap_or_default()
}

fn json_err(msg: impl Into<String>) -> String {
    serde_json::to_string(&ErrorResponse {
        error: msg.into(),
    })
    .unwrap_or_default()
}

// ── Exported functions ──────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn init_runtime() {
    console_error_panic_hook::set_once();

    RUNTIME.with(|cell| {
        *cell.borrow_mut() = Some(WasmRuntime {
            engine: ExecutionEngine::new(),
            last_result: None,
        });
    });
}

#[wasm_bindgen]
pub fn destroy_runtime() {
    RUNTIME.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[wasm_bindgen]
pub fn register_block(config_json: &str) -> String {
    let config: BlockConfigJson = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => return json_err(format!("Invalid config JSON: {}", e)),
    };

    let block = match create_block(&config.block_type) {
        Ok(b) => b,
        Err(e) => return json_err(e),
    };

    let id = config.id.clone();
    let params = convert_parameters(&config.parameters);

    match with_runtime(|rt| {
        rt.engine.add_block(&id, block);
        if !params.is_empty() {
            if let Err(e) = block_on(rt.engine.initialize_block(&id, params)) {
                return Err(format!("{}", e));
            }
        }
        Ok(())
    }) {
        Ok(Ok(())) => json_ok(&id),
        Ok(Err(e)) => json_err(e),
        Err(e) => json_err(e),
    }
}

#[wasm_bindgen]
pub fn create_connection(conn_json: &str) -> String {
    let cj: ConnectionJson = match serde_json::from_str(conn_json) {
        Ok(c) => c,
        Err(e) => return json_err(format!("Invalid connection JSON: {}", e)),
    };

    let conn_id = format!(
        "{}_{}_{}_{}",
        cj.source_block_id, cj.source_port_id, cj.target_block_id, cj.target_port_id
    );

    let conn = Connection {
        id: conn_id.clone(),
        source_block_id: cj.source_block_id,
        source_port_id: cj.source_port_id,
        target_block_id: cj.target_block_id,
        target_port_id: cj.target_port_id,
        backpressure: cj.backpressure,
        buffer_size: cj.buffer_size,
    };

    match with_runtime(|rt| {
        rt.engine.add_connection(conn);
    }) {
        Ok(()) => json_ok(&conn_id),
        Err(e) => json_err(e),
    }
}

#[wasm_bindgen]
pub fn validate() -> String {
    match with_runtime(|rt| {
        rt.engine.auto_detect_entry_points();
        rt.engine.validate()
    }) {
        Ok(v) => serde_json::to_string(&ValidationResponse {
            valid: v.valid,
            errors: v.errors.iter().map(|e| e.message.clone()).collect(),
            warnings: v.warnings.iter().map(|w| w.message.clone()).collect(),
        })
        .unwrap_or_default(),
        Err(e) => serde_json::to_string(&ValidationResponse {
            valid: false,
            errors: vec![e],
            warnings: vec![],
        })
        .unwrap_or_default(),
    }
}

#[wasm_bindgen]
pub fn execute(workload_json: &str, progress_callback: &js_sys::Function) -> String {
    let wj: WorkloadJson = match serde_json::from_str(workload_json) {
        Ok(w) => w,
        Err(e) => {
            return serde_json::to_string(&ExecutionResponse {
                success: false,
                duration: 0.0,
                metrics: empty_metrics(),
                errors: vec![format!("Invalid workload JSON: {}", e)],
            })
            .unwrap_or_default();
        }
    };

    report_progress(progress_callback, 0.0, "validating", None, "Validating graph...");

    match with_runtime(|rt| {
        rt.engine.auto_detect_entry_points();

        // Build workload config.
        let config = WorkloadConfig {
            operations: wj
                .operations
                .iter()
                .map(|o| OperationConfig {
                    op_type: parse_op_type(&o.op_type),
                    weight: o.weight,
                })
                .collect(),
            distribution: parse_distribution(&wj.distribution),
            total_ops: wj.total_ops,
            seed: 0,
        };

        let records = WorkloadGenerator::generate_records(&config);

        report_progress(progress_callback, 10.0, "executing", None, "Starting execution...");

        // Feed records to all entry-point blocks.
        let entry_points = rt.engine.entry_points().to_vec();
        let mut input = HashMap::new();
        for ep in &entry_points {
            input.insert(
                (ep.clone(), "records".into()),
                PortValue::Stream(records.clone()),
            );
        }

        report_progress(progress_callback, 20.0, "executing", None, "Running blocks...");

        let exec_result = block_on(rt.engine.execute(input));

        report_progress(
            progress_callback,
            90.0,
            "aggregating",
            None,
            "Aggregating metrics...",
        );

        rt.last_result = Some(exec_result.clone());
        exec_result
    }) {
        Ok(exec) => {
            report_progress(progress_callback, 100.0, "aggregating", None, "Complete");
            serde_json::to_string(&build_execution_response(&exec)).unwrap_or_default()
        }
        Err(e) => serde_json::to_string(&ExecutionResponse {
            success: false,
            duration: 0.0,
            metrics: empty_metrics(),
            errors: vec![e],
        })
        .unwrap_or_default(),
    }
}

#[wasm_bindgen]
pub fn cancel_execution() {
    let _ = with_runtime(|rt| {
        rt.engine.cancel();
    });
}

#[wasm_bindgen]
pub fn get_metrics() -> String {
    match with_runtime(|rt| rt.last_result.clone()) {
        Ok(Some(exec)) => {
            serde_json::to_string(&build_execution_response(&exec).metrics).unwrap_or_default()
        }
        _ => serde_json::to_string(&empty_metrics()).unwrap_or_default(),
    }
}

#[wasm_bindgen]
pub fn get_block_types() -> String {
    let types = vec![
        // Storage
        BlockTypeInfo {
            block_type: "heap_storage".into(),
            name: "Heap File Storage".into(),
            category: "Storage".into(),
            description: "Page-based heap file with insert, get, scan, delete operations".into(),
        },
        BlockTypeInfo {
            block_type: "lsm_tree".into(),
            name: "LSM Tree".into(),
            category: "Storage".into(),
            description: "Log-Structured Merge-Tree with memtable, SSTables, and compaction".into(),
        },
        BlockTypeInfo {
            block_type: "clustered_storage".into(),
            name: "Clustered Storage".into(),
            category: "Storage".into(),
            description: "Records physically ordered by cluster key for fast range scans".into(),
        },
        BlockTypeInfo {
            block_type: "columnar_storage".into(),
            name: "Columnar Storage".into(),
            category: "Storage".into(),
            description: "Column-oriented storage for analytical workloads with projection pushdown".into(),
        },
        // Index
        BlockTypeInfo {
            block_type: "btree_index".into(),
            name: "B-Tree Index".into(),
            category: "Index".into(),
            description:
                "B-tree index with configurable fanout, point lookups, and range scans".into(),
        },
        BlockTypeInfo {
            block_type: "hash_index".into(),
            name: "Hash Index".into(),
            category: "Index".into(),
            description: "Hash-based index for O(1) point lookups with bucket chaining".into(),
        },
        BlockTypeInfo {
            block_type: "covering_index".into(),
            name: "Covering Index".into(),
            category: "Index".into(),
            description: "Index with included columns for index-only scans without table lookups".into(),
        },
        // Buffer
        BlockTypeInfo {
            block_type: "lru_buffer".into(),
            name: "LRU Buffer Pool".into(),
            category: "Buffer".into(),
            description: "Least Recently Used page cache with configurable pool size".into(),
        },
        // Execution
        BlockTypeInfo {
            block_type: "sequential_scan".into(),
            name: "Sequential Scan".into(),
            category: "Execution".into(),
            description: "Full table scan that reads every page sequentially".into(),
        },
        BlockTypeInfo {
            block_type: "index_scan".into(),
            name: "Index Scan".into(),
            category: "Execution".into(),
            description: "Uses an index to fetch only matching records from storage".into(),
        },
        BlockTypeInfo {
            block_type: "filter".into(),
            name: "Filter".into(),
            category: "Execution".into(),
            description: "Predicate filter with configurable column, operator, and value".into(),
        },
        BlockTypeInfo {
            block_type: "sort".into(),
            name: "Sort".into(),
            category: "Execution".into(),
            description: "Sort records by column with in-memory or external merge sort".into(),
        },
        BlockTypeInfo {
            block_type: "hash_join".into(),
            name: "Hash Join".into(),
            category: "Execution".into(),
            description: "Build-probe hash join for equi-join queries".into(),
        },
        // Concurrency
        BlockTypeInfo {
            block_type: "row_lock".into(),
            name: "Row Lock (2PL)".into(),
            category: "Concurrency".into(),
            description: "Strict two-phase locking with deadlock detection".into(),
        },
        BlockTypeInfo {
            block_type: "mvcc".into(),
            name: "MVCC".into(),
            category: "Concurrency".into(),
            description: "Multi-Version Concurrency Control with snapshot isolation".into(),
        },
        // Transaction
        BlockTypeInfo {
            block_type: "wal".into(),
            name: "Write-Ahead Log".into(),
            category: "Transaction".into(),
            description: "Append-only log for crash recovery and transaction durability".into(),
        },
    ];

    serde_json::to_string(&types).unwrap_or_default()
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn report_progress(
    callback: &js_sys::Function,
    progress: f64,
    phase: &str,
    current_block: Option<&str>,
    message: &str,
) {
    let report = ProgressResponse {
        progress,
        phase: phase.into(),
        current_block_id: current_block.map(|s| s.into()),
        message: message.into(),
    };
    if let Ok(json) = serde_json::to_string(&report) {
        let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&json));
    }
}

fn build_execution_response(exec: &EngineExecutionResult) -> ExecutionResponse {
    ExecutionResponse {
        success: exec.success,
        duration: exec.duration_ms,
        metrics: MetricsResponse {
            throughput: exec.metrics.throughput,
            latency: LatencyResponse {
                avg: exec.metrics.latency.avg,
                p50: exec.metrics.latency.p50,
                p95: exec.metrics.latency.p95,
                p99: exec.metrics.latency.p99,
            },
            total_operations: exec.metrics.total_operations,
            successful_operations: exec.metrics.successful_operations,
            failed_operations: exec.metrics.failed_operations,
            block_metrics: exec
                .block_metrics
                .iter()
                .map(|bm| BlockMetricsResponse {
                    block_id: bm.block_id.clone(),
                    block_type: bm.block_type.clone(),
                    block_name: bm.block_name.clone(),
                    execution_time: bm.execution_time_ms,
                    percentage: bm.percentage,
                    counters: bm.counters.clone(),
                })
                .collect(),
        },
        errors: exec.errors.clone(),
    }
}

fn empty_metrics() -> MetricsResponse {
    MetricsResponse {
        throughput: 0.0,
        latency: LatencyResponse {
            avg: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
        },
        total_operations: 0,
        successful_operations: 0,
        failed_operations: 0,
        block_metrics: Vec::new(),
    }
}
