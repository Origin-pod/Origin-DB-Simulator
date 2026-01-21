# Rust Implementation Guide

This guide covers Rust-specific considerations and best practices for implementing the Block System.

---

## Why Rust for Database Systems?

### Advantages
- **Memory Safety**: No null pointer dereferences, no data races
- **Zero-Cost Abstractions**: High-level code with C-like performance
- **Fearless Concurrency**: Thread safety enforced at compile time
- **Excellent Package Management**: Cargo makes dependency management easy
- **Growing Ecosystem**: Many DB-related crates (e.g., RocksDB, Sled)

### Real-World Examples
- **TiKV**: Distributed key-value store (CNCF project)
- **Sled**: Embedded database
- **Databend**: Cloud data warehouse
- **Polars**: DataFrame library
- **RisingWave**: Streaming database

---

## Project Setup

### 1. Initialize Workspace

```bash
# Create workspace
mkdir db-simulator
cd db-simulator

# Initialize cargo workspace
cargo init --lib block-system
cargo init --lib visual-editor
cargo init --bin simulator

# Create workspace Cargo.toml
cat > Cargo.toml << 'EOF'
[workspace]
members = ["block-system", "visual-editor", "simulator"]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
thiserror = "1.0"
EOF
```

### 2. Block System Dependencies

```toml
# block-system/Cargo.toml

[package]
name = "block-system"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
bytes = "1.5"

# Concurrency
parking_lot = "0.12"  # Faster RwLock
crossbeam = "0.8"

# Optional: for actual storage
rocksdb = { version = "0.21", optional = true }

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
proptest = "1.4"  # Property-based testing
tempfile = "3.8"

[features]
default = []
rocksdb-backend = ["rocksdb"]

[[bench]]
name = "block_benchmarks"
harness = false
```

---

## Core Design Patterns

### 1. Trait Objects vs Generic Traits

```rust
// Option 1: Trait objects (dynamic dispatch)
pub trait Block: Send + Sync {
    fn execute(&mut self, ctx: ExecutionContext) -> Result<ExecutionResult>;
}

// Usage: Store different blocks in Vec
let blocks: Vec<Box<dyn Block>> = vec![
    Box::new(HeapFileBlock::new()),
    Box::new(BPlusTreeBlock::new()),
];

// Option 2: Generic traits (static dispatch)
pub trait Block: Send + Sync {
    type Input;
    type Output;
    fn execute(&mut self, input: Self::Input) -> Result<Self::Output>;
}

// Usage: Better performance, but less flexible
fn process<B: Block>(block: &mut B, input: B::Input) -> B::Output {
    block.execute(input).unwrap()
}
```

**Recommendation**: Use trait objects for the registry (flexibility), but provide generic APIs for hot paths (performance).

### 2. Error Handling with thiserror

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Block initialization failed: {0}")]
    InitializationError(String),

    #[error("Invalid parameter '{param}': {reason}")]
    InvalidParameter { param: String, reason: String },

    #[error("Port '{0}' not found")]
    PortNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Usage
fn validate_param(value: i64) -> Result<(), BlockError> {
    if value < 0 {
        return Err(BlockError::InvalidParameter {
            param: "page_size".to_string(),
            reason: "must be positive".to_string(),
        });
    }
    Ok(())
}
```

### 3. Async/Await

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Block: Send + Sync {
    async fn initialize(&mut self, params: Params) -> Result<()>;
    async fn execute(&mut self, ctx: ExecutionContext) -> Result<ExecutionResult>;
}

// Implementation
#[async_trait]
impl Block for HeapFileBlock {
    async fn initialize(&mut self, params: Params) -> Result<()> {
        // Async initialization
        tokio::fs::create_dir_all(&self.data_dir).await?;
        Ok(())
    }

    async fn execute(&mut self, ctx: ExecutionContext) -> Result<ExecutionResult> {
        // Can use .await inside
        let data = self.read_page(page_id).await?;
        Ok(ExecutionResult { data })
    }
}
```

