# Modular DB Builder: Visual Database Construction Kit

**Last Updated**: 2026-01-20

---

## **Project Vision**

A **visual database construction simulator** that allows users to design and build custom databases from scratch using reusable, composable blocks representing fundamental database components.

### **Core Concept**
Think of it as:
- **Lego for Databases** — Snap together storage engines, index structures, concurrency protocols
- **Unreal Engine Blueprints for Data Systems** — Visual programming for database internals
- **Modular Synthesizer for Databases** — Mix and match components to create custom architectures

### **Key Differentiator**
Instead of simulating *existing* database designs, users **create new database architectures** optimized for their specific use cases by:
1. Selecting blocks (storage engine, indexing, concurrency control)
2. Connecting them on a visual canvas
3. Configuring parameters
4. Running workloads to see how the design performs
5. Iterating and experimenting

---

## **Mental Model**

```
┌─────────────────────────────────────────────┐
│          Block Library                      │
│  (Storage, Indexes, Concurrency, Buffers)   │
└─────────────────────────────────────────────┘
              ↓ Select & Configure
┌─────────────────────────────────────────────┐
│       Composition Canvas                    │
│   (Drag, Drop, Connect blocks visually)     │
└─────────────────────────────────────────────┘
              ↓ Compile to DB
┌─────────────────────────────────────────────┐
│       Simulation Engine                     │
│   (Execute workloads, collect metrics)      │
└─────────────────────────────────────────────┘
              ↓ Analyze
┌─────────────────────────────────────────────┐
│       Metrics & Visualization               │
│   (Performance, bottlenecks, tradeoffs)     │
└─────────────────────────────────────────────┘
```

---

## **Core Architecture**

### **1. Block System**

A **block** is a self-contained, reusable component that implements a specific database function.

#### **Block Anatomy**

```typescript
interface Block {
  // Identity
  id: string;
  name: string;
  category: BlockCategory;
  description: string;

  // Interface
  inputs: Port[];      // What data/signals it receives
  outputs: Port[];     // What it produces
  parameters: Parameter[];  // Configurable settings

  // Behavior
  implementation: BlockImplementation;

  // Constraints
  requires: Constraint[];   // Dependencies (e.g., "needs WAL")
  guarantees: Guarantee[];  // What it provides (e.g., "ACID compliance")

  // Metadata
  metrics: Metric[];        // What it measures (IO, latency)
  documentation: string;    // How it works
  references: string[];     // Papers, implementations
}
```

#### **Block Categories**

```
1. Storage Engine Blocks
   ├── Heap Files (PostgreSQL-style)
   ├── Clustered B-tree (InnoDB-style)
   ├── LSM Tree (RocksDB-style)
   ├── Columnar Storage (Parquet-style)
   └── PAX (Partition Attributes Across)

2. Index Structure Blocks
   ├── B-tree
   ├── B+tree
   ├── Hash Index
   ├── Bitmap Index
   ├── Skip List
   ├── Trie/Radix Tree
   └── Inverted Index (full-text)

3. Concurrency Control Blocks
   ├── Two-Phase Locking (2PL)
   ├── MVCC (Multi-Version Concurrency Control)
   ├── Optimistic Concurrency Control (OCC)
   ├── Timestamp Ordering (MVTO)
   └── Serializable Snapshot Isolation (SSI)

4. Buffer Management Blocks
   ├── LRU (Least Recently Used)
   ├── Clock (Second Chance)
   ├── 2Q (Two Queues)
   ├── ARC (Adaptive Replacement Cache)
   └── Custom Policies (hot/cold separation)

5. Query Execution Blocks
   ├── Sequential Scan
   ├── Index Scan
   ├── Nested Loop Join
   ├── Hash Join
   ├── Merge Join
   ├── Sort Operator
   └── Aggregation Operator

6. Transaction & Recovery Blocks
   ├── Write-Ahead Log (WAL)
   ├── ARIES Recovery
   ├── Shadow Paging
   ├── Checkpoint Manager
   └── Commit Protocols (2PC, 3PC)

7. Compression Blocks
   ├── Dictionary Encoding
   ├── Run-Length Encoding (RLE)
   ├── Delta Encoding
   ├── Bit Packing
   └── Columnar Compression (Parquet-style)

8. Partitioning Blocks
   ├── Hash Partitioning
   ├── Range Partitioning
   ├── List Partitioning
   └── Composite Partitioning

9. Optimization Blocks
   ├── Query Planner
   ├── Cost Model
   ├── Statistics Collector
   ├── Adaptive Indexing
   └── Materialized Views

10. Distribution Blocks (Advanced)
    ├── Replication (Primary-Replica)
    ├── Sharding
    ├── Consistency Protocols (Paxos, Raft)
    └── Query Router
```

