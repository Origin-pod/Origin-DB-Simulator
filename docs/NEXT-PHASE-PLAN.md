# Next Phase: Rust Block Implementations & WASM Integration

## Where We Are

| Layer | Status | Details |
|-------|--------|---------|
| **Frontend (React/TS)** | **COMPLETE** | All 12 milestones done. Visual canvas, workload editor, mock execution, metrics dashboard, comparison, templates, persistence, WASM bridge ready. |
| **Rust Core Types** | **COMPLETE** | Block trait, Port system, Parameter system, Metrics collector, Constraint/Guarantee types, Registry, Runtime stub. |
| **Rust Block Implementations** | **NOT STARTED** | 0 of 55+ planned blocks implemented. Only the trait and type infrastructure exists. |
| **Rust Execution Engine** | **NOT STARTED** | Runtime is a 46-line stub. No data flow, no block-to-block routing, no workload execution. |
| **WASM Compilation** | **NOT STARTED** | No `wasm-bindgen` deps, no JS exports, no `wasm-pack` setup. Frontend bridge is ready and waiting. |

---

## Phase 1: First Three Blocks (Week 1–2)

**Goal**: Implement three core blocks end-to-end with tests. These form the minimum viable pipeline: `Schema → Heap Storage → B-tree Index`.

### 1.1 — Heap File Storage Block

The foundational storage block. Reference: `docs/04-EXAMPLES-RUST.md`.

```
block-system/src/categories/storage/heap_file.rs
```

**What it does**:
- Stores records in fixed-size pages (default 8KB)
- Manages a free-space map for insert placement
- Supports insert, get-by-TupleId, sequential scan, delete (mark dead)
- Tracks metrics: pages_read, pages_written, fragmentation_pct

**Parameters**: `page_size`, `fill_factor`

**Ports**:
- Input: `records` (DataStream)
- Output: `stored` (DataStream)

**Tests**:
- Insert N records, verify page allocation
- Sequential scan returns all live records
- Delete marks slots dead, doesn't return on scan
- Fill factor respected (new page when threshold hit)

---

### 1.2 — B-tree Index Block

Point-lookup and range-scan index.

```
block-system/src/categories/index/btree.rs
```

**What it does**:
- In-memory B-tree with configurable fanout
- Supports insert key→TupleId, point lookup, range scan
- Tracks metrics: tree_depth, lookups, splits, pages_read

**Parameters**: `fanout`, `key_column`, `unique`

**Ports**:
- Input: `records` (DataStream) — keys to index
- Output: `lookup_results` (DataStream)

**Tests**:
- Insert 10K keys, verify O(log n) depth
- Point lookup returns correct TupleId
- Range scan returns ordered results
- Unique constraint rejects duplicates

---

### 1.3 — LRU Buffer Pool Block

Caching layer between storage and execution.

```
block-system/src/categories/buffer/lru_buffer.rs
```

**What it does**:
- Fixed-size page cache with LRU eviction
- get_page(id) → cache hit or miss
- Tracks metrics: cache_hits, cache_misses, hit_rate_pct, evictions

**Parameters**: `size` (number of pages), `page_size`

**Ports**:
- Input: `requests` (DataStream) — page requests
- Output: `pages` (DataStream) — served pages

**Tests**:
- Access patterns that fit in cache → 100% hit rate
- Access patterns exceeding cache → evictions happen in LRU order
- Verify hit rate metric accuracy

---

### 1.4 — Wire Blocks Together

Create integration tests that compose all three:

```rust
// Schema → HeapFile → BTreeIndex
//                  └→ LRUBuffer → SequentialScan
```

- Insert 1000 records through HeapFile
- Index them with B-tree
- Read through LRU Buffer
- Verify metrics aggregate correctly

---

## Phase 2: Execution Engine (Week 3–4)

**Goal**: Build the runtime that connects blocks via ports, routes data, and collects metrics.

### 2.1 — Data Flow Engine

