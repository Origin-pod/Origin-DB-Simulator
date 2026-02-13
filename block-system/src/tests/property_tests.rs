//! Property-based tests using proptest.
//!
//! These tests verify invariants that must hold for *any* input, catching
//! edge cases that hand-written tests miss.

use proptest::prelude::*;
use serde_json::json;

use crate::categories::storage::heap_file::HeapFileBlock;
use crate::categories::storage::lsm_tree::LSMTreeBlock;
use crate::categories::index::btree::BTreeIndexBlock;
use crate::categories::index::hash_index::HashIndexBlock;
use crate::categories::buffer::lru_buffer::LRUBufferBlock;
use crate::categories::concurrency::mvcc::MVCCBlock;
use crate::categories::transaction::wal::{WALBlock, LogRecordType};
use crate::categories::TupleId;
use crate::core::port::Record;

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
// Heap File Properties
// ---------------------------------------------------------------------------

proptest! {
    /// Heap file never loses live records: after inserting N records,
    /// a scan must return exactly N records.
    #[test]
    fn heap_insert_then_scan_returns_all(count in 1..500u32) {
        let mut heap = HeapFileBlock::new();
        for i in 0..count {
            heap.insert(make_record(i as i64, "prop"));
        }
        let live = heap.scan();
        prop_assert_eq!(live.len(), count as usize);
    }

    /// Point lookup always finds an inserted record.
    #[test]
    fn heap_insert_then_get_succeeds(count in 1..200u32) {
        let mut heap = HeapFileBlock::new();
        let mut tids = Vec::new();
        for i in 0..count {
            tids.push(heap.insert(make_record(i as i64, "prop")));
        }
        for tid in &tids {
            prop_assert!(heap.get(*tid).is_some());
        }
    }

    /// Deleting a record removes it from scans and point lookups.
    #[test]
    fn heap_delete_removes_record(count in 2..200u32, del_idx in 0..200u32) {
        let del_idx = del_idx % count; // normalize
        let mut heap = HeapFileBlock::new();
        let mut tids = Vec::new();
        for i in 0..count {
            tids.push(heap.insert(make_record(i as i64, "prop")));
        }

        let deleted_tid = tids[del_idx as usize];
        prop_assert!(heap.delete(deleted_tid));
        prop_assert!(heap.get(deleted_tid).is_none());

        let live = heap.scan();
        prop_assert_eq!(live.len(), (count - 1) as usize);
    }
}

// ---------------------------------------------------------------------------
// B-tree Index Properties
// ---------------------------------------------------------------------------