---

### **2. Block Interface System**

Blocks communicate via **typed ports** (like in visual programming languages).

#### **Port Types**

```typescript
enum PortType {
  // Data flow
  RECORD_STREAM,      // Stream of records
  PAGE_STREAM,        // Stream of pages
  INDEX_LOOKUP,       // Key → Record mapping

  // Control flow
  TRANSACTION_START,
  TRANSACTION_COMMIT,
  LOCK_REQUEST,
  LOCK_GRANT,

  // Metadata
  STATISTICS,         // Table/index stats
  QUERY_PLAN,         // Execution plan
  METRICS             // Performance metrics
}

interface Port {
  name: string;
  type: PortType;
  dataType: DataType;  // What flows through (e.g., Record, Page, Lock)
  cardinality: '1' | 'N';  // Single or stream
}
```

#### **Example: Storage Engine Block**

```typescript
const HeapFileBlock: Block = {
  id: 'storage-heap',
  name: 'Heap File Storage',
  category: 'Storage Engine',

  inputs: [
    { name: 'insert', type: 'RECORD_STREAM', dataType: 'Record', cardinality: 'N' },
    { name: 'scan', type: 'CONTROL', dataType: 'ScanRequest', cardinality: '1' }
  ],

  outputs: [
    { name: 'records', type: 'RECORD_STREAM', dataType: 'Record', cardinality: 'N' },
    { name: 'page_ids', type: 'PAGE_STREAM', dataType: 'PageID', cardinality: 'N' }
  ],

  parameters: [
    { name: 'page_size', type: 'number', default: 8192, unit: 'bytes' },
    { name: 'fill_factor', type: 'number', default: 0.9, range: [0.5, 1.0] }
  ],

  implementation: HeapFileImplementation,

  metrics: [
    { name: 'pages_written', type: 'counter' },
    { name: 'pages_read', type: 'counter' },
    { name: 'fragmentation', type: 'gauge', unit: 'percentage' }
  ]
};
```

#### **Example: B-tree Index Block**

```typescript
const BTreeIndexBlock: Block = {
  id: 'index-btree',
  name: 'B-tree Index',
  category: 'Index Structure',

  inputs: [
    { name: 'insert', type: 'RECORD_STREAM', dataType: 'KeyValue', cardinality: 'N' },
    { name: 'lookup', type: 'INDEX_LOOKUP', dataType: 'Key', cardinality: '1' },
    { name: 'range_scan', type: 'INDEX_LOOKUP', dataType: 'KeyRange', cardinality: '1' }
  ],

  outputs: [
    { name: 'record_ids', type: 'RECORD_STREAM', dataType: 'RecordID', cardinality: 'N' },
    { name: 'stats', type: 'STATISTICS', dataType: 'IndexStats', cardinality: '1' }
  ],

  parameters: [
    { name: 'fanout', type: 'number', default: 128, range: [4, 512] },
    { name: 'unique', type: 'boolean', default: false },
    { name: 'covering_columns', type: 'string[]', default: [] }
  ],

  implementation: BTreeImplementation,

  metrics: [
    { name: 'tree_height', type: 'gauge' },
    { name: 'page_splits', type: 'counter' },
    { name: 'lookup_time', type: 'histogram', unit: 'ms' }
  ]
};
```

---

### **3. Composition Canvas**

A **visual editor** where users connect blocks to build their database architecture.

#### **Canvas Features**

1. **Block Palette** — Browse and search available blocks
2. **Drag & Drop** — Add blocks to canvas
3. **Connection Lines** — Wire outputs to inputs (type-checked)
4. **Parameter Panel** — Configure block settings
5. **Validation** — Check for invalid connections, missing dependencies
6. **Templates** — Pre-built architectures (OLTP, OLAP, Hybrid)
7. **Zoom & Pan** — Navigate complex designs
8. **Layers** — Organize blocks by subsystem

#### **Visual Representation**

