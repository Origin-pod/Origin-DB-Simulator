//! Integration tests: wiring HeapFile, BTreeIndex, and LRUBuffer together
//!
//! These tests simulate a realistic pipeline:
//!   Records → HeapFile (storage) → BTreeIndex (indexing)
//!                                → LRUBuffer (cached reads)

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::categories::buffer::LRUBufferBlock;
    use crate::categories::index::BTreeIndexBlock;
    use crate::categories::storage::HeapFileBlock;
    use crate::categories::TupleId;
    use crate::core::block::{Block, ExecutionContext};
    use crate::core::metrics::{Logger, MetricsCollector, StorageContext};
    use crate::core::parameter::ParameterValue;
    use crate::core::port::{PortValue, Record};

    /// Helper: build N records with incrementing ids.
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

    fn make_context(input_port: &str, records: Vec<Record>) -> ExecutionContext {
        let mut inputs = HashMap::new();
        inputs.insert(input_port.into(), PortValue::Stream(records));
        ExecutionContext {
            inputs,
            parameters: HashMap::new(),
            metrics: MetricsCollector::new(),
            logger: Logger::new(),
            storage: StorageContext::new(),
        }
    }

    // ====================================================================
    // Test 1: HeapFile → BTreeIndex pipeline (1000 records)
    // ====================================================================

    #[tokio::test]
    async fn test_heap_to_btree_pipeline() {
        // --- Step 1: Store 1000 records in the heap file ---
        let mut heap = HeapFileBlock::new();
        heap.initialize(HashMap::new()).await.unwrap();

        let records = generate_records(1000);
        let ctx = make_context("records", records);
        let heap_result = heap.execute(ctx).await.unwrap();

        assert!(heap_result.errors.is_empty());
        assert_eq!(heap.live_record_count(), 1000);
        assert!(heap.page_count() >= 1);

        // Output should have 1000 records with _page_id and _slot_id.
        let stored = match heap_result.outputs.get("stored").unwrap() {
            PortValue::Stream(recs) => recs.clone(),
            _ => panic!("Expected Stream output"),
        };
        assert_eq!(stored.len(), 1000);

        // Verify first record has tuple id fields.
        let first = &stored[0];
        assert!(first.data.contains_key("_page_id"));
        assert!(first.data.contains_key("_slot_id"));

        // --- Step 2: Index the stored records with B-tree ---
        let mut btree = BTreeIndexBlock::new();
        let mut btree_params = HashMap::new();
        btree_params.insert("fanout".into(), ParameterValue::Integer(32));
        btree_params.insert("key_column".into(), ParameterValue::String("id".into()));
        btree.initialize(btree_params).await.unwrap();

        let btree_ctx = make_context("records", stored);
        let btree_result = btree.execute(btree_ctx).await.unwrap();

        assert!(btree_result.errors.is_empty());
        assert_eq!(btree.key_count(), 1000);

        // Tree depth should be reasonable for 1000 keys with fanout 32.
        let depth = btree.depth();
        assert!(
            depth <= 4,
            "Depth {} too large for 1000 keys fanout 32",
            depth
        );

        // Point lookups should work.
        for i in [0, 42, 500, 999] {
            let tid = btree.lookup(&serde_json::json!(i as i64));
            assert!(tid.is_some(), "Key {} not found in index", i);

            // Cross-check: retrieve the record from the heap.
            let record = heap.get(tid.unwrap());
            assert!(record.is_some(), "TupleId {:?} not found in heap", tid);
            let id_val = record.unwrap().get::<i64>("id").unwrap().unwrap();
            assert_eq!(id_val, i as i64);
        }

        // Range scan should return ordered results.
        let range = btree.range_scan(&serde_json::json!(100), &serde_json::json!(110));
        assert_eq!(range.len(), 11, "Range [100, 110] should have 11 keys");

        // Verify scan results point to valid heap records.
        for (key, tid) in &range {
            let rec = heap.get(*tid).expect("Range scan TID should be valid");
            let rec_id = rec.get::<i64>("id").unwrap().unwrap();
            assert_eq!(serde_json::json!(rec_id), *key);
        }
    }

    // ====================================================================
    // Test 2: HeapFile → LRUBuffer read-through (cached page access)
    // ====================================================================

    #[tokio::test]
    async fn test_heap_to_buffer_pipeline() {
        // --- Step 1: Store records with small pages ---
        let mut heap = HeapFileBlock::new();
        let mut heap_params = HashMap::new();
        heap_params.insert("page_size".into(), ParameterValue::Integer(1024));
        heap_params.insert("fill_factor".into(), ParameterValue::Number(0.8));
        heap.initialize(heap_params).await.unwrap();

        let records = generate_records(200);
        let ctx = make_context("records", records);
        let heap_result = heap.execute(ctx).await.unwrap();
        assert!(heap_result.errors.is_empty());

        let stored = match heap_result.outputs.get("stored").unwrap() {
            PortValue::Stream(recs) => recs.clone(),
            _ => panic!("Expected Stream output"),
        };

        // Should have multiple pages with small page size.
        assert!(
            heap.page_count() > 1,
            "Expected multiple pages, got {}",
            heap.page_count()
        );

        // --- Step 2: Read pages through LRU buffer ---
        let mut buffer = LRUBufferBlock::new();
        let mut buf_params = HashMap::new();
        buf_params.insert("size".into(), ParameterValue::Integer(5)); // Tiny pool.
        buf_params.insert("page_size".into(), ParameterValue::Integer(1024));
        buffer.initialize(buf_params).await.unwrap();

        let buf_ctx = make_context("requests", stored.clone());
        let buf_result = buffer.execute(buf_ctx).await.unwrap();

        assert!(buf_result.errors.is_empty());

        let hits = *buf_result.metrics.get("cache_hits").unwrap();
        let misses = *buf_result.metrics.get("cache_misses").unwrap();
        let total = hits + misses;
        assert_eq!(total, 200.0, "Should have processed 200 page requests");

        // With 200 records on >1 pages, some hits are expected since
        // consecutive records share the same page.
        assert!(hits > 0.0, "Should have some cache hits from shared pages");

        // --- Step 3: Re-read the same pages — should be mostly hits ---
        let buf_ctx2 = make_context("requests", stored);
        let buf_result2 = buffer.execute(buf_ctx2).await.unwrap();

        let hits2 = *buf_result2.metrics.get("cache_hits").unwrap();
        // Cumulative hits should be higher (pool remembers from first pass).
        assert!(hits2 > hits, "Second pass should have more cumulative hits");
    }

    // ====================================================================
    // Test 3: Full pipeline — HeapFile → BTreeIndex + LRUBuffer
    // ====================================================================

    #[tokio::test]
    async fn test_full_pipeline_with_metrics() {
        // --- Storage ---
        let mut heap = HeapFileBlock::new();
        heap.initialize(HashMap::new()).await.unwrap();

        let records = generate_records(500);
        let store_ctx = make_context("records", records);
        let store_result = heap.execute(store_ctx).await.unwrap();

        let stored = match store_result.outputs.get("stored").unwrap() {
            PortValue::Stream(recs) => recs.clone(),
            _ => panic!("Expected Stream"),
        };

        // --- Index ---
        let mut btree = BTreeIndexBlock::new();
        btree.initialize(HashMap::new()).await.unwrap();

        let idx_ctx = make_context("records", stored.clone());
        let idx_result = btree.execute(idx_ctx).await.unwrap();

        // --- Buffer ---
        let mut buffer = LRUBufferBlock::new();
        let mut buf_params = HashMap::new();
        buf_params.insert("size".into(), ParameterValue::Integer(64));
        buffer.initialize(buf_params).await.unwrap();

        let buf_ctx = make_context("requests", stored);
        let buf_result = buffer.execute(buf_ctx).await.unwrap();

        // --- Verify metrics from all three ---
        // Heap
        assert_eq!(heap.live_record_count(), 500);
        assert!(heap.page_count() >= 1);
        assert_eq!(heap.fragmentation_pct(), 0.0);

        // BTree
        assert_eq!(btree.key_count(), 500);
        assert!(btree.depth() >= 1);
        assert_eq!(*idx_result.metrics.get("total_keys").unwrap(), 500.0);

        // Buffer
        let hit_rate = *buf_result.metrics.get("hit_rate_pct").unwrap();
        assert!(hit_rate >= 0.0 && hit_rate <= 100.0);
        assert_eq!(
            *buf_result.metrics.get("cache_hits").unwrap()
                + *buf_result.metrics.get("cache_misses").unwrap(),
            500.0
        );

        // --- Delete some records and verify fragmentation ---
        for i in 0..50 {
            heap.delete(TupleId::new(0, i));
        }
        assert_eq!(heap.live_record_count(), 450);
        assert!(heap.fragmentation_pct() > 0.0);
    }

    // ====================================================================
    // Test 4: BTree point-lookup cross-validated with HeapFile
    // ====================================================================

    #[tokio::test]
    async fn test_index_lookup_cross_validation() {
        let mut heap = HeapFileBlock::new();

        // Insert records directly (not through execute).
        let mut tids = Vec::new();
        for i in 0..100 {
            let mut r = Record::new();
            r.insert("id".into(), i as i64).unwrap();
            r.insert("value".into(), format!("val_{}", i)).unwrap();
            tids.push(heap.insert(r));
        }

        // Build index from the TupleIds.
        let mut btree = BTreeIndexBlock::new();
        let mut params = HashMap::new();
        params.insert("fanout".into(), ParameterValue::Integer(8));
        btree.initialize(params).await.unwrap();

        for (i, tid) in tids.iter().enumerate() {
            btree
                .insert_key(serde_json::json!(i as i64), *tid)
                .unwrap();
        }

        // Every lookup through the index should return a valid heap record.
        for i in 0..100 {
            let tid = btree
                .lookup(&serde_json::json!(i as i64))
                .expect("Key should exist");
            let record = heap.get(tid).expect("TID should point to valid record");
            assert_eq!(record.get::<i64>("id").unwrap().unwrap(), i as i64);
            assert_eq!(
                record.get::<String>("value").unwrap().unwrap(),
                format!("val_{}", i)
            );
        }
    }

    // ====================================================================
    // Test 5: Buffer eviction under sequential scan workload
    // ====================================================================

    #[tokio::test]
    async fn test_buffer_under_sequential_scan() {
        let mut buffer = LRUBufferBlock::new();
        let mut params = HashMap::new();
        params.insert("size".into(), ParameterValue::Integer(10));
        buffer.initialize(params).await.unwrap();

        // Simulate a sequential scan touching 100 different pages.
        let records: Vec<Record> = (0..100usize)
            .map(|i| {
                let mut r = Record::new();
                r.insert("_page_id".into(), i).unwrap();
                r
            })
            .collect();

        let ctx = make_context("requests", records);
        let result = buffer.execute(ctx).await.unwrap();

        let hits = *result.metrics.get("cache_hits").unwrap();
        let misses = *result.metrics.get("cache_misses").unwrap();
        let evictions = *result.metrics.get("evictions").unwrap();

        // All unique pages — sequential scan thrashes the small cache.
        assert_eq!(misses, 100.0);
        assert_eq!(hits, 0.0);
        assert_eq!(evictions, 90.0); // 100 - 10 capacity
        assert_eq!(buffer.current_size(), 10);

        // Only the last 10 pages should be cached.
        for page_id in 90..100 {
            assert!(buffer.contains(page_id));
        }
        for page_id in 0..90 {
            assert!(!buffer.contains(page_id));
        }
    }
}