### 4. Interior Mutability for Shared State

```rust
use parking_lot::RwLock;
use std::sync::Arc;

pub struct HeapFileBlock {
    metadata: BlockMetadata,
    // Shared, thread-safe mutable state
    pages: Arc<RwLock<Vec<Page>>>,
    free_space_map: Arc<RwLock<HashMap<usize, f64>>>,
}

impl HeapFileBlock {
    pub fn insert_tuple(&self, data: &[u8]) -> Result<TupleId> {
        // Write lock for mutation
        let mut pages = self.pages.write();
        // ... insert logic
        Ok(TupleId { page_id: 0, slot_id: 0 })
    }

    pub fn read_tuple(&self, tid: &TupleId) -> Result<Vec<u8>> {
        // Read lock for reading
        let pages = self.pages.read();
        // ... read logic
        Ok(vec![])
    }
}
```

**Why `parking_lot`?**
- Faster than `std::sync::RwLock`
- No poisoning (simpler API)
- Better performance under contention

### 5. Zero-Copy with Bytes

```rust
use bytes::{Bytes, BytesMut, Buf, BufMut};

pub struct Page {
    data: Bytes,  // Reference-counted, zero-copy
}

impl Page {
    pub fn new(size: usize) -> Self {
        let mut buf = BytesMut::with_capacity(size);
        buf.resize(size, 0);
        Self {
            data: buf.freeze(),
        }
    }

    pub fn write_at(&mut self, offset: usize, data: &[u8]) {
        // Clone is cheap (just increments refcount)
        let mut writable = BytesMut::from(self.data.as_ref());
        writable[offset..offset + data.len()].copy_from_slice(data);
        self.data = writable.freeze();
    }

    pub fn read_at(&self, offset: usize, len: usize) -> Bytes {
        // Zero-copy slice
        self.data.slice(offset..offset + len)
    }
}
```

---

## Memory Management

### 1. Avoid Allocations in Hot Paths

```rust
// Bad: Allocates on every call
fn process_record(record: &Record) -> String {
    format!("Processing: {}", record.id)
}

// Good: Reuse buffer
fn process_records(records: &[Record], buf: &mut String) {
    for record in records {
        buf.clear();
        use std::fmt::Write;
        write!(buf, "Processing: {}", record.id).unwrap();
        // Use buf...
    }
}

// Even better: Use stack allocation when possible
fn process_record_stack(record: &Record) -> arrayvec::ArrayString<64> {
    use std::fmt::Write;
    let mut buf = arrayvec::ArrayString::new();
    write!(&mut buf, "Processing: {}", record.id).unwrap();
    buf
}
```

### 2. Object Pooling

```rust
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

pub struct PagePool {
    free_pages: Arc<ArrayQueue<Page>>,
    page_size: usize,
}

impl PagePool {
    pub fn new(capacity: usize, page_size: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(capacity));

        // Pre-allocate pages
        for _ in 0..capacity {
            let _ = pool.push(Page::new(page_size));
        }

        Self {
            free_pages: pool,
            page_size,
        }
    }

    pub fn acquire(&self) -> Option<Page> {
        self.free_pages.pop()
            .or_else(|| Some(Page::new(self.page_size)))
    }

    pub fn release(&self, page: Page) {
        let _ = self.free_pages.push(page);
        // If queue is full, page is dropped (that's okay)
    }
}
```

### 3. Arena Allocation

```rust
use typed_arena::Arena;

// Useful for temporary allocations in a query execution
pub struct QueryContext<'arena> {
    arena: &'arena Arena<Record>,
}

impl<'arena> QueryContext<'arena> {
    pub fn alloc_record(&self, id: i64, data: String) -> &'arena Record {
        self.arena.alloc(Record { id, data })
    }
}

// Usage
fn execute_query() {
    let arena = Arena::new();
    let ctx = QueryContext { arena: &arena };

    // Allocate many records
    for i in 0..1000 {
        let record = ctx.alloc_record(i, format!("data_{}", i));
        // Use record...
    }

    // All records freed at once when arena drops
}
```