```
┌──────────────────────────────────────────────────────────────┐
│  Canvas: "My Custom OLTP Database"                           │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│   ┌─────────────┐                                            │
│   │  Schema     │                                            │
│   │  Definition │                                            │
│   └──────┬──────┘                                            │
│          │ (table metadata)                                  │
│          ↓                                                    │
│   ┌─────────────┐      ┌─────────────┐                      │
│   │ Heap File   │──────→│  B-tree     │                      │
│   │ Storage     │      │  Index      │                      │
│   └──────┬──────┘      └──────┬──────┘                      │
│          │ (pages)            │ (record IDs)                 │
│          ↓                    ↓                               │
│   ┌─────────────┐      ┌─────────────┐                      │
│   │  LRU Buffer │      │   MVCC      │                      │
│   │  Pool       │      │  Concurrency│                      │
│   └─────────────┘      └─────────────┘                      │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

#### **Connection Rules**

- **Type Safety**: Only compatible ports can connect (e.g., `RECORD_STREAM` → `RECORD_STREAM`)
- **Cardinality Matching**: 1-to-1, 1-to-N, N-to-1 connections
- **Constraint Checking**: If a block requires WAL, ensure a WAL block exists
- **Cycle Detection**: Prevent circular dependencies

---

### **4. Simulation Engine**

The engine **compiles** the visual design into an executable database and runs workloads against it.

#### **Compilation Process**

```
1. Validate Canvas
   ├── Check all required inputs are connected
   ├── Verify constraints are satisfied
   └── Detect cycles or unreachable blocks

2. Topological Sort
   ├── Order blocks for execution
   └── Determine data flow paths

3. Initialize Blocks
   ├── Allocate resources (buffers, pages)
   ├── Set parameters
   └── Build internal structures (B-trees, hash tables)

4. Wire Connections
   ├── Set up data pipes between blocks
   └── Register event handlers

5. Ready to Execute
   └── Database is "compiled" and ready for workloads
```

#### **Execution Model**

Two modes:

**A. Event-Driven Execution**
- Blocks react to events (e.g., INSERT triggers storage block)
- Asynchronous, simulates real concurrent execution
- More realistic but harder to debug

**B. Step-Through Execution**
- Execute one operation at a time
- Show data flowing through blocks
- Educational, debuggable

#### **Workload Runner**

```typescript
interface Workload {
  name: string;
  queries: Query[];
  transactions: Transaction[];
  distribution: Distribution;  // Zipfian, uniform, etc.
  concurrency: number;         // Parallel threads
}

// Example workload
const OLTPWorkload: Workload = {
  name: 'E-commerce OLTP',
  queries: [
    { type: 'SELECT', template: 'SELECT * FROM users WHERE id = ?', weight: 50 },
    { type: 'INSERT', template: 'INSERT INTO orders (...) VALUES (...)', weight: 30 },
    { type: 'UPDATE', template: 'UPDATE inventory SET qty = qty - 1 WHERE id = ?', weight: 20 }
  ],
  distribution: 'zipfian',  // 20% of users get 80% of traffic
  concurrency: 100
};
```

---

### **5. Metrics & Analysis System**

Collect data at **block level** and **system level**.

#### **Block-Level Metrics**

Each block exposes its own metrics:
- **Storage Block**: Pages written, pages read, fragmentation
- **Index Block**: Lookup time, page splits, tree height
- **Buffer Block**: Cache hit ratio, evictions
- **Concurrency Block**: Lock waits, deadlocks, aborts

#### **System-Level Metrics**

Aggregate across all blocks:
- **Throughput**: Queries per second (QPS)
- **Latency**: p50, p95, p99 query response time
- **Resource Usage**: Memory, disk IO, CPU
- **Bottlenecks**: Which block is the slowest?

#### **Visualization**

```
┌─────────────────────────────────────────┐
│  Performance Dashboard                  │
├─────────────────────────────────────────┤
│                                          │
│  Throughput: 1,234 QPS                   │
│  Latency (p99): 45ms                     │
│  Cache Hit Ratio: 87%                    │
│                                          │
│  ┌───────────────────────────┐          │
│  │ Latency Breakdown:        │          │
│  │  Storage:     20ms (44%)  │          │
│  │  Index:       15ms (33%)  │          │
│  │  Locking:     10ms (22%)  │          │
│  └───────────────────────────┘          │
│                                          │
│  ⚠️ Bottleneck: Storage block           │
│     → Suggestion: Add buffer pool       │
│                                          │
└─────────────────────────────────────────┘
```

---

### **6. Block SDK (Extensibility)**

Users can **create custom blocks** and add them to the library.

#### **Block Development Flow**

```
1. Define Block Interface
   ├── Specify inputs/outputs
   ├── Define parameters
   └── Declare constraints

