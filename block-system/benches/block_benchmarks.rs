//! Criterion benchmarks for DB Simulator block operations.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks measure the core operations of each block type,
//! including configuration comparisons (e.g., B-tree fanout, page sizes).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use std::collections::HashMap;

use block_system::categories::storage::heap_file::HeapFileBlock;
use block_system::categories::storage::lsm_tree::LSMTreeBlock;
use block_system::categories::index::btree::BTreeIndexBlock;
use block_system::categories::index::hash_index::HashIndexBlock;
use block_system::categories::buffer::lru_buffer::LRUBufferBlock;
use block_system::categories::concurrency::mvcc::MVCCBlock;
use block_system::categories::concurrency::row_lock::{RowLockBlock, LockMode};
use block_system::categories::transaction::wal::{WALBlock, LogRecordType};
use block_system::core::block::Block;
use block_system::core::port::Record;
use block_system::core::parameter::ParameterValue;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_record(id: i64, name: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), id).unwrap();
    r.insert("name".into(), name).unwrap();
    r
}

// ---------------------------------------------------------------------------
// Heap File Benchmarks
// ---------------------------------------------------------------------------

fn bench_heap_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("heap_insert");

    for count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let mut heap = HeapFileBlock::new();
                    for i in 0..n {
                        heap.insert(make_record(i, "bench"));
                    }
                    black_box(heap.scan().len())
                });
            },
        );
    }
    group.finish();
}

fn bench_heap_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("heap_scan");

    for count in [100, 1_000, 10_000] {
        let mut heap = HeapFileBlock::new();
        for i in 0..count {
            heap.insert(make_record(i, "bench"));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, _| {
                b.iter(|| black_box(heap.scan().len()));
            },
        );
    }
    group.finish();
}

fn bench_heap_point_lookup(c: &mut Criterion) {
    let mut heap = HeapFileBlock::new();
    let mut tids = Vec::new();
    for i in 0..1_000 {
        tids.push(heap.insert(make_record(i, "bench")));
    }

    c.bench_function("heap_point_lookup_1k", |b| {
        let mut idx = 0usize;
        b.iter(|| {
            let tid = tids[idx % tids.len()];
            idx += 1;
            black_box(heap.get(tid))
        });
    });
}

// ---------------------------------------------------------------------------
// B-tree Benchmarks
// ---------------------------------------------------------------------------

fn bench_btree_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insert");

    for count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let mut tree = BTreeIndexBlock::new();
                    for i in 0..n {
                        let tid = block_system::categories::TupleId::new(0, i as usize);
                        tree.insert_key(json!(i), tid);
                    }
                    black_box(())
                });
            },
        );
    }
    group.finish();
}

fn bench_btree_lookup(c: &mut Criterion) {
    let mut tree = BTreeIndexBlock::new();
    for i in 0..10_000i64 {
        let tid = block_system::categories::TupleId::new(0, i as usize);
        tree.insert_key(json!(i), tid);
    }

    c.bench_function("btree_lookup_10k", |b| {
        let mut idx = 0i64;
        b.iter(|| {
            let key = json!(idx % 10_000);
            idx += 1;
            black_box(tree.lookup(&key))
        });
    });
}

fn bench_btree_range_scan(c: &mut Criterion) {
    let mut tree = BTreeIndexBlock::new();
    for i in 0..10_000i64 {
        let tid = block_system::categories::TupleId::new(0, i as usize);
        tree.insert_key(json!(i), tid);
    }

    let mut group = c.benchmark_group("btree_range_scan");
    for range_size in [10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(range_size),
            &range_size,
            |b, &size| {
                b.iter(|| {
                    let start = json!(5_000);
                    let end = json!(5_000 + size);
                    black_box(tree.range_scan(&start, &end))
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Hash Index Benchmarks
// ---------------------------------------------------------------------------

fn bench_hash_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_index_insert");

    for count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let mut idx = HashIndexBlock::new();
                    for i in 0..n {
                        let tid = block_system::categories::TupleId::new(0, i as usize);
                        idx.insert_key(json!(i), tid);
                    }
                    black_box(())
                });
            },
        );
    }
    group.finish();
}

fn bench_hash_lookup(c: &mut Criterion) {
    let mut idx = HashIndexBlock::new();
    for i in 0..10_000i64 {
        let tid = block_system::categories::TupleId::new(0, i as usize);
        idx.insert_key(json!(i), tid);
    }

    c.bench_function("hash_lookup_10k", |b| {
        let mut i = 0i64;
        b.iter(|| {
            let key = json!(i % 10_000);
            i += 1;
            black_box(idx.lookup(&key))
        });
    });
}

// ---------------------------------------------------------------------------
// B-tree vs Hash Index Comparison
// ---------------------------------------------------------------------------