---

## Concurrency Patterns

### 1. Lock-Free Data Structures

```rust
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct LockFreeMetrics {
    counter: AtomicU64,
    events: SegQueue<Event>,
}

impl LockFreeMetrics {
    pub fn increment(&self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_event(&self, event: Event) {
        self.events.push(event);
    }

    pub fn get_count(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }
}
```

### 2. Actor Pattern with Tokio

```rust
use tokio::sync::mpsc;

pub struct StorageActor {
    receiver: mpsc::Receiver<StorageCommand>,
    state: HashMap<String, Vec<u8>>,
}

pub enum StorageCommand {
    Write { key: String, value: Vec<u8>, response: oneshot::Sender<Result<()>> },
    Read { key: String, response: oneshot::Sender<Result<Vec<u8>>> },
}

impl StorageActor {
    async fn run(mut self) {
        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                StorageCommand::Write { key, value, response } => {
                    self.state.insert(key, value);
                    let _ = response.send(Ok(()));
                }
                StorageCommand::Read { key, response } => {
                    let result = self.state.get(&key).cloned()
                        .ok_or_else(|| anyhow::anyhow!("Key not found"));
                    let _ = response.send(result);
                }
            }
        }
    }
}

// Client handle
#[derive(Clone)]
pub struct StorageHandle {
    sender: mpsc::Sender<StorageCommand>,
}

impl StorageHandle {
    pub async fn write(&self, key: String, value: Vec<u8>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(StorageCommand::Write { key, value, response: tx }).await?;
        rx.await?
    }

    pub async fn read(&self, key: String) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(StorageCommand::Read { key, response: tx }).await?;
        rx.await?
    }
}
```

### 3. Parallel Processing with Rayon

```rust
use rayon::prelude::*;

// Process records in parallel
pub fn process_batch(records: &[Record]) -> Vec<ProcessedRecord> {
    records.par_iter()
        .map(|record| process_single(record))
        .collect()
}

// Parallel aggregation
pub fn aggregate_metrics(blocks: &[Arc<dyn Block>]) -> HashMap<String, f64> {
    blocks.par_iter()
        .flat_map(|block| {
            block.get_metrics()
                .into_iter()
                .map(|(k, v)| (k, v))
        })
        .fold(HashMap::new, |mut acc, (key, value)| {
            *acc.entry(key).or_insert(0.0) += value;
            acc
        })
        .reduce(HashMap::new, |mut a, b| {
            for (k, v) in b {
                *a.entry(k).or_insert(0.0) += v;
            }
            a
        })
}
```

---

## Testing Strategies

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_allocation() {
        let pool = PagePool::new(10, 8192);
        let page = pool.acquire().unwrap();
        assert_eq!(page.size(), 8192);
        pool.release(page);
    }

    #[tokio::test]
    async fn test_async_execution() {
        let mut block = HeapFileBlock::new();
        block.initialize(HashMap::new()).await.unwrap();

        let result = block.execute(ExecutionContext::default()).await;
        assert!(result.is_ok());
    }
}
```

### 2. Property-Based Testing with Proptest

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_insert_read_roundtrip(data in prop::collection::vec(any::<u8>(), 1..1000)) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let mut block = HeapFileBlock::new();
            block.initialize(HashMap::new()).await.unwrap();

            let tid = block.insert_tuple(&data).unwrap();
            let read_data = block.read_tuple(&tid).unwrap();

            prop_assert_eq!(data, read_data);
        });
    }

    #[test]
    fn test_btree_properties(
        keys in prop::collection::vec(0i64..10000, 1..100)
    ) {
        let mut tree = BPlusTree::new(128);

        // Insert all keys
        for key in &keys {
            tree.insert(*key, vec![]).unwrap();
        }

        // All keys should be findable
        for key in &keys {
            prop_assert!(tree.search(*key).is_some());
        }

        // Tree should be balanced
        prop_assert!(tree.check_balance());
    }
}
```