2. Implement Behavior
   ├── Write core algorithm (TypeScript)
   ├── Handle events (insert, read, update)
   └── Emit metrics

3. Test Block
   ├── Unit tests for logic
   ├── Integration tests with other blocks
   └── Performance benchmarks

4. Package Block
   ├── Bundle code, metadata, docs
   └── Publish to block registry

5. Share Block
   ├── Others can install and use
   └── Community can extend and fork
```

#### **Example: Custom LSM Compaction Strategy**

```typescript
// 1. Define interface
const CustomCompactionBlock: BlockDefinition = {
  id: 'compaction-tiered-custom',
  name: 'Custom Tiered Compaction',
  category: 'Optimization',

  inputs: [
    { name: 'sstables', type: 'PAGE_STREAM', dataType: 'SSTable' }
  ],

  outputs: [
    { name: 'merged_sstables', type: 'PAGE_STREAM', dataType: 'SSTable' }
  ],

  parameters: [
    { name: 'tier_ratio', type: 'number', default: 4 },
    { name: 'max_tiers', type: 'number', default: 5 }
  ]
};

// 2. Implement behavior
class CustomCompactionImplementation implements BlockImplementation {
  execute(inputs: BlockInputs, params: BlockParams): BlockOutputs {
    const sstables = inputs.sstables;
    const tierRatio = params.tier_ratio;

    // Custom compaction logic here
    const mergedSSTables = this.compactTiers(sstables, tierRatio);

    // Emit metrics
    this.emitMetric('compactions_performed', 1);
    this.emitMetric('bytes_compacted', mergedSSTables.totalSize);

    return { merged_sstables: mergedSSTables };
  }

  compactTiers(sstables: SSTable[], ratio: number): SSTable[] {
    // Implementation details...
  }
}
```

---

## **Key Design Principles**

### **1. Composability**
- Blocks are **independent** — no tight coupling
- Clear **interfaces** — well-defined inputs/outputs
- **Mix and match** — any valid combination should work

### **2. Reusability**
- Blocks are **parameterized** — one block, many configurations
- **Domain-agnostic** — works for OLTP, OLAP, streaming, etc.
- **Shareable** — export/import block libraries

### **3. Observability**
- Every block is **instrumented** — built-in metrics
- **Visual feedback** — see data flowing in real-time
- **Debuggable** — step through execution, inspect state

### **4. Extensibility**
- **Open architecture** — users can add new blocks
- **Plugin system** — custom blocks without modifying core
- **Community-driven** — share blocks, fork designs

### **5. Educational**
- **Learn by doing** — experiment with different architectures
- **Compare tradeoffs** — see impact of design choices
- **Grounded in research** — link to papers, real implementations

---

## **Use Cases**

### **Use Case 1: Optimize for Write-Heavy Workload**

**Scenario**: Time-series IoT data (high insert rate, rare reads)

**Design**:
```
Schema → LSM Tree Storage → LZ4 Compression → Sequential Buffer
                ↓
         Skip List Index (in-memory)
                ↓
         Tiered Compaction
```

**Why?**
- LSM tree: Write-optimized (append-only)
- Skip list: Fast in-memory lookups
- Tiered compaction: Minimize write amplification
- Sequential buffer: Batch writes

**Result**: 10x write throughput vs B-tree design

---

### **Use Case 2: Optimize for Read-Heavy OLTP**

**Scenario**: E-commerce product catalog (heavy reads, infrequent writes)

**Design**:
```
Schema → Clustered B-tree Storage → Covering Indexes
                ↓
         ARC Buffer Pool (adaptive)
                ↓
         MVCC Concurrency (no read locks)
```

**Why?**
- Clustered B-tree: Data physically sorted by primary key
- Covering indexes: Avoid table lookups
- ARC buffer: Adapts to access patterns
- MVCC: Readers don't block writers

**Result**: 5x read throughput, low latency

---

### **Use Case 3: Optimize for Analytics (OLAP)**

**Scenario**: Data warehouse (complex aggregations, scans)

**Design**:
```
Schema → Columnar Storage → Dictionary Encoding
                ↓
         Bitmap Indexes
                ↓
         Materialized Views
                ↓
         Vectorized Execution
