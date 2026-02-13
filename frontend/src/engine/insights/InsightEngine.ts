// ---------------------------------------------------------------------------
// Insight Engine — Turns execution metrics into educational narratives
//
// Pure class. Takes ExecutionResult + canvas state + BLOCK_REGISTRY and
// produces Insight[] using deterministic rules. No side effects.
// ---------------------------------------------------------------------------

import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { ExecutionResult, BlockMetrics } from '@/engine/types';
import { getBlockDefinition } from '@/types/blocks';
import type { Insight } from './types';

// Block category inference (same as MetricsDashboard)
type Category = 'storage' | 'index' | 'buffer' | 'execution' | 'concurrency' | 'transaction';

const BLOCK_CATEGORY_MAP: Record<string, Category> = {
  schema_definition: 'storage',
  heap_storage: 'storage',
  clustered_storage: 'storage',
  columnar_storage: 'storage',
  lsm_tree: 'storage',
  btree_index: 'index',
  hash_index: 'index',
  covering_index: 'index',
  lru_buffer: 'buffer',
  clock_buffer: 'buffer',
  sequential_scan: 'execution',
  index_scan: 'execution',
  filter: 'execution',
  sort: 'execution',
  hash_join: 'execution',
  row_lock: 'concurrency',
  mvcc: 'concurrency',
  wal: 'transaction',
  bloom_filter: 'index',
  statistics_collector: 'execution',
  hash_partitioner: 'storage',
  replication: 'concurrency',
  dictionary_encoding: 'storage',
};