```
block-system/src/runtime/engine.rs
```

**What it builds**:
- Accepts a graph of blocks + connections (matching frontend's canvas model)
- Topological sort to determine execution order
- Routes `PortValue` data from output ports to connected input ports
- Collects per-block timing and metrics
- Supports cancellation

**Key types**:
```rust
pub struct ExecutionEngine {
    blocks: HashMap<String, Box<dyn Block>>,
    connections: Vec<Connection>,
    metrics: HashMap<String, MetricsCollector>,
}

impl ExecutionEngine {
    pub fn add_block(&mut self, id: String, block: Box<dyn Block>);
    pub fn add_connection(&mut self, conn: Connection);
    pub fn validate(&self) -> ValidationResult;
    pub async fn execute(&mut self, workload: WorkloadConfig) -> ExecutionResult;
    pub fn cancel(&mut self);
}
```

### 2.2 — Workload Generator

```
block-system/src/runtime/workload.rs
```

- Takes workload config (operations, distribution, concurrency)
- Generates operation stream (INSERT/SELECT/UPDATE/DELETE)
- Supports distributions: uniform, zipfian, latest
- Feeds operations into the block graph's entry point

### 2.3 — Validation Engine

```
block-system/src/runtime/validation.rs
```

- Graph validation: no cycles, required ports connected, type compatibility
- Parameter validation: constraints checked
- Mirrors the frontend's MockExecutionEngine validation but in Rust

---

## Phase 3: WASM Compilation (Week 5–6)

**Goal**: Compile the Rust block-system to WASM and connect it to the frontend.

### 3.1 — Add WASM Dependencies

```toml
# block-system/Cargo.toml additions

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1"

[profile.release]
opt-level = "s"       # Optimize for size
lto = true
```

### 3.2 — WASM Exports Module

```
block-system/src/wasm_api.rs
```

Create `#[wasm_bindgen]` exports matching the frontend's `WASMModule` interface:

```rust
#[wasm_bindgen]
pub fn init_runtime() { ... }

#[wasm_bindgen]
pub fn register_block(config_json: &str) -> String { ... }

#[wasm_bindgen]
pub fn create_connection(conn_json: &str) -> String { ... }

#[wasm_bindgen]
pub fn validate() -> String { ... }

#[wasm_bindgen]
pub fn execute(workload_json: &str, progress_callback: &js_sys::Function) -> String { ... }

#[wasm_bindgen]
pub fn cancel_execution() { ... }

#[wasm_bindgen]
pub fn get_metrics() -> String { ... }

#[wasm_bindgen]
pub fn get_block_types() -> String { ... }
```

### 3.3 — Build Pipeline

```bash
# Install wasm-pack
cargo install wasm-pack

# Build WASM (outputs to frontend/src/pkg/)
wasm-pack build --target web --out-dir ../frontend/src/pkg

# Frontend picks it up automatically via the existing loader
```

### 3.4 — Frontend Integration

The frontend already has everything wired:
- `wasm/loader.ts` — loads `../pkg/block_system`
- `wasm/bridge.ts` — typed API over WASM module
- `engine/WASMExecutionEngine.ts` — implements `ExecutionEngine` via bridge
- `engine/ExecutionEngine.ts` — factory picks WASM when available
- `stores/executionStore.ts` — shows "Mock" vs "WASM" badge

**No frontend changes needed** — just build the WASM and it lights up.

---

## Phase 4: More Blocks (Week 7–8)

### Storage Blocks
| Block | Priority | Complexity |
|-------|----------|------------|
| LSM Tree | High | Hard — memtable + levels + compaction |
| Clustered Storage | Medium | Medium — B-tree ordered by cluster key |
| Columnar Storage | Medium | Medium — column chunks + compression |

### Index Blocks
| Block | Priority | Complexity |
|-------|----------|------------|
| Hash Index | High | Easy — hash map with bucket chaining |
| Covering Index | Medium | Medium — index with included columns |

### Execution Blocks
| Block | Priority | Complexity |
|-------|----------|------------|
| Sequential Scan | High | Easy — iterate all pages |
| Index Scan | High | Easy — lookup via index, fetch from storage |
| Filter | Medium | Easy — predicate evaluation |
| Sort | Medium | Medium — in-memory + external sort |
| Hash Join | Low | Hard — build/probe with spill |

### Concurrency Blocks
| Block | Priority | Complexity |
|-------|----------|------------|
| Row Lock (2PL) | High | Medium — lock table + deadlock detection |
| MVCC | High | Hard — version chains + garbage collection |

### Transaction Blocks
| Block | Priority | Complexity |
|-------|----------|------------|
| WAL | High | Medium — append-only log + fsync simulation |

---

## Phase 5: Polish & Ship (Week 9–10)

### Benchmarks
- Set up `criterion` benchmarks for each block
- Compare block configurations (e.g., B-tree fanout 64 vs 256)
- Publish benchmark results in the app

### Educational Content
- Add algorithm descriptions to each block's documentation field
- Show complexity analysis (O(log n) lookups, O(1) hash, etc.)
- Link to papers/references (B-tree: Bayer & McCreight 1972, etc.)

### Testing
- Property-based tests with `proptest` for all blocks
- Fuzz testing for edge cases
- End-to-end tests: template designs → WASM execution → metrics verification

### Deployment
- GitHub Pages for the frontend (static site)
- WASM bundle served alongside the frontend
- No backend needed — everything runs in the browser

---

## File Structure After Phase 3

```
block-system/src/
├── core/                          # ✅ Done — traits & types
│   ├── block.rs
│   ├── port.rs
│   ├── parameter.rs
│   ├── constraint.rs
│   ├── metrics.rs
│   └── registry.rs
├── categories/                    # 🔨 Phase 1 & 4
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── heap_file.rs           # Phase 1
│   │   ├── lsm_tree.rs            # Phase 4
│   │   └── clustered.rs           # Phase 4
│   ├── index/
│   │   ├── mod.rs
│   │   ├── btree.rs               # Phase 1
│   │   └── hash_index.rs          # Phase 4
│   ├── buffer/
│   │   ├── mod.rs
│   │   └── lru_buffer.rs          # Phase 1
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── sequential_scan.rs     # Phase 4
│   │   └── filter.rs              # Phase 4
│   ├── concurrency/
│   │   ├── mod.rs
│   │   ├── row_lock.rs            # Phase 4
│   │   └── mvcc.rs                # Phase 4
│   └── transaction/
│       ├── mod.rs
│       └── wal.rs                  # Phase 4
├── runtime/                       # 🔨 Phase 2
│   ├── mod.rs
│   ├── engine.rs                  # Execution engine
│   ├── workload.rs                # Workload generator
│   └── validation.rs              # Graph validation
├── wasm_api.rs                    # 🔨 Phase 3 — wasm-bindgen exports
└── lib.rs
```

---

## Success Criteria

| Phase | Done When |
|-------|-----------|
| **Phase 1** | 3 blocks pass all unit tests, integration test connects them |
| **Phase 2** | Engine executes a 3-block pipeline, returns real metrics |
| **Phase 3** | `wasm-pack build` succeeds, frontend shows "WASM" badge, execution uses real engine |
| **Phase 4** | 10+ blocks implemented, all 4 templates run on WASM |
| **Phase 5** | Deployed to GitHub Pages, benchmarks documented |

---

## Quick Start: Phase 1

```bash
# From project root
cd block-system

# Run existing tests to verify foundation
cargo test

# Start implementing: create the storage module
mkdir -p src/categories/storage
# → implement heap_file.rs

# Test as you go
cargo test --lib

# When Phase 3 is ready:
wasm-pack build --target web --out-dir ../frontend/src/pkg
cd ../frontend && npm run dev
# → Frontend auto-detects WASM and shows green "WASM" badge
```