```

**Why?**
- Columnar: Only read needed columns
- Dictionary encoding: Compress repeated values
- Bitmap indexes: Fast filtering
- Materialized views: Pre-compute aggregations

**Result**: 50x scan performance vs row-based storage

---

### **Use Case 4: Distributed Write-Heavy System**

**Scenario**: Social media feed (global scale, high concurrency)

**Design**:
```
Schema → Sharded LSM Trees → Replication (3x)
                ↓
         Consistent Hashing
                ↓
         Eventually Consistent Reads
                ↓
         Conflict Resolution (LWW)
```

**Why?**
- Sharding: Horizontal scalability
- Replication: High availability
- Eventually consistent: Lower latency
- LWW (Last Write Wins): Simple conflict resolution

**Result**: Millions of writes/sec, global availability

---

## **Advanced Features**

### **1. Block Variants**

Same conceptual block, multiple implementations:

```
Storage Engine Block:
├── Heap File (PostgreSQL-style)
├── Clustered Index (InnoDB-style)
├── LSM Tree (RocksDB-style)
├── Columnar (Parquet-style)
└── Custom (user-defined)
```

Users can **swap implementations** and compare performance.

---

### **2. Block Composition (Nested Blocks)**

Create **higher-level blocks** from primitive blocks:

```
"MySQL InnoDB-like Storage" Block:
  ├── Clustered B+tree
  ├── MVCC with Undo Logs
  ├── Doublewrite Buffer
  └── Adaptive Hash Index
```

This becomes a **reusable template** for others.

---

### **3. Conditional Logic Blocks**

Add **control flow** to designs:

```
Query Router Block:
  IF query type == 'SELECT':
    → Route to Read Replica
  ELSE IF query type == 'INSERT':
    → Route to Primary
  ELSE:
    → Route to Analytics Engine
```

---

### **4. Feedback Loops**

Blocks can **adapt based on runtime metrics**:

```
Adaptive Index Block:
  MONITOR query patterns
  IF certain columns frequently filtered:
    → Auto-create index
  IF index rarely used:
    → Drop index to save space
```

---

### **5. Cost Model Integration**

Each block has a **cost function**:

```typescript
interface BlockCostModel {
  estimateIO(operation: Operation): number;
  estimateCPU(operation: Operation): number;
  estimateMemory(operation: Operation): number;
  estimateLatency(operation: Operation): number;
}
```

The simulator can **predict performance** before executing.

---

## **Implementation Architecture**

### **Tech Stack**

**Frontend**:
- **React + TypeScript** — UI framework
- **React Flow** — Visual canvas (node-based editor)
- **Monaco Editor** — Code editing for custom blocks
- **D3.js / Recharts** — Metrics visualization
- **Tailwind CSS** — Styling

**Simulation Engine**:
- **TypeScript** — Core logic (runs in browser)
- **Web Workers** — Offload heavy computation
- **IndexedDB** — Persist designs locally
- **WebAssembly (future)** — High-performance simulation

**Block System**:
- **Plugin Architecture** — Dynamic block loading
- **JSON Schema** — Block metadata validation
- **Module Federation** — Share blocks across instances

---

### **Data Model**

#### **Database Design (Canvas State)**

```typescript
interface DatabaseDesign {
  id: string;
  name: string;
  description: string;

  // Visual layout
  blocks: BlockInstance[];
  connections: Connection[];

  // Configuration
  schema: Schema;
  workload: Workload;

  // Metadata
  author: string;
  created: Date;
  tags: string[];
}

interface BlockInstance {
  id: string;
  blockType: string;  // Reference to block definition
  position: { x: number; y: number };
  parameters: Record<string, any>;
}

interface Connection {
  id: string;
  source: { blockId: string; portName: string };
  target: { blockId: string; portName: string };
}
```

#### **Execution State**

```typescript
interface ExecutionState {
  designId: string;

  // Runtime state
  blocks: Map<string, BlockRuntime>;
  dataFlows: Map<string, DataStream>;

  // Metrics
  metrics: MetricsCollector;
  traces: ExecutionTrace[];

