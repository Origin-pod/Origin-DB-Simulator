# Block Categories - Detailed Planning

This document outlines all 10 block categories and their specific implementations.

---

## 1. Storage Engine Blocks

**Purpose**: Define how data is physically stored on disk/memory.

### 1.1 Heap File Storage (PostgreSQL-style)
- **Description**: Unordered collection of pages
- **Use Case**: General-purpose storage, no clustering
- **Inputs**: Write requests, read requests
- **Outputs**: Data records
- **Parameters**:
  - Page size (default: 8KB)
  - Fill factor
  - Free space map threshold
- **Metrics**: Pages written, pages read, fragmentation

### 1.2 Clustered B-tree Storage (InnoDB-style)
- **Description**: Data stored in B-tree leaf nodes, ordered by primary key
- **Use Case**: Range queries, ordered access
- **Inputs**: Insert/update/delete operations
- **Outputs**: Ordered data records
- **Parameters**:
  - Page size
  - Fill factor
  - Split threshold
- **Metrics**: Tree height, rebalancing operations

### 1.3 LSM Tree Storage (RocksDB-style)
- **Description**: Log-structured merge tree with levels
- **Use Case**: Write-heavy workloads
- **Inputs**: Write operations
- **Outputs**: Data records (via compaction)
- **Parameters**:
  - Memtable size
  - Number of levels
  - Compaction strategy
- **Metrics**: Write amplification, compaction overhead

### 1.4 Columnar Storage (Parquet-style)
- **Description**: Column-oriented storage
- **Use Case**: Analytical queries, OLAP
- **Inputs**: Columnar data batches
- **Outputs**: Column chunks
- **Parameters**:
  - Row group size
  - Compression per column
- **Metrics**: Compression ratio, column scan time

### 1.5 PAX Storage (Partition Attributes Across)
- **Description**: Hybrid row/column storage within pages
- **Use Case**: Mixed workloads
- **Inputs**: Mixed read/write patterns
- **Outputs**: Data records
- **Parameters**:
  - Minipages per page
  - Cache line size
- **Metrics**: Cache hit rate, scan efficiency

---

## 2. Index Structure Blocks

**Purpose**: Fast data lookup mechanisms.

### 2.1 B-tree Index
- **Description**: Balanced tree with keys in all nodes
- **Use Case**: Exact match, range queries
- **Parameters**: Order, page size
- **Metrics**: Tree height, splits/merges

### 2.2 B+tree Index
- **Description**: All data in leaf nodes, linked
- **Use Case**: Range scans, sequential access
- **Parameters**: Order, fill factor
- **Metrics**: Leaf-to-leaf traversals

### 2.3 Hash Index
- **Description**: Hash table for exact matches
- **Use Case**: Equality lookups
- **Parameters**: Bucket count, hash function
- **Metrics**: Collision rate

### 2.4 Bitmap Index
- **Description**: Bitmap per distinct value
- **Use Case**: Low-cardinality columns, OLAP
- **Parameters**: Compression method
- **Metrics**: Bitmap operations

### 2.5 Skip List
- **Description**: Probabilistic balanced structure
- **Use Case**: Concurrent access, simple implementation
- **Parameters**: Max level, probability
- **Metrics**: Average search depth

### 2.6 Trie/Radix Tree
- **Description**: Prefix tree for strings
- **Use Case**: String prefix matching
- **Parameters**: Branching factor
- **Metrics**: Tree depth

### 2.7 Inverted Index
- **Description**: Token → document list
- **Use Case**: Full-text search
- **Parameters**: Tokenizer, stemming
- **Metrics**: Index size, query latency

---

## 3. Concurrency Control Blocks

**Purpose**: Manage concurrent access to data.

### 3.1 Two-Phase Locking (2PL)
- **Description**: Growing phase (acquire locks), shrinking phase (release)
- **Guarantees**: Serializability
- **Parameters**:
  - Lock timeout
  - Deadlock detection interval
- **Metrics**: Lock wait time, deadlocks