proptest! {
    /// B-tree lookup always finds a key that was inserted.
    #[test]
    fn btree_insert_then_lookup_succeeds(count in 1..500u32) {
        let mut tree = BTreeIndexBlock::new();
        for i in 0..count {
            let tid = TupleId::new(0, i as usize);
            let _ = tree.insert_key(json!(i), tid);
        }
        for i in 0..count {
            let result = tree.lookup(&json!(i));
            prop_assert!(result.is_some(), "key {} not found", i);
        }
    }

    /// Range scan returns results in sorted order.
    #[test]
    fn btree_range_scan_is_sorted(
        count in 1..300u32,
        start in 0..100u32,
        range in 1..50u32,
    ) {
        let mut tree = BTreeIndexBlock::new();
        for i in 0..count {
            let tid = TupleId::new(0, i as usize);
            let _ = tree.insert_key(json!(i), tid);
        }

        let end = start + range;
        let results = tree.range_scan(&json!(start), &json!(end));

        // Verify results are in ascending order by checking tuple IDs
        for window in results.windows(2) {
            let (_, prev_tid) = &window[0];
            let (_, next_tid) = &window[1];
            prop_assert!(
                prev_tid.slot_id < next_tid.slot_id,
                "Range scan not sorted: slot {} >= {}",
                prev_tid.slot_id,
                next_tid.slot_id,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Hash Index Properties
// ---------------------------------------------------------------------------

proptest! {
    /// Hash index point lookup always finds an inserted key.
    #[test]
    fn hash_insert_then_lookup_succeeds(count in 1..500u32) {
        let mut idx = HashIndexBlock::new();
        for i in 0..count {
            let tid = TupleId::new(0, i as usize);
            idx.insert_key(json!(i), tid);
        }
        for i in 0..count {
            let result = idx.lookup(&json!(i));
            prop_assert!(result.is_some(), "key {} not found in hash index", i);
        }
    }

    /// Hash index returns the correct TupleId for each key.
    #[test]
    fn hash_lookup_returns_correct_tid(count in 1..300u32) {
        let mut idx = HashIndexBlock::new();
        for i in 0..count {
            let tid = TupleId::new(i as usize, i as usize);
            idx.insert_key(json!(i), tid);
        }
        for i in 0..count {
            let result = idx.lookup(&json!(i)).unwrap();
            prop_assert_eq!(result.page_id, i as usize);
            prop_assert_eq!(result.slot_id, i as usize);
        }
    }
}

// ---------------------------------------------------------------------------
// LRU Buffer Properties
// ---------------------------------------------------------------------------

proptest! {
    /// LRU buffer never exceeds its capacity.
    #[test]
    fn lru_never_exceeds_capacity(
        capacity in 1..64usize,
        accesses in prop::collection::vec(0..1000usize, 1..500),
    ) {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = capacity;

        for page_id in &accesses {
            pool.get_page(*page_id);
            prop_assert!(
                pool.current_size() <= capacity,
                "Pool size {} exceeded capacity {}",
                pool.current_size(),
                capacity,
            );
        }
    }

    /// Hit rate is between 0% and 100%.
    #[test]
    fn lru_hit_rate_in_range(
        accesses in prop::collection::vec(0..100usize, 1..200),
    ) {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 16;

        for page_id in &accesses {
            pool.get_page(*page_id);
        }

        let rate = pool.hit_rate_pct();
        prop_assert!(rate >= 0.0 && rate <= 100.0, "hit rate out of range: {}", rate);
    }

    /// Accessing the same page twice always results in a hit on the second access.
    #[test]
    fn lru_repeated_access_is_hit(page_id in 0..1000usize) {
        let mut pool = LRUBufferBlock::new();
        pool.capacity = 16;

        let first = pool.get_page(page_id);
        let second = pool.get_page(page_id);
        prop_assert!(!first, "First access should be a miss");
        prop_assert!(second, "Second access should be a hit");
    }
}

// ---------------------------------------------------------------------------
// LSM Tree Properties
// ---------------------------------------------------------------------------

proptest! {
    /// LSM tree always returns the latest written value for a key.
    #[test]
    fn lsm_read_returns_latest_write(
        updates in 1..10u32,
    ) {
        let mut lsm = LSMTreeBlock::new();
        let key = "test_key".to_string();

        for i in 0..updates {
            lsm.put(key.clone(), json!(i));
        }

        let result = lsm.get(&key);
        prop_assert_eq!(result, Some(json!(updates - 1)));
    }

    /// LSM tree returns None for keys never written.
    #[test]
    fn lsm_missing_key_returns_none(count in 1..100u32) {
        let mut lsm = LSMTreeBlock::new();
        for i in 0..count {
            lsm.put(format!("key_{i}"), json!(i));
        }
        let result = lsm.get("nonexistent_key");
        prop_assert_eq!(result, None);
    }
}

// ---------------------------------------------------------------------------
// MVCC Properties
// ---------------------------------------------------------------------------

proptest! {
    /// Snapshot isolation: a transaction always sees the value as of its
    /// start time, regardless of later commits.
    #[test]
    fn mvcc_snapshot_isolation(writes in 1..20u32) {
        let mut mvcc = MVCCBlock::new();
        let key = "shared_key";

        // Transaction 1 writes initial value and commits
        let ts1 = mvcc.begin_txn();
        mvcc.write(ts1, key, json!("initial"));
        mvcc.commit(ts1);

        // Transaction 2 takes a snapshot (reads "initial")
        let ts2 = mvcc.begin_txn();
        let snapshot_val = mvcc.read(ts2, key);
        prop_assert_eq!(snapshot_val, Some(json!("initial")));

        // Transaction 3 writes new values and commits
        for i in 0..writes {
            let ts3 = mvcc.begin_txn();
            mvcc.write(ts3, key, json!(format!("update_{i}")));
            mvcc.commit(ts3);
        }

        // Transaction 2 should still see "initial" (snapshot isolation)
        let still_initial = mvcc.read(ts2, key);
        prop_assert_eq!(still_initial, Some(json!("initial")));
    }

    /// A transaction always sees its own writes.
    #[test]
    fn mvcc_read_own_writes(value in 0..1000i64) {
        let mut mvcc = MVCCBlock::new();
        let ts = mvcc.begin_txn();
        mvcc.write(ts, "mykey", json!(value));
        let result = mvcc.read(ts, "mykey");
        prop_assert_eq!(result, Some(json!(value)));
    }

    /// Committed data is visible to new transactions.
    #[test]
    fn mvcc_committed_visible_to_new_txn(value in 0..1000i64) {
        let mut mvcc = MVCCBlock::new();
        let ts1 = mvcc.begin_txn();
        mvcc.write(ts1, "key", json!(value));
        mvcc.commit(ts1);

        let ts2 = mvcc.begin_txn();
        let result = mvcc.read(ts2, "key");
        prop_assert_eq!(result, Some(json!(value)));
    }
}

// ---------------------------------------------------------------------------
// WAL Properties
// ---------------------------------------------------------------------------

proptest! {
    /// WAL LSNs are strictly monotonically increasing.
    #[test]
    fn wal_lsn_strictly_increasing(count in 2..500u32) {
        let mut wal = WALBlock::new();
        let mut last_lsn = 0u64;

        for _ in 0..count {
            let lsn = wal.append(LogRecordType::Insert, 64);
            prop_assert!(lsn > last_lsn, "LSN {} not > previous {}", lsn, last_lsn);
            last_lsn = lsn;
        }
    }

    /// WAL current_lsn is always >= the number of user appends
    /// (checkpoints may add extra internal records).
    #[test]
    fn wal_tracks_all_appends(count in 1..500u32) {
        let mut wal = WALBlock::new();
        let mut lsns = Vec::new();
        for _ in 0..count {
            lsns.push(wal.append(LogRecordType::Insert, 64));
        }

        // Every LSN we received must be unique
        let unique_count = {
            let mut sorted = lsns.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        prop_assert_eq!(unique_count, count as usize, "LSNs must be unique");

        // current_lsn must be at least as large as our last append
        let last = *lsns.last().unwrap();
        prop_assert!(wal.current_lsn() >= last, "current_lsn less than last append LSN");
    }
}

// ---------------------------------------------------------------------------
// Cross-block integration properties
// ---------------------------------------------------------------------------

proptest! {
    /// Records inserted into heap file and indexed by B-tree are always
    /// retrievable via the index.
    #[test]
    fn heap_btree_round_trip(count in 1..200u32) {
        let mut heap = HeapFileBlock::new();
        let mut tree = BTreeIndexBlock::new();

        for i in 0..count {
            let tid = heap.insert(make_record(i as i64, "prop"));
            let _ = tree.insert_key(json!(i), tid);
        }

        // Every key in the index should point to a valid heap record
        for i in 0..count {
            let tid = tree.lookup(&json!(i));
            prop_assert!(tid.is_some(), "Key {} not in index", i);
            let record = heap.get(tid.unwrap());
            prop_assert!(record.is_some(), "TupleId for key {} not in heap", i);
        }
    }
}