  // Control
  status: 'idle' | 'running' | 'paused' | 'stopped';
  currentStep: number;
}

interface BlockRuntime {
  blockId: string;
  state: any;  // Internal state (e.g., B-tree structure)
  metrics: BlockMetrics;
}
```

---

## **MVP Roadmap**

### **Phase 1: Core Framework (Week 1-2)**

**Goal**: Build the block system and canvas

**Features**:
- Block definition system (TypeScript interfaces)
- Visual canvas (React Flow integration)
- Drag & drop blocks
- Connect blocks
- Parameter configuration panel

**Deliverable**: User can place 3 basic blocks (Storage, Index, Buffer) and connect them.

---

### **Phase 2: Basic Simulation (Week 2-3)**

**Goal**: Execute simple workloads

**Features**:
- Implement 3-5 core blocks:
  - Heap File Storage
  - B-tree Index
  - LRU Buffer Pool
  - Sequential Scan
  - Index Scan
- Simple workload runner (INSERT, SELECT)
- Metrics collection (IO count, latency)

**Deliverable**: User can insert rows, query with index, and see metrics.

---

### **Phase 3: Visualization & Metrics (Week 3-4)**

**Goal**: Show what's happening inside

**Features**:
- Real-time metrics dashboard
- B-tree visualization (show tree structure)
- Data flow animation (show records moving between blocks)
- Step-through debugger

**Deliverable**: User can see B-tree grow as records are inserted.

---

### **Phase 4: More Blocks & Comparison (Week 4-5)**

**Goal**: Enable experimentation

**Features**:
- Add 10+ more blocks:
  - LSM Tree Storage
  - Hash Index
  - MVCC Concurrency
  - Clock Buffer Policy
  - Join operators
- Side-by-side comparison (run two designs simultaneously)
- Export/import designs (JSON)

**Deliverable**: User can compare Heap+B-tree vs LSM+SkipList performance.

---

### **Phase 5: Extensibility & Templates (Week 5-6)**

**Goal**: Community and reusability

**Features**:
- Block SDK (create custom blocks)
- Template library (OLTP, OLAP, Hybrid designs)
- Block marketplace (share custom blocks)
- Documentation generator (auto-docs from block metadata)

**Deliverable**: User can write a custom compaction strategy block and share it.

---

## **Key Technical Challenges**

### **Challenge 1: Block Execution Model**

**Problem**: How to execute blocks efficiently?

**Options**:
- **Pull-based**: Blocks request data from upstream (iterator model)
- **Push-based**: Blocks push data to downstream (pipeline model)
- **Hybrid**: Mix of both

**Recommendation**: Start with pull-based (simpler), add push later.

---

### **Challenge 2: State Management**

**Problem**: Blocks have internal state (B-tree structure, buffer contents)

**Options**:
- **Immutable snapshots**: Copy state on every operation (easy to debug, slow)
- **Mutable state**: In-place updates (fast, harder to debug)

**Recommendation**: Mutable state for performance, with optional snapshots for debugging.

---

### **Challenge 3: Concurrency Simulation**

**Problem**: How to simulate multiple concurrent transactions?

**Options**:
- **Single-threaded with event loop**: Interleave operations (simpler)
- **Web Workers**: True parallelism (more realistic, harder)

**Recommendation**: Single-threaded first, Web Workers later.

---

### **Challenge 4: Performance at Scale**

**Problem**: Simulating millions of records is slow in browser

**Solutions**:
- **Sampling**: Simulate subset of data, extrapolate metrics
- **Coarse-grained simulation**: Skip low-level details
- **WebAssembly**: Rewrite hot paths in Rust/C++

**Recommendation**: Start with sampling (10k-100k records max).

---

## **Block Library Starter Kit**

### **Must-Have Blocks (MVP)**

1. **Storage Engines** (3):
   - Heap File
   - Clustered B-tree
   - LSM Tree

2. **Indexes** (3):
   - B-tree
   - Hash Index
   - Skip List

3. **Buffers** (2):
   - LRU Buffer Pool
   - Clock Buffer Pool

4. **Concurrency** (2):
   - Two-Phase Locking
   - Basic MVCC

5. **Query Execution** (4):
   - Sequential Scan
   - Index Scan
   - Nested Loop Join
   - Hash Join

**Total: 14 blocks**

---

### **Nice-to-Have Blocks (Post-MVP)**

6. **Compression** (3):
   - Dictionary Encoding
   - RLE
   - Delta Encoding

7. **Partitioning** (2):
   - Hash Partitioning
   - Range Partitioning

8. **Recovery** (2):
   - Write-Ahead Log
   - Checkpoint Manager

9. **Advanced Indexes** (3):
   - Bitmap Index
   - Full-Text Index
   - Geospatial Index

10. **Distribution** (3):
    - Sharding
    - Replication
    - Query Router

**Total: 13 additional blocks**

---

## **Example: Building "Postgres-like" Database**

Using the block library, compose a PostgreSQL-inspired architecture:

```
┌──────────────────────────────────────────┐
│  Schema Definition                       │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  Heap File Storage                       │
│  - Page size: 8KB                        │
│  - TOAST for large values                │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  B-tree Indexes (separate from storage)  │
│  - Default index type                    │
│  - Supports range queries                │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  MVCC Concurrency Control                │
│  - Snapshot isolation                    │
│  - No read locks                         │
│  - Tuple versioning with xmin/xmax       │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  Clock Buffer Pool                       │
│  - Clock-sweep algorithm                 │
│  - shared_buffers size                   │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  Write-Ahead Log (WAL)                   │
│  - ARIES-style recovery                  │
│  - Checkpoint every N transactions       │
└───────────┬──────────────────────────────┘
            ↓