### 3. Integration Tests

```rust
// tests/integration_test.rs

use block_system::*;

#[tokio::test]
async fn test_full_transaction() {
    let registry = BlockRegistry::new();
    registry.register(Arc::new(HeapFileBlock::new())).unwrap();
    registry.register(Arc::new(BPlusTreeBlock::new())).unwrap();

    // Create configuration
    let config = create_test_config();

    // Execute workload
    let results = execute_workload(&config, 1000).await.unwrap();

    // Verify correctness
    assert_eq!(results.len(), 1000);
}
```

### 4. Benchmarking with Criterion

```rust
// benches/storage_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use block_system::*;

fn bench_insert_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_throughput");

    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            b.iter(|| {
                runtime.block_on(async {
                    let mut block = HeapFileBlock::new();
                    block.initialize(HashMap::new()).await.unwrap();

                    for i in 0..size {
                        let data = format!("record_{}", i).into_bytes();
                        black_box(block.insert_tuple(&data).unwrap());
                    }
                });
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_insert_throughput);
criterion_main!(benches);
```

---

## Performance Optimization

### 1. Profile-Guided Optimization

```toml
# Cargo.toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization, slower compile
opt-level = 3           # Maximum optimization
debug = true            # Keep debug symbols for profiling

[profile.bench]
inherits = "release"
```

### 2. Using `perf` for Profiling

```bash
# Build with debug symbols
cargo build --release

# Run with perf
perf record --call-graph dwarf ./target/release/block-system

# Analyze
perf report

# Or use flamegraph
cargo install flamegraph
cargo flamegraph --bench block_benchmarks
```

### 3. Memory Profiling with Valgrind

```bash
# Install valgrind
sudo apt install valgrind

# Run memory check
valgrind --leak-check=full ./target/release/block-system

# Or use heaptrack
heaptrack ./target/release/block-system
```

---

## Best Practices

### 1. Error Handling
- Use `Result<T, E>` everywhere
- Use `?` operator for propagation
- Create domain-specific error types with `thiserror`
- Don't use `unwrap()` in production code

### 2. Async Code
- Use `async-trait` for trait methods
- Don't block in async functions
- Use `tokio::spawn` for parallel tasks
- Consider using actors for stateful components

### 3. Safety
- Minimize `unsafe` code
- Document all `unsafe` blocks
- Use `#[deny(unsafe_code)]` where possible
- Leverage type system for correctness

### 4. Performance
- Benchmark early and often
- Profile before optimizing
- Use zero-copy where possible
- Consider lock-free data structures for hot paths

### 5. Testing
- Write unit tests for all blocks
- Use property-based testing for algorithms
- Integration tests for full workflows
- Benchmark critical paths

---

## Next Steps

1. **Set up project structure** (workspace, dependencies)
2. **Implement core traits** (Block, Port, Parameter)
3. **Build registry** (registration, lookup, validation)
4. **Create first block** (HeapFileBlock)
5. **Add tests and benchmarks**
6. **Implement execution engine**
7. **Build more blocks**

---

## Resources

### Rust Database Projects
- [TiKV](https://github.com/tikv/tikv) - Distributed key-value store
- [Sled](https://github.com/spacejam/sled) - Embedded database
- [Polars](https://github.com/pola-rs/polars) - DataFrame library
- [Databend](https://github.com/datafuselabs/databend) - Cloud warehouse

### Learning Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Database Internals Book](https://www.databass.dev/) by Alex Petrov

### Crates to Explore
- `parking_lot` - Better locks
- `crossbeam` - Concurrency utilities
- `rayon` - Data parallelism
- `bytes` - Zero-copy byte buffers
- `serde` - Serialization
- `criterion` - Benchmarking