### 3.2 MVCC (Multi-Version Concurrency Control)
- **Description**: Multiple versions of data
- **Guarantees**: Snapshot isolation
- **Parameters**:
  - Version retention policy
  - Vacuum threshold
- **Metrics**: Version chain length, vacuum overhead

### 3.3 Optimistic Concurrency Control (OCC)
- **Description**: Validate at commit time
- **Guarantees**: Serializability
- **Parameters**:
  - Validation strategy
  - Retry limit
- **Metrics**: Abort rate, validation overhead

### 3.4 Timestamp Ordering (MVTO)
- **Description**: Assign timestamps to transactions
- **Guarantees**: Serializability
- **Parameters**:
  - Timestamp generation method
- **Metrics**: Timestamp conflicts

### 3.5 Serializable Snapshot Isolation (SSI)
- **Description**: Snapshot isolation + serializable checking
- **Guarantees**: Full serializability
- **Parameters**:
  - Conflict detection granularity
- **Metrics**: False positive rate

---

## 4. Buffer Management Blocks

**Purpose**: Cache pages in memory.

### 4.1 LRU (Least Recently Used)
- **Description**: Evict least recently used page
- **Parameters**: Buffer pool size
- **Metrics**: Hit rate, evictions

### 4.2 Clock (Second Chance)
- **Description**: Circular buffer with reference bit
- **Parameters**: Buffer pool size
- **Metrics**: Clock hand rotations

### 4.3 2Q (Two Queues)
- **Description**: Separate queues for hot/cold data
- **Parameters**: Queue size ratio
- **Metrics**: Queue hit rates

### 4.4 ARC (Adaptive Replacement Cache)
- **Description**: Balances recency and frequency
- **Parameters**: Target sizes
- **Metrics**: Adaptation rate

### 4.5 Custom Policies
- **Description**: Workload-specific strategies
- **Parameters**: Custom logic
- **Metrics**: Application-specific

---

## 5. Query Execution Blocks

**Purpose**: Execute query operations.

### 5.1 Sequential Scan
- **Description**: Scan entire table
- **Outputs**: Filtered records
- **Metrics**: Pages scanned, selectivity

### 5.2 Index Scan
- **Description**: Use index to find records
- **Inputs**: Index block
- **Metrics**: Index lookups, random I/O

### 5.3 Nested Loop Join
- **Description**: Iterate outer × inner
- **Parameters**: Block nested vs. tuple
- **Metrics**: Iterations, I/O cost

### 5.4 Hash Join
- **Description**: Build hash table, probe
- **Parameters**: Hash table size
- **Metrics**: Build time, probe time

### 5.5 Merge Join
- **Description**: Merge sorted inputs
- **Requires**: Sorted inputs
- **Metrics**: Comparison count

### 5.6 Sort Operator
- **Description**: Sort data stream
- **Parameters**: Algorithm (quicksort, merge)
- **Metrics**: Memory spills

### 5.7 Aggregation Operator
- **Description**: Compute aggregates (SUM, COUNT, etc.)
- **Parameters**: Hash vs. sort-based
- **Metrics**: Hash table size

---

## 6. Transaction & Recovery Blocks

**Purpose**: ACID guarantees and crash recovery.

### 6.1 Write-Ahead Log (WAL)
- **Description**: Log changes before applying
- **Guarantees**: Durability, atomicity
- **Parameters**:
  - Log buffer size
  - Flush policy (commit, periodic)
- **Metrics**: Log size, flush time

### 6.2 ARIES Recovery
- **Description**: Analysis, redo, undo phases
- **Guarantees**: Crash recovery
- **Parameters**: Checkpoint interval
- **Metrics**: Recovery time

### 6.3 Shadow Paging
- **Description**: Copy-on-write pages
- **Guarantees**: Atomic commits
- **Parameters**: Shadow page table size
- **Metrics**: Page copy overhead

### 6.4 Checkpoint Manager
- **Description**: Create consistent snapshots
- **Parameters**: Checkpoint strategy
- **Metrics**: Checkpoint time

### 6.5 Commit Protocols
- **Description**: 2PC, 3PC for distributed transactions
- **Guarantees**: Atomic commits
- **Metrics**: Coordinator overhead

