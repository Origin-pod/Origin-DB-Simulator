# Block System - Modular Database Construction Kit

The Block System is the foundational layer of the Modular DB Builder, providing a composable architecture for building database systems from reusable components.

## 🎯 Overview

A **block** is a self-contained component that implements a specific database function (storage engine, index, concurrency control, etc.). Blocks can be connected like LEGO pieces to create complete database systems.

### Key Features

- **🧩 Composable**: Mix and match blocks to build custom databases
- **♻️ Reusable**: Write once, use in multiple configurations
- **🧪 Testable**: Each block can be tested independently
- **📊 Observable**: Built-in metrics and monitoring
- **📚 Educational**: Learn by building real database components

## 📖 Documentation

### Planning Documents

1. **[00-OVERVIEW.md](docs/00-OVERVIEW.md)**
   - System architecture and design philosophy
   - Block categories overview
   - Development phases

2. **[01-CORE-COMPONENTS-RUST.md](docs/01-CORE-COMPONENTS-RUST.md)**
   - Core Rust traits and types
   - Block trait definition
   - Port and parameter systems
   - Registry implementation
   - Constraint and metrics systems

3. **[02-BLOCK-CATEGORIES.md](docs/02-BLOCK-CATEGORIES.md)**
   - All 10 block categories detailed
   - 55+ specific block implementations planned
   - Use cases and trade-offs

4. **[03-IMPLEMENTATION-PLAN.md](docs/03-IMPLEMENTATION-PLAN.md)**
   - 7-week development roadmap
   - Milestones and deliverables
   - Testing strategy
   - Risk management

5. **[04-EXAMPLES-RUST.md](docs/04-EXAMPLES-RUST.md)**
   - Complete Rust block implementations
   - HeapFileBlock example
   - B+tree example
   - Testing patterns
   - Benchmarking examples

6. **[05-RUST-IMPLEMENTATION-GUIDE.md](docs/05-RUST-IMPLEMENTATION-GUIDE.md)**
   - Rust-specific patterns and best practices
   - Memory management strategies
   - Concurrency patterns
   - Performance optimization
   - Testing and benchmarking

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Cargo (comes with Rust)

### Project Structure

```
block-system/
├── Cargo.toml              # Dependencies and configuration
├── src/
│   ├── lib.rs             # Library root
│   ├── core/              # Core infrastructure
│   │   ├── block.rs       # Block trait
│   │   ├── port.rs        # Port system
│   │   ├── parameter.rs   # Parameters
│   │   ├── registry.rs    # Block registry
│   │   ├── constraint.rs  # Constraints
│   │   └── metrics.rs     # Metrics
│   ├── categories/        # Block implementations
│   │   ├── storage/       # Storage engines
│   │   ├── index/         # Index structures
│   │   ├── concurrency/   # Concurrency control
│   │   ├── buffer/        # Buffer management
│   │   ├── execution/     # Query execution
│   │   ├── transaction/   # Transaction & recovery
│   │   ├── compression/   # Compression
│   │   ├── partitioning/  # Partitioning
│   │   ├── optimization/  # Optimization
│   │   └── distribution/  # Distribution
│   └── runtime/           # Execution engine
│       ├── executor.rs    # Block executor
│       └── dataflow.rs    # Data flow
├── tests/                 # Integration tests
├── benches/              # Benchmarks
└── docs/                 # Documentation
```

## 🏗️ Building a Database

### Example: Simple OLTP Database

```rust
use block_system::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create registry
    let registry = BlockRegistry::new();

    // Register blocks
    registry.register(Arc::new(HeapFileBlock::new()))?;
    registry.register(Arc::new(BPlusTreeBlock::new()))?;
    registry.register(Arc::new(TwoPhaseLockingBlock::new()))?;

    // Configure database
    let mut config = DatabaseConfig::new("My OLTP DB");

    config.add_block("storage.heap-file", "storage",
        [("page_size", 8192)])?;

    config.add_block("index.bplustree", "index",
        [("order", 128)])?;

    config.add_block("concurrency.2pl", "lock_mgr",
        [("timeout", 30000)])?;

    // Connect blocks
    config.connect("storage", "write_results", "index", "insert_key")?;

    // Run database
    let db = Database::from_config(config)?;
    db.start().await?;

    Ok(())
}
```