function categoryOf(blockType: string): Category {
  return BLOCK_CATEGORY_MAP[blockType] ?? 'storage';
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

export class InsightEngine {
  private nodes: Node<BlockNodeData>[];
  private result: ExecutionResult;
  private blockTypes: Set<string>;
  private blocksByType: Map<string, Node<BlockNodeData>[]>;

  constructor(
    nodes: Node<BlockNodeData>[],
    _edges: Edge[],
    result: ExecutionResult,
  ) {
    this.nodes = nodes;
    this.result = result;

    this.blockTypes = new Set(nodes.map((n) => (n.data as BlockNodeData).blockType));
    this.blocksByType = new Map();
    for (const n of nodes) {
      const bt = (n.data as BlockNodeData).blockType;
      const arr = this.blocksByType.get(bt) ?? [];
      arr.push(n);
      this.blocksByType.set(bt, arr);
    }
  }

  analyze(): Insight[] {
    const insights: Insight[] = [];
    let idCounter = 0;
    const nextId = () => `insight-${++idCounter}`;

    const sorted = [...this.result.blockMetrics].sort(
      (a, b) => b.percentage - a.percentage,
    );
    if (sorted.length === 0) return insights;

    // ------ Rule 1: Storage bottleneck without index ------
    this.checkStorageBottleneck(sorted, insights, nextId);

    // ------ Rule 2: Low cache hit rate ------
    this.checkLowCacheHitRate(insights, nextId);

    // ------ Rule 3: Missing index opportunity ------
    this.checkMissingIndex(sorted, insights, nextId);

    // ------ Rule 4: B-tree fanout / depth insight ------
    this.checkBtreeDepth(insights, nextId);

    // ------ Rule 5: LSM write/read tradeoff ------
    this.checkLsmTradeoff(insights, nextId);

    // ------ Rule 6: High tail latency ------
    this.checkTailLatency(insights, nextId);

    // ------ Rule 7: Tradeoff confirmation from docs ------
    this.checkTradeoffConfirmation(sorted, insights, nextId);

    // ------ Rule 8: Sequential scan explanation ------
    this.checkSequentialScan(sorted, insights, nextId);

    // ------ Rule 9: LSM without Bloom filter ------
    this.checkLsmWithoutBloom(insights, nextId);

    // ------ Rule 10: Bloom filter effectiveness ------
    this.checkBloomFilterEffectiveness(insights, nextId);

    // ------ Rule 11: High replication + strong consistency ------
    this.checkReplicationConsistency(insights, nextId);

    // ------ Rule 12: Hash partitioner distribution ------
    this.checkPartitionDistribution(insights, nextId);

    // ------ Rule 13: Dictionary encoding compression ------
    this.checkDictionaryCompression(insights, nextId);

    // ------ Rule 14: Clock buffer vs LRU explanation ------
    this.checkClockBuffer(insights, nextId);

    return insights;
  }

  // -----------------------------------------------------------------------
  // Rule implementations
  // -----------------------------------------------------------------------

  private checkStorageBottleneck(
    sorted: BlockMetrics[],
    insights: Insight[],
    nextId: () => string,
  ) {
    const top = sorted[0];
    if (top.percentage <= 40) return;
    if (categoryOf(top.blockType) !== 'storage') return;

    const hasIndex = this.blockTypes.has('btree_index') ||
      this.blockTypes.has('hash_index') ||
      this.blockTypes.has('covering_index');

    const def = getBlockDefinition(top.blockType);
    const complexity = def?.documentation?.complexity;

    if (!hasIndex) {
      const totalOps = this.result.metrics.totalOperations;
      insights.push({
        id: nextId(),
        type: 'bottleneck',
        severity: 'important',
        title: `${top.blockName} Is Scanning Every Record`,
        explanation: `Without an index, every read operation must scan all records sequentially${complexity ? ` with ${complexity.time} time complexity` : ''}. ` +
          `This block consumed ${top.percentage.toFixed(0)}% of total execution time across ${fmtNum(totalOps)} operations.`,
        whyItMatters:
          'This is the single biggest performance factor in your design. ' +
          'Real databases like PostgreSQL create B-tree indexes by default on primary keys to avoid exactly this problem.',
        suggestion: 'Add a B-tree index to reduce lookups from O(n) to O(log n). ' +
          'For point lookups, a hash index gives O(1) access.',
        learnMore: { blockType: top.blockType, section: 'algorithm' },
        realWorldExample: 'PostgreSQL creates a B-tree index on every primary key. ' +
          'Without it, a SELECT on a 1M row table reads every single page.',
      });
    } else {
      insights.push({
        id: nextId(),
        type: 'bottleneck',
        severity: 'suggestion',
        title: `Storage Still Dominates — ${top.blockName} at ${top.percentage.toFixed(0)}%`,
        explanation: `Even with indexing, storage consumed the most time. ` +
          `This often means the workload has many writes (which bypass indexes) ` +
          `or the buffer pool is too small to cache hot pages.`,
        whyItMatters:
          'In real databases, writes always hit storage. ' +
          'The buffer pool acts as a shield — reads that hit cache skip disk entirely.',
        suggestion: 'Try increasing the buffer pool size, or check if your workload is write-heavy.',
        learnMore: { blockType: top.blockType, section: 'tradeoffs' },
      });
    }
  }

  private checkLowCacheHitRate(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (categoryOf(bm.blockType) !== 'buffer') continue;

      const hitRate = bm.counters['hit_rate_pct'];
      if (hitRate === undefined || hitRate >= 70) continue;

      const hits = bm.counters['cache_hits'] ?? 0;
      const misses = bm.counters['cache_misses'] ?? 0;
      const total = hits + misses;

      // Find buffer size parameter
      const node = this.findNodeByBlockId(bm.blockId);
      const bufferSize = node ? Number((node.data as BlockNodeData).parameters['size'] ?? 128) : 128;

      insights.push({
        id: nextId(),
        type: 'bottleneck',
        severity: 'important',
        title: `Cache Hit Rate Is Only ${hitRate.toFixed(0)}%`,
        explanation: `${bm.blockName} served ${fmtNum(hits)} hits but missed ${fmtNum(misses)} times out of ${fmtNum(total)} requests. ` +
          `With a ${bufferSize}MB buffer, the working set of your data is larger than what fits in memory.`,
        whyItMatters:
          'Every cache miss means a disk read, which is ~100x slower than a memory read. ' +
          'A hit rate below 70% means most operations are bottlenecked on I/O.',
        suggestion: `Increase the buffer pool from ${bufferSize}MB — try doubling it. ` +
          'In PostgreSQL, shared_buffers is typically 25% of total RAM.',
        learnMore: { blockType: bm.blockType, section: 'algorithm' },
        realWorldExample: 'PostgreSQL defaults shared_buffers to 128MB but production systems ' +
          'typically use 8-32GB. A well-tuned buffer pool achieves 99%+ hit rates.',
      });
    }
  }

  private checkMissingIndex(
    sorted: BlockMetrics[],
    insights: Insight[],
    nextId: () => string,
  ) {
    // Already has an index? Skip.
    const hasIndex = this.blockTypes.has('btree_index') ||
      this.blockTypes.has('hash_index') ||
      this.blockTypes.has('covering_index');
    if (hasIndex) return;

    // Must have storage
    const hasStorage = this.blockTypes.has('heap_storage') ||
      this.blockTypes.has('clustered_storage') ||
      this.blockTypes.has('lsm_tree') ||
      this.blockTypes.has('columnar_storage');
    if (!hasStorage) return;

    // Only show if storage isn't already the top bottleneck (avoid duplicate with Rule 1)
    if (sorted.length > 0) {
      const top = sorted[0];
      if (categoryOf(top.blockType) === 'storage' && top.percentage > 40) return;
    }

    insights.push({
      id: nextId(),
      type: 'opportunity',
      severity: 'suggestion',
      title: 'No Index in Design — Consider Adding One',
      explanation:
        'Your design stores and queries data without any index structure. ' +
        'Every lookup requires scanning all records linearly.',
      whyItMatters:
        'Indexes are the most impactful optimization in database design. ' +
        'They trade extra storage and write overhead for dramatically faster reads.',
      suggestion: 'Add a B-tree index for range queries or a hash index for point lookups.',
      learnMore: { blockType: 'btree_index', section: 'useCases' },
      realWorldExample: 'Every production database uses indexes. PostgreSQL, MySQL, and SQLite ' +
        'all create a B-tree index on the primary key automatically.',
    });
  }

  private checkBtreeDepth(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'btree_index') continue;

      const depth = bm.counters['tree_depth'];
      if (depth === undefined) continue;

      const node = this.findNodeByBlockId(bm.blockId);
      const fanout = node ? Number((node.data as BlockNodeData).parameters['fanout'] ?? 128) : 128;
      const splits = bm.counters['splits'] ?? 0;
      const totalOps = this.result.metrics.totalOperations;

      // Calculate what optimal depth would be
      const recordEstimate = Math.max(totalOps, 1000);
      const optimalDepth = Math.ceil(Math.log(recordEstimate) / Math.log(fanout));

      if (splits > 10 && fanout < 64) {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'suggestion',
          title: `B-tree Has ${depth} Levels with ${splits} Splits`,
          explanation:
            `With a fanout of ${fanout}, each internal node holds ${fanout} keys. ` +
            `A depth of ${depth} means every lookup traverses ${depth} nodes. ` +
            `The ${splits} splits indicate the tree is actively growing — more data means more levels.`,
          whyItMatters:
            `Depth directly determines lookup cost: ${depth} levels = ${depth} page reads per lookup. ` +
            `Higher fanout means wider nodes and fewer levels. PostgreSQL's default ~300 fanout ` +
            `keeps even billion-row tables to 3-4 levels.`,
          suggestion: `Increase fanout from ${fanout} — a fanout of 128+ keeps depth low. ` +
            `With fanout 128 and ${fmtNum(recordEstimate)} records, optimal depth is ~${optimalDepth}.`,
          learnMore: { blockType: 'btree_index', section: 'algorithm' },
          realWorldExample:
            'PostgreSQL B-trees use ~300 keys per page. A table with 1 billion rows ' +
            'needs only 4 levels: log₃₀₀(1,000,000,000) ≈ 3.6.',
        });
      } else if (depth >= 1) {
        // General educational insight about the tree
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `B-tree Index: ${depth} Levels, Fanout ${fanout}`,
          explanation:
            `Each lookup traverses ${depth} tree levels (${depth} page reads). ` +
            `With fanout ${fanout}, each internal node branches into ${fanout} children. ` +
            `This gives O(log₍${fanout}₎ n) lookup time.`,
          whyItMatters:
            'B-trees are the most widely used index structure in databases because they ' +
            'maintain sorted order (enabling range queries) with logarithmic lookup cost.',
          learnMore: { blockType: 'btree_index', section: 'complexity' },
        });
      }
    }
  }

  private checkLsmTradeoff(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'lsm_tree') continue;

      const compactions = bm.counters['compactions'] ?? 0;
      const levelsUsed = bm.counters['levels_used'] ?? 0;
      const memtableFlushes = bm.counters['memtable_flushes'] ?? 0;
      const def = getBlockDefinition('lsm_tree');
      const tradeoffs = def?.documentation?.tradeoffs ?? [];

      if (compactions > 0 || memtableFlushes > 0) {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `LSM Tree: ${memtableFlushes} Flushes, ${compactions} Compactions`,
          explanation:
            `The LSM tree wrote data to the in-memory memtable, flushed it ${memtableFlushes} time(s) to disk, ` +
            `and ran ${compactions} background compaction(s) across ${levelsUsed || '?'} levels. ` +
            `This is the write-optimized design in action — writes are fast (append to memtable) ` +
            `but reads may need to check multiple levels.`,
          whyItMatters:
            'This is the fundamental LSM tradeoff: writes are O(1) amortized, but reads are O(L) ' +
            'where L is the number of levels. Compaction merges levels to keep reads fast, ' +
            'at the cost of write amplification.' +
            (tradeoffs.length > 0 ? ` Key tradeoff: "${tradeoffs[0]}"` : ''),
          suggestion:
            'To improve read performance, increase bloom filter bits (reduces false positives) ' +
            'or decrease the level multiplier (fewer levels but more compaction work).',
          learnMore: { blockType: 'lsm_tree', section: 'tradeoffs' },
          realWorldExample:
            'RocksDB (used by CockroachDB, TiKV) and Apache Cassandra both use LSM trees. ' +
            'They tune compaction strategies to balance write throughput vs read latency.',
        });
      }
    }
  }

  private checkTailLatency(
    insights: Insight[],
    nextId: () => string,
  ) {
    const { latency } = this.result.metrics;
    if (latency.p99 <= latency.avg * 5) return;

    const ratio = (latency.p99 / latency.avg).toFixed(0);
    insights.push({
      id: nextId(),
      type: 'bottleneck',
      severity: 'suggestion',
      title: `High Tail Latency — p99 Is ${ratio}x the Average`,
      explanation:
        `Average latency is ${fmtMs(latency.avg)} but p99 is ${fmtMs(latency.p99)}. ` +
        `This means 1% of operations are significantly slower than typical. ` +
        `Common causes: cache misses forcing disk reads, lock contention, or background compaction.`,
      whyItMatters:
        'In real systems, tail latency matters more than averages. ' +
        'If your service handles 1000 requests/sec, 10 users per second see the worst-case latency. ' +
        'Google targets p99 latency in their SLOs, not averages.',
      suggestion:
        'Check if the buffer pool is too small (causing occasional disk reads), ' +
        'or if concurrent operations are causing lock contention.',
    });
  }

  private checkTradeoffConfirmation(
    sorted: BlockMetrics[],
    insights: Insight[],
    nextId: () => string,
  ) {
    // For the top bottleneck block, check if its documentation tradeoffs
    // are actually being observed in the metrics
    const top = sorted[0];
    if (top.percentage <= 20) return;

    const def = getBlockDefinition(top.blockType);
    if (!def?.documentation?.tradeoffs || def.documentation.tradeoffs.length === 0) return;

    // Already covered by more specific rules
    if (top.blockType === 'lsm_tree') return;
    if (categoryOf(top.blockType) === 'storage' && top.percentage > 40) return;

    const firstTradeoff = def.documentation.tradeoffs[0];
    insights.push({
      id: nextId(),
      type: 'explanation',
      severity: 'info',
      title: `Tradeoff in Action: ${top.blockName}`,
      explanation:
        `"${top.blockName}" consumed ${top.percentage.toFixed(0)}% of execution time. ` +
        `This relates to a known tradeoff: "${firstTradeoff}"`,
      whyItMatters:
        'Database engineering is about understanding tradeoffs — there is no universally ' +
        'optimal design. Each choice optimizes for some workloads at the cost of others.',
      learnMore: { blockType: top.blockType, section: 'tradeoffs' },
    });
  }

  private checkSequentialScan(
    _sorted: BlockMetrics[],
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'sequential_scan') continue;
      if (bm.percentage <= 15) continue;

      const pagesScanned = bm.counters['pages_scanned'] ?? bm.counters['pages_read'] ?? 0;

      const hasIndex = this.blockTypes.has('btree_index') ||
        this.blockTypes.has('hash_index');

      insights.push({
        id: nextId(),
        type: hasIndex ? 'explanation' : 'opportunity',
        severity: hasIndex ? 'info' : 'suggestion',
        title: pagesScanned > 0
          ? `Sequential Scan Read ${fmtNum(pagesScanned)} Pages`
          : `Sequential Scan at ${bm.percentage.toFixed(0)}% of Time`,
        explanation:
          `A sequential scan reads every page in the table — O(n) where n is the number of pages. ` +
          (pagesScanned > 0 ? `Here it touched ${fmtNum(pagesScanned)} pages. ` : '') +
          `This is efficient when you need most of the data (analytics, aggregations) ` +
          `but wasteful for selective queries that only need a few rows.`,
        whyItMatters:
          'Database query optimizers choose between sequential scan and index scan based on selectivity. ' +
          'If a query returns >5-10% of rows, a sequential scan is often faster than an index scan ' +
          'because it reads pages in order (sequential I/O) rather than random I/O.',
        suggestion: hasIndex
          ? 'Sequential scans can be beneficial for full-table operations. ' +
            'The optimizer would choose this over an index when reading most rows.'
          : 'For selective queries (finding a few rows), add an index to skip the full scan.',
        learnMore: { blockType: 'sequential_scan', section: 'algorithm' },
        realWorldExample:
          'PostgreSQL\'s query planner switches from index scan to sequential scan ' +
          'when it estimates more than ~5% of rows will match. This is called the "tipping point".',
      });
    }
  }

  private checkLsmWithoutBloom(
    insights: Insight[],
    nextId: () => string,
  ) {
    if (!this.blockTypes.has('lsm_tree')) return;
    if (this.blockTypes.has('bloom_filter')) return;

    insights.push({
      id: nextId(),
      type: 'opportunity',
      severity: 'suggestion',
      title: 'LSM Tree Without Bloom Filter',
      explanation:
        'Your LSM tree must check every SSTable level on reads because there is no Bloom filter to skip non-matching SSTables. ' +
        'This causes significant read amplification.',
      whyItMatters:
        'Bloom filters are the standard optimization for LSM reads. They answer "is this key definitely NOT in this SSTable?" ' +
        'in O(1) time, avoiding expensive disk reads.',
      suggestion: 'Add a Bloom filter after the LSM tree to skip SSTables that don\'t contain the requested key.',
      learnMore: { blockType: 'bloom_filter', section: 'useCases' },
      realWorldExample:
        'Cassandra and RocksDB both attach a Bloom filter to every SSTable. ' +
        'With 10 bits per key, false positive rates drop below 1%, dramatically reducing read I/O.',
    });
  }

  private checkBloomFilterEffectiveness(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'bloom_filter') continue;

      const checks = bm.counters['checks'] ?? 0;
      const fpRate = bm.counters['false_positive_rate'] ?? 0;
      const trueNeg = bm.counters['true_negatives'] ?? 0;

      if (checks === 0) continue;

      if (fpRate > 0.05) {
        insights.push({
          id: nextId(),
          type: 'bottleneck',
          severity: 'suggestion',
          title: `Bloom Filter False Positive Rate: ${(fpRate * 100).toFixed(1)}%`,
          explanation:
            `The Bloom filter checked ${fmtNum(checks)} queries but had a ${(fpRate * 100).toFixed(1)}% false positive rate. ` +
            `This means ${(fpRate * 100).toFixed(1)}% of "maybe present" answers were wrong, causing unnecessary disk reads.`,
          whyItMatters:
            'A high false positive rate defeats the purpose of the Bloom filter. ' +
            'Each false positive triggers an unnecessary SSTable read.',
          suggestion:
            'Increase num_bits or num_hash_functions to reduce false positives. ' +
            'The optimal ratio is about 10 bits per expected item with 7 hash functions.',
          learnMore: { blockType: 'bloom_filter', section: 'algorithm' },
        });
      } else if (trueNeg > 0) {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `Bloom Filter Prevented ${fmtNum(trueNeg)} Unnecessary Reads`,
          explanation:
            `The Bloom filter checked ${fmtNum(checks)} queries and correctly rejected ${fmtNum(trueNeg)} — ` +
            `each of those would have been a wasted disk read without the filter. ` +
            `False positive rate: ${(fpRate * 100).toFixed(2)}%.`,
          whyItMatters:
            'This is the Bloom filter doing exactly what it\'s designed for: ' +
            'trading a tiny amount of memory for massive I/O savings.',
          learnMore: { blockType: 'bloom_filter', section: 'algorithm' },
          realWorldExample:
            'Cassandra uses ~10 bits per key, giving ~1% false positive rate. ' +
            'This means 99% of unnecessary SSTable reads are eliminated.',
        });
      }
    }
  }

  private checkReplicationConsistency(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'replication') continue;

      const replicated = bm.counters['writes_replicated'] ?? 0;
      const lagMs = bm.counters['replication_lag_ms'] ?? 0;
      const violations = bm.counters['consistency_violations'] ?? 0;

      const node = this.findNodeByBlockId(bm.blockId);
      const rf = node ? Number((node.data as BlockNodeData).parameters['replication_factor'] ?? 3) : 3;
      const cl = node ? String((node.data as BlockNodeData).parameters['consistency_level'] ?? 'quorum') : 'quorum';

      if (rf >= 3 && cl === 'all') {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'important',
          title: `Full Consistency with ${rf} Replicas Adds Latency`,
          explanation:
            `With replication_factor=${rf} and consistency=ALL, every write must wait for ALL ${rf} replicas to acknowledge. ` +
            `This added ${fmtMs(lagMs)} of replication latency across ${fmtNum(replicated)} writes.`,
          whyItMatters:
            'This is the CAP theorem in action. Requiring all replicas to acknowledge trades availability for consistency — ' +
            'if any one replica is down, writes fail entirely.',
          suggestion:
            'Consider QUORUM consistency (majority of replicas) for a balance of consistency and availability. ' +
            'QUORUM read + QUORUM write still guarantees linearizability.',
          learnMore: { blockType: 'replication', section: 'tradeoffs' },
          realWorldExample:
            'Cassandra defaults to LOCAL_QUORUM. DynamoDB uses quorum internally across 3 AZs. ' +
            'Full ALL consistency is rare in production due to availability concerns.',
        });
      } else if (violations > 0) {
        insights.push({
          id: nextId(),
          type: 'bottleneck',
          severity: 'suggestion',
          title: `${violations} Consistency Violations Detected`,
          explanation:
            `With consistency=${cl.toUpperCase()}, ${violations} writes could not get enough acknowledgments. ` +
            `These operations were accepted but may not be durable on all replicas.`,
          whyItMatters:
            'Consistency violations mean reads might return stale data. ' +
            'This is acceptable for some use cases (caches, counters) but dangerous for financial data.',
          suggestion: 'Increase consistency_level to QUORUM or ALL if data accuracy is critical.',
          learnMore: { blockType: 'replication', section: 'tradeoffs' },
        });
      } else if (replicated > 0) {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `Replicated ${fmtNum(replicated)} Writes Across ${rf} Nodes`,
          explanation:
            `Each write was sent to ${rf} replicas with ${cl.toUpperCase()} consistency. ` +
            `Average replication lag: ${fmtMs(lagMs)}.`,
          whyItMatters:
            'Replication provides fault tolerance — if one node fails, the data exists on other replicas. ' +
            'The consistency level determines the durability-latency tradeoff.',
          learnMore: { blockType: 'replication', section: 'algorithm' },
        });
      }
    }
  }

  private checkPartitionDistribution(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'hash_partitioner') continue;

      const partitioned = bm.counters['records_partitioned'] ?? 0;
      const hottest = bm.counters['hottest_partition_pct'] ?? 0;
      const evenness = bm.counters['evenness_score'] ?? 0;

      if (partitioned === 0) continue;

      const node = this.findNodeByBlockId(bm.blockId);
      const numPartitions = node ? Number((node.data as BlockNodeData).parameters['num_partitions'] ?? 8) : 8;

      if (hottest > 30) {
        insights.push({
          id: nextId(),
          type: 'bottleneck',
          severity: 'important',
          title: `Hot Partition Detected — ${hottest.toFixed(0)}% of Data in One Partition`,
          explanation:
            `With ${numPartitions} partitions, each should hold ~${(100 / numPartitions).toFixed(0)}% of data. ` +
            `Instead, the hottest partition has ${hottest.toFixed(0)}%. Evenness score: ${(evenness * 100).toFixed(0)}%.`,
          whyItMatters:
            'Hot partitions defeat the purpose of partitioning — one node handles disproportionate load ' +
            'while others sit idle. This is the #1 scaling problem in distributed databases.',
          suggestion:
            'Choose a partition key with high cardinality and even distribution. ' +
            'Avoid keys like "date" or "region" that create natural hotspots.',
          learnMore: { blockType: 'hash_partitioner', section: 'tradeoffs' },
          realWorldExample:
            'DynamoDB throttles hot partition keys. Cassandra\'s vnodes (virtual nodes) help, ' +
            'but a bad partition key still creates hotspots.',
        });
      } else {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `Hash Partitioner: ${fmtNum(partitioned)} Records Across ${numPartitions} Partitions`,
          explanation:
            `Data was distributed across ${numPartitions} partitions with ${(evenness * 100).toFixed(0)}% evenness. ` +
            `Hottest partition: ${hottest.toFixed(0)}% (ideal: ${(100 / numPartitions).toFixed(0)}%).`,
          whyItMatters:
            'Good hash partitioning is the foundation of horizontal scaling. ' +
            'Each partition can be served by a different node, scaling throughput linearly.',
          learnMore: { blockType: 'hash_partitioner', section: 'algorithm' },
        });
      }
    }
  }

  private checkDictionaryCompression(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'dictionary_encoding') continue;

      const encoded = bm.counters['entries_encoded'] ?? 0;
      const dictSize = bm.counters['dictionary_size'] ?? 0;
      const ratio = bm.counters['compression_ratio'] ?? 1;
      const fullEvents = bm.counters['dictionary_full_events'] ?? 0;

      if (encoded === 0) continue;

      if (ratio > 2) {
        insights.push({
          id: nextId(),
          type: 'explanation',
          severity: 'info',
          title: `Dictionary Encoding Compressed Data ${ratio.toFixed(1)}x`,
          explanation:
            `Encoded ${fmtNum(encoded)} values using a dictionary of ${fmtNum(dictSize)} unique entries. ` +
            `Compression ratio: ${ratio.toFixed(1)}x — each repeated value is stored as a small integer code instead of the full value.`,
          whyItMatters:
            'Dictionary encoding is most effective on low-cardinality columns (e.g., country, status, category). ' +
            'Columnar databases like ClickHouse and Parquet use this extensively.',
          learnMore: { blockType: 'dictionary_encoding', section: 'algorithm' },
          realWorldExample:
            'ClickHouse applies dictionary encoding automatically to columns with low cardinality. ' +
            'A "country" column with 200 values can be stored in 1 byte per row instead of 20+.',
        });
      }

      if (fullEvents > 0) {
        insights.push({
          id: nextId(),
          type: 'bottleneck',
          severity: 'suggestion',
          title: `Dictionary Full — ${fullEvents} Values Couldn't Be Encoded`,
          explanation:
            `The dictionary reached its maximum size of ${fmtNum(dictSize)} entries. ` +
            `${fullEvents} additional unique values were passed through without compression.`,
          whyItMatters:
            'When the dictionary is full, new unique values can\'t benefit from compression. ' +
            'This typically means the column has higher cardinality than expected.',
          suggestion:
            'Increase max_dictionary_size, or reconsider whether this column benefits from dictionary encoding. ' +
            'High-cardinality columns (like UUIDs) are poor candidates.',
          learnMore: { blockType: 'dictionary_encoding', section: 'tradeoffs' },
        });
      }
    }
  }

  private checkClockBuffer(
    insights: Insight[],
    nextId: () => string,
  ) {
    for (const bm of this.result.blockMetrics) {
      if (bm.blockType !== 'clock_buffer') continue;

      const hitRate = bm.counters['hit_rate_pct'] ?? 0;
      const sweeps = bm.counters['clock_hand_sweeps'] ?? 0;
      const evictions = bm.counters['evictions'] ?? 0;

      if (hitRate === 0 && evictions === 0) continue;

      // Check if there's also an LRU buffer to compare
      const hasLru = this.blockTypes.has('lru_buffer');

      insights.push({
        id: nextId(),
        type: 'explanation',
        severity: 'info',
        title: `Clock Buffer: ${hitRate.toFixed(0)}% Hit Rate, ${fmtNum(sweeps)} Clock Sweeps`,
        explanation:
          `The CLOCK algorithm uses a circular buffer with reference bits. On eviction, the clock hand sweeps ` +
          `and clears reference bits until it finds an unreferenced page. ` +
          `${fmtNum(evictions)} evictions required ${fmtNum(sweeps)} clock hand sweeps.` +
          (hasLru ? ' Compare this with the LRU buffer in your design to see the difference.' : ''),
        whyItMatters:
          'CLOCK approximates LRU with lower overhead — it avoids moving pages to the front of a list on every access. ' +
          'PostgreSQL uses a CLOCK-based algorithm for its buffer manager for exactly this reason.',
        learnMore: { blockType: 'clock_buffer', section: 'algorithm' },
        realWorldExample:
          'PostgreSQL\'s buffer manager uses a clock-sweep algorithm. Each buffer page has a "usage count" ' +
          'that decrements on each sweep pass. Pages with higher counts survive longer.',
      });
    }
  }

  // -----------------------------------------------------------------------
  // Helpers
  // -----------------------------------------------------------------------

  private findNodeByBlockId(blockId: string): Node<BlockNodeData> | undefined {
    return this.nodes.find((n) => n.id === blockId);
  }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function fmtNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(Math.round(n));
}

function fmtMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(2)}ms`;
}
