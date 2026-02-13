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