## 📦 Block Categories

### 1. Storage Engine Blocks
- Heap File (PostgreSQL-style)
- Clustered B-tree (InnoDB-style)
- LSM Tree (RocksDB-style)
- Columnar Storage (Parquet-style)
- PAX Storage

### 2. Index Structure Blocks
- B-tree / B+tree
- Hash Index
- Bitmap Index
- Skip List
- Trie / Radix Tree
- Inverted Index

### 3. Concurrency Control Blocks
- Two-Phase Locking (2PL)
- MVCC
- Optimistic Concurrency Control (OCC)
- Timestamp Ordering
- Serializable Snapshot Isolation (SSI)

### 4. Buffer Management Blocks
- LRU
- Clock (Second Chance)
- 2Q
- ARC
- Custom Policies

### 5. Query Execution Blocks
- Sequential Scan
- Index Scan
- Nested Loop Join
- Hash Join
- Merge Join
- Sort / Aggregation

### 6. Transaction & Recovery Blocks
- Write-Ahead Log (WAL)
- ARIES Recovery
- Shadow Paging
- Checkpoint Manager
- Commit Protocols (2PC, 3PC)

### 7-10. Additional Categories
- Compression Blocks
- Partitioning Blocks
- Optimization Blocks
- Distribution Blocks

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test heap_file

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

## 📊 Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench heap_file_insert

# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bench block_benchmarks
```

## 🎯 Development Roadmap

### Phase 1: Foundation (Weeks 1-2) ✅
- [x] Core type definitions
- [x] Port system
- [x] Parameter system
- [x] Block registry
- [x] Complete documentation

### Phase 2: First Block (Week 3)
- [ ] HeapFileBlock implementation
- [ ] B+tree implementation
- [ ] Full test coverage
- [ ] Performance benchmarks

### Phase 3: Runtime (Week 4)
- [ ] Execution engine
- [ ] Data flow system
- [ ] Metrics collection
- [ ] Integration tests

### Phase 4: Essential Blocks (Weeks 5-6)
- [ ] Concurrency control (2PL)
- [ ] Buffer management (LRU)
- [ ] Query execution (scans, joins)
- [ ] Transaction & recovery (WAL)

### Phase 5: Integration (Week 7)
- [ ] End-to-end testing
- [ ] Performance benchmarks
- [ ] Documentation
- [ ] Examples

### Phase 6: Advanced Blocks (Week 8+)
- [ ] LSM Tree
- [ ] MVCC
- [ ] Advanced joins
- [ ] Query optimization

## 🤝 Contributing

We're building this as an educational project! Contributions are welcome:

1. Pick a block from the categories
2. Follow the implementation pattern
3. Add comprehensive tests
4. Document the algorithm
5. Submit a PR

## 📚 Resources

### Database Systems
- [Database Internals](https://www.databass.dev/) by Alex Petrov
- [CMU Database Course](https://15445.courses.cs.cmu.edu/)
- [Designing Data-Intensive Applications](https://dataintensive.net/) by Martin Kleppmann

### Rust Database Projects
- [TiKV](https://github.com/tikv/tikv) - Distributed KV store
- [Sled](https://github.com/spacejam/sled) - Embedded database
- [Databend](https://github.com/datafuselabs/databend) - Cloud warehouse
- [Polars](https://github.com/pola-rs/polars) - DataFrame library

### Rust Learning
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

## 📄 License

[Choose appropriate license - MIT, Apache 2.0, etc.]

## 🎓 Educational Purpose

This project is designed for learning database internals. Each block:
- Implements a real database technique
- Is documented with academic references
- Includes complexity analysis
- Explains trade-offs
- Provides working examples

Perfect for students, educators, and anyone interested in database systems!

---

**Status**: 🚧 Planning Phase Complete - Ready for Implementation

**Next Step**: Begin Phase 2 - Implement HeapFileBlock