fn bench_index_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_point_lookup_comparison");

    // B-tree
    let mut tree = BTreeIndexBlock::new();
    for i in 0..10_000i64 {
        let tid = block_system::categories::TupleId::new(0, i as usize);
        tree.insert_key(json!(i), tid);
    }

    // Hash
    let mut hash = HashIndexBlock::new();
    for i in 0..10_000i64 {
        let tid = block_system::categories::TupleId::new(0, i as usize);
        hash.insert_key(json!(i), tid);
    }

    group.bench_function("btree", |b| {
        let mut i = 0i64;
        b.iter(|| {
            let key = json!(i % 10_000);
            i += 1;
            black_box(tree.lookup(&key))
        });
    });

    group.bench_function("hash", |b| {
        let mut i = 0i64;
        b.iter(|| {
            let key = json!(i % 10_000);
            i += 1;
            black_box(hash.lookup(&key))
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// LRU Buffer Benchmarks
// ---------------------------------------------------------------------------

fn make_lru_pool(size: i64) -> LRUBufferBlock {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut pool = LRUBufferBlock::new();
    let mut params = HashMap::new();
    params.insert("size".into(), ParameterValue::Integer(size));
    rt.block_on(pool.initialize(params)).unwrap();
    pool
}

fn bench_lru_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_sequential_access");

    for cache_size in [32i64, 128, 512] {
        group.bench_with_input(
            BenchmarkId::from_parameter(cache_size),
            &cache_size,
            |b, &size| {
                b.iter(|| {
                    let mut pool = make_lru_pool(size);
                    for page in 0..1_000 {
                        pool.get_page(page);
                    }
                    black_box(pool.hit_rate_pct())
                });
            },
        );
    }
    group.finish();
}

fn bench_lru_temporal_locality(c: &mut Criterion) {
    c.bench_function("lru_temporal_locality_128", |b| {
        b.iter(|| {
            let mut pool = make_lru_pool(128);
            // Simulate temporal locality: repeatedly access hot set of 32 pages
            for round in 0..100 {
                for page in 0..32 {
                    pool.get_page(page);
                }
                // Occasional cold access
                pool.get_page(1_000 + round);
            }
            black_box(pool.hit_rate_pct())
        });
    });
}

// ---------------------------------------------------------------------------
// LSM Tree Benchmarks
// ---------------------------------------------------------------------------

fn bench_lsm_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_write");

    for count in [100, 1_000, 5_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let mut lsm = LSMTreeBlock::new();
                    for i in 0..n {
                        lsm.put(format!("key_{i:06}"), json!(i));
                    }
                    black_box(lsm.write_amplification())
                });
            },
        );
    }
    group.finish();
}

fn bench_lsm_read(c: &mut Criterion) {
    let mut lsm = LSMTreeBlock::new();
    for i in 0..5_000 {
        lsm.put(format!("key_{i:06}"), json!(i));
    }

    c.bench_function("lsm_read_5k", |b| {
        let mut i = 0;
        b.iter(|| {
            let key = format!("key_{:06}", i % 5_000);
            i += 1;
            black_box(lsm.get(&key))
        });
    });
}

// ---------------------------------------------------------------------------
// MVCC Benchmarks
// ---------------------------------------------------------------------------

fn bench_mvcc_write_read(c: &mut Criterion) {
    c.bench_function("mvcc_100_txn_write_read", |b| {
        b.iter(|| {
            let mut mvcc = MVCCBlock::new();
            // 100 transactions each writing 1 key then reading it
            for _ in 0..100 {
                let ts = mvcc.begin_txn();
                mvcc.write(ts, "key1", json!("value"));
                let val = mvcc.read(ts, "key1");
                mvcc.commit(ts);
                black_box(val);
            }
        });
    });
}

fn bench_mvcc_concurrent_keys(c: &mut Criterion) {
    c.bench_function("mvcc_1000_keys", |b| {
        b.iter(|| {
            let mut mvcc = MVCCBlock::new();
            let ts = mvcc.begin_txn();
            for i in 0..1_000 {
                mvcc.write(ts, &format!("key_{i}"), json!(i));
            }
            mvcc.commit(ts);

            let snap = mvcc.begin_txn();
            for i in 0..1_000 {
                black_box(mvcc.read(snap, &format!("key_{i}")));
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Row Lock Benchmarks
// ---------------------------------------------------------------------------

fn bench_row_lock_acquire_release(c: &mut Criterion) {
    c.bench_function("row_lock_100_txn", |b| {
        b.iter(|| {
            let mut lock_mgr = RowLockBlock::new();
            for _ in 0..100 {
                let txn = lock_mgr.begin_txn();
                lock_mgr.acquire_lock(txn, "resource_a", LockMode::Shared);
                lock_mgr.commit(txn);
            }
        });
    });
}

// ---------------------------------------------------------------------------
// WAL Benchmarks
// ---------------------------------------------------------------------------

fn bench_wal_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal_append");

    for count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let mut wal = WALBlock::new();
                    for _ in 0..n {
                        wal.append(LogRecordType::Insert, 128);
                    }
                    black_box(())
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Storage Engine Comparison: Heap vs LSM for mixed workloads
// ---------------------------------------------------------------------------

fn bench_storage_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_insert_1000");

    group.bench_function("heap", |b| {
        b.iter(|| {
            let mut heap = HeapFileBlock::new();
            for i in 0..1_000i64 {
                heap.insert(make_record(i, "bench"));
            }
            black_box(heap.scan().len())
        });
    });

    group.bench_function("lsm", |b| {
        b.iter(|| {
            let mut lsm = LSMTreeBlock::new();
            for i in 0..1_000 {
                lsm.put(format!("key_{i:06}"), json!({"id": i, "name": "bench"}));
            }
            black_box(lsm.get("key_000500"))
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

criterion_group!(
    storage_benches,
    bench_heap_insert,
    bench_heap_scan,
    bench_heap_point_lookup,
    bench_lsm_write,
    bench_lsm_read,
    bench_storage_comparison,
);

criterion_group!(
    index_benches,
    bench_btree_insert,
    bench_btree_lookup,
    bench_btree_range_scan,
    bench_hash_insert,
    bench_hash_lookup,
    bench_index_comparison,
);

criterion_group!(
    buffer_benches,
    bench_lru_sequential,
    bench_lru_temporal_locality,
);

criterion_group!(
    concurrency_benches,
    bench_mvcc_write_read,
    bench_mvcc_concurrent_keys,
    bench_row_lock_acquire_release,
);

criterion_group!(
    transaction_benches,
    bench_wal_append,
);

criterion_main!(
    storage_benches,
    index_benches,
    buffer_benches,
    concurrency_benches,
    transaction_benches,
);