┌──────────────────────────────────────────┐
│  Query Planner                           │
│  - Cost-based optimizer                  │
│  - Uses table statistics                 │
└──────────────────────────────────────────┘
```

**Result**: A functioning database that behaves like PostgreSQL!

---

## **Research Questions**

1. **Block Granularity**: How fine-grained should blocks be?
   - Too atomic → complex wiring
   - Too coarse → not flexible enough

2. **Type System**: How to ensure type safety?
   - Static typing (compile-time checks)
   - Dynamic typing (runtime checks)

3. **Performance Fidelity**: How accurate should simulation be?
   - High-fidelity → slow, accurate
   - Low-fidelity → fast, approximate

4. **User Mental Model**: What abstraction level works best?
   - Database researchers → low-level details
   - Software engineers → higher-level concepts

5. **Block Discovery**: How do users find the right blocks?
   - Search by name/description
   - Browse by category
   - AI-powered recommendations

---

## **Success Metrics**

A successful system should:

1. **Enable Experimentation** — Users try 5+ different designs for same use case
2. **Teach Internals** — Users understand *why* certain designs perform better
3. **Be Extensible** — Community creates 50+ custom blocks
4. **Support Research** — Used in database courses or papers
5. **Be Practical** — Insights translate to real production systems

---

## **Inspiration & References**

### **Similar Systems**
- **LabVIEW** — Visual programming for instrumentation
- **Unreal Blueprint** — Visual scripting for game logic
- **Max/MSP** — Modular synthesizer for audio
- **Node-RED** — Flow-based programming for IoT
- **Scratch** — Visual programming for education

### **Database Research**
- **DBMS Internals** (CMU 15-721, Andy Pavlo)
- **Database Internals** (Alex Petrov) — Storage engines deep dive
- **Architecture of a Database System** (Hellerstein et al.)
- **The Design and Implementation of Modern Column-Oriented Database Systems** (Abadi et al.)

### **Open Source Databases to Study**
- **PostgreSQL** — Heap + MVCC + B-tree
- **MySQL InnoDB** — Clustered index + MVCC + doublewrite buffer
- **SQLite** — Single-file, B-tree for everything
- **RocksDB** — LSM tree with pluggable compaction
- **DuckDB** — Columnar, vectorized execution

---

## **Next Steps**

### **Immediate Actions**

1. **Prototype the Canvas** — Get React Flow working with basic blocks
2. **Define Core Block Interfaces** — TypeScript types for 5 essential blocks
3. **Implement First Block** — Heap File Storage (simplest)
4. **Build Simple Workload Runner** — INSERT and SELECT operations
5. **Collect First Metrics** — Pages written, pages read

### **Open Questions**

- Should we support SQL syntax or a custom query DSL?
- How to visualize complex structures (B-trees, LSM trees) in limited space?
- What's the right balance between simulation accuracy and speed?
- How to handle blocks that depend on each other (e.g., MVCC needs WAL)?

---

**End of Document**