---

## 7. Compression Blocks

**Purpose**: Reduce storage footprint.

### 7.1 Dictionary Encoding
- **Description**: Map values to integers
- **Use Case**: Low-cardinality strings
- **Metrics**: Dictionary size, compression ratio

### 7.2 Run-Length Encoding (RLE)
- **Description**: Encode runs of same value
- **Use Case**: Sorted data, bitmaps
- **Metrics**: Run length distribution

### 7.3 Delta Encoding
- **Description**: Store deltas from base value
- **Use Case**: Timestamps, sequences
- **Metrics**: Delta size distribution

### 7.4 Bit Packing
- **Description**: Pack values into fewer bits
- **Use Case**: Small integer ranges
- **Metrics**: Bits per value

### 7.5 Columnar Compression
- **Description**: Combined techniques (Parquet-style)
- **Use Case**: Analytical workloads
- **Metrics**: Overall compression ratio

---

## 8. Partitioning Blocks

**Purpose**: Divide data into partitions.

### 8.1 Hash Partitioning
- **Description**: Hash function determines partition
- **Use Case**: Even distribution
- **Parameters**: Hash function, partition count
- **Metrics**: Skew

### 8.2 Range Partitioning
- **Description**: Value ranges → partitions
- **Use Case**: Time-series, ordered data
- **Parameters**: Range boundaries
- **Metrics**: Partition sizes

### 8.3 List Partitioning
- **Description**: Explicit value lists
- **Use Case**: Categorical data
- **Parameters**: Value lists
- **Metrics**: Partition distribution

### 8.4 Composite Partitioning
- **Description**: Combine multiple strategies
- **Use Case**: Complex partitioning needs
- **Metrics**: Multi-level distribution

---

## 9. Optimization Blocks

**Purpose**: Query optimization and performance tuning.

### 9.1 Query Planner
- **Description**: Generate query execution plans
- **Outputs**: Execution plan
- **Metrics**: Planning time

### 9.2 Cost Model
- **Description**: Estimate operation costs
- **Parameters**: Cost coefficients
- **Metrics**: Estimation accuracy

### 9.3 Statistics Collector
- **Description**: Collect data distribution stats
- **Outputs**: Histograms, distinct counts
- **Metrics**: Stats freshness

### 9.4 Adaptive Indexing
- **Description**: Automatically create indexes
- **Parameters**: Threshold for index creation
- **Metrics**: Index utility

### 9.5 Materialized Views
- **Description**: Precomputed query results
- **Parameters**: Refresh strategy
- **Metrics**: View staleness

---

## 10. Distribution Blocks (Advanced)

**Purpose**: Distributed database capabilities.

### 10.1 Replication (Primary-Replica)
- **Description**: Replicate data to multiple nodes
- **Parameters**: Synchronous vs. asynchronous
- **Metrics**: Replication lag

### 10.2 Sharding
- **Description**: Partition data across nodes
- **Parameters**: Sharding key
- **Metrics**: Cross-shard queries

### 10.3 Consistency Protocols (Paxos, Raft)
- **Description**: Distributed consensus
- **Guarantees**: Consistency
- **Metrics**: Consensus latency

### 10.4 Query Router
- **Description**: Route queries to correct shard/replica
- **Parameters**: Routing strategy
- **Metrics**: Routing overhead

---

## Implementation Priority

### Tier 1 (Essential - Implement First)
- Heap File Storage
- B+tree Index
- Two-Phase Locking
- LRU Buffer Manager
- Sequential Scan
- Write-Ahead Log

### Tier 2 (Core Features)
- LSM Tree Storage
- Hash Index
- MVCC
- Hash Join, Merge Join
- ARIES Recovery
- Dictionary Encoding

### Tier 3 (Advanced)
- Columnar Storage
- Bitmap Index
- SSI
- ARC Buffer Manager
- Query Planner
- Replication

### Tier 4 (Specialized)
- PAX Storage
- Inverted Index
- Distribution Blocks
- Adaptive Indexing
