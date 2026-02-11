import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from '@/stores/workloadStore';
import type {
  ExecutionResult,
  BlockMetrics,
  ProgressCallback,
  LatencyMetrics,
} from './types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/** Random number in [lo, hi) */
function rand(lo: number, hi: number): number {
  return lo + Math.random() * (hi - lo);
}

/** Clamp a number */
function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

// ---------------------------------------------------------------------------
// Topological sort (Kahn's algorithm)
// ---------------------------------------------------------------------------

function topoSort(nodeIds: string[], edges: Edge[]): string[] | null {
  const inDegree = new Map<string, number>();
  const adj = new Map<string, string[]>();

  for (const id of nodeIds) {
    inDegree.set(id, 0);
    adj.set(id, []);
  }

  for (const edge of edges) {
    if (!inDegree.has(edge.source) || !inDegree.has(edge.target)) continue;
    adj.get(edge.source)!.push(edge.target);
    inDegree.set(edge.target, (inDegree.get(edge.target) ?? 0) + 1);
  }

  const queue: string[] = [];
  for (const [id, deg] of inDegree) {
    if (deg === 0) queue.push(id);
  }

  const sorted: string[] = [];
  while (queue.length > 0) {
    const node = queue.shift()!;
    sorted.push(node);
    for (const neighbor of adj.get(node) ?? []) {
      const newDeg = (inDegree.get(neighbor) ?? 1) - 1;
      inDegree.set(neighbor, newDeg);
      if (newDeg === 0) queue.push(neighbor);
    }
  }

  // If we didn't visit all nodes, there's a cycle
  return sorted.length === nodeIds.length ? sorted : null;
}

// ---------------------------------------------------------------------------
// Mock metric generators per block type
// ---------------------------------------------------------------------------

interface MetricGen {
  estimateMs: (params: Record<string, unknown>, totalOps: number, readRatio: number) => number;
  counters: (params: Record<string, unknown>, totalOps: number, readRatio: number) => Record<string, number>;
}

const METRIC_GENERATORS: Record<string, MetricGen> = {
  // Storage
  heap_storage: {
    estimateMs: (_p, ops) => ops * rand(0.03, 0.06),
    counters: (p, ops) => {
      const pageSize = Number(p.pageSize ?? 8192);
      const pagesWritten = Math.ceil((ops * 100) / pageSize);
      return { pages_written: pagesWritten, pages_read: Math.ceil(pagesWritten * 1.2) };
    },
  },
  clustered_storage: {
    estimateMs: (_p, ops) => ops * rand(0.04, 0.08),
    counters: (p, ops) => {
      const pageSize = Number(p.pageSize ?? 8192);
      const pages = Math.ceil((ops * 100) / pageSize);
      return { pages_written: pages, reorg_count: Math.ceil(pages * 0.1) };
    },
  },
  lsm_tree: {
    estimateMs: (_p, ops) => ops * rand(0.02, 0.04),
    counters: (p, ops) => {
      const memSize = Number(p.memtableSize ?? 64);
      const flushes = Math.ceil(ops / (memSize * 1000));
      return { memtable_writes: ops, flushes, compactions: Math.ceil(flushes * 0.3) };
    },
  },
  schema_definition: {
    estimateMs: () => rand(1, 5),
    counters: () => ({ schemas_created: 1 }),
  },

  // Index
  btree_index: {
    estimateMs: (p, ops, readRatio) => {
      const fanout = Number(p.fanout ?? 128);
      const depth = Math.max(1, Math.ceil(Math.log(ops) / Math.log(fanout)));
      return ops * readRatio * depth * rand(0.05, 0.1);
    },
    counters: (p, ops, readRatio) => {
      const fanout = Number(p.fanout ?? 128);
      const depth = Math.max(1, Math.ceil(Math.log(ops) / Math.log(fanout)));
      return { lookups: Math.ceil(ops * readRatio), tree_depth: depth, pages_read: Math.ceil(ops * readRatio * depth) };
    },
  },
  hash_index: {
    estimateMs: (_p, ops, readRatio) => ops * readRatio * rand(0.02, 0.05),
    counters: (_p, ops, readRatio) => {
      const lookups = Math.ceil(ops * readRatio);
      return { lookups, collisions: Math.ceil(lookups * 0.05) };
    },
  },
  covering_index: {
    estimateMs: (_p, ops, readRatio) => ops * readRatio * rand(0.03, 0.07),
    counters: (_p, ops, readRatio) => ({
      lookups: Math.ceil(ops * readRatio),
      table_lookups_avoided: Math.ceil(ops * readRatio * 0.8),
    }),
  },

  // Buffer
  lru_buffer: {
    estimateMs: (p, ops) => {
      const size = Number(p.size ?? 128);
      const hitRate = clamp(0.5 + size / 1000, 0.5, 0.99);
      return ops * (1 - hitRate) * rand(0.5, 1.0);
    },
    counters: (p, ops) => {
      const size = Number(p.size ?? 128);
      const hitRate = clamp(0.5 + size / 1000, 0.5, 0.99);
      return {
        cache_hits: Math.ceil(ops * hitRate),
        cache_misses: Math.ceil(ops * (1 - hitRate)),
        hit_rate_pct: Math.round(hitRate * 100),
        evictions: Math.ceil(ops * (1 - hitRate) * 0.8),
      };
    },
  },
  clock_buffer: {
    estimateMs: (p, ops) => {
      const size = Number(p.size ?? 128);
      const hitRate = clamp(0.45 + size / 1100, 0.45, 0.97);
      return ops * (1 - hitRate) * rand(0.5, 1.0);
    },
    counters: (p, ops) => {
      const size = Number(p.size ?? 128);
      const hitRate = clamp(0.45 + size / 1100, 0.45, 0.97);
      return {
        cache_hits: Math.ceil(ops * hitRate),
        cache_misses: Math.ceil(ops * (1 - hitRate)),
        hit_rate_pct: Math.round(hitRate * 100),
        clock_hand_sweeps: Math.ceil(ops * (1 - hitRate) * 1.5),
      };
    },
  },

  // Execution
  sequential_scan: {
    estimateMs: (_p, ops) => ops * rand(0.08, 0.15),
    counters: (p, ops) => {
      const prefetch = Number(p.prefetchPages ?? 32);
      return { pages_scanned: Math.ceil(ops / 10), prefetch_batches: Math.ceil(ops / (10 * prefetch)) };
    },
  },
  index_scan: {
    estimateMs: (_p, ops, readRatio) => ops * readRatio * rand(0.04, 0.08),
    counters: (_p, ops, readRatio) => ({
      index_lookups: Math.ceil(ops * readRatio),
      rows_fetched: Math.ceil(ops * readRatio),
    }),
  },
  filter: {
    estimateMs: (_p, ops) => ops * rand(0.01, 0.03),
    counters: (_p, ops) => {
      const selectivity = rand(0.1, 0.5);
      return { rows_in: ops, rows_out: Math.ceil(ops * selectivity), selectivity_pct: Math.round(selectivity * 100) };
    },
  },
  sort: {
    estimateMs: (p, ops) => {
      const mem = Number(p.memoryLimit ?? 256);
      const needsExternal = ops > mem * 5000;
      return ops * Math.log2(ops) * rand(0.001, 0.003) * (needsExternal ? 3 : 1);
    },
    counters: (p, ops) => {
      const mem = Number(p.memoryLimit ?? 256);
      const needsExternal = ops > mem * 5000;
      return {
        comparisons: Math.ceil(ops * Math.log2(ops)),
        external_sort: needsExternal ? 1 : 0,
        temp_files: needsExternal ? Math.ceil(ops / (mem * 5000)) : 0,
      };
    },
  },
  hash_join: {
    estimateMs: (p, ops) => {
      const mem = Number(p.buildMemory ?? 256);
      const spill = ops > mem * 3000;
      return ops * rand(0.05, 0.1) * (spill ? 2.5 : 1);
    },
    counters: (p, ops) => {
      const mem = Number(p.buildMemory ?? 256);
      const spill = ops > mem * 3000;
      return {
        build_rows: Math.ceil(ops * 0.3),
        probe_rows: Math.ceil(ops * 0.7),
        output_rows: Math.ceil(ops * 0.25),
        spilled_partitions: spill ? Math.ceil(ops / (mem * 3000)) : 0,
      };
    },
  },

  // Concurrency
  row_lock: {
    estimateMs: (_p, ops) => ops * rand(0.01, 0.03),
    counters: (p, ops) => {
      const timeout = Number(p.lockTimeout ?? 5000);
      const contentionRate = clamp(0.05 - timeout / 200000, 0.001, 0.1);
      return {
        locks_acquired: ops,
        lock_waits: Math.ceil(ops * contentionRate),
        deadlocks: Math.ceil(ops * contentionRate * 0.01),
      };
    },
  },
  mvcc: {
    estimateMs: (_p, ops) => ops * rand(0.02, 0.05),
    counters: (_p, ops) => ({
      versions_created: Math.ceil(ops * 0.3),
      versions_gc: Math.ceil(ops * 0.2),
      snapshot_reads: Math.ceil(ops * 0.7),
    }),
  },

  // Transaction
  wal: {
    estimateMs: (p, ops) => {
      const bufSize = Number(p.bufferSize ?? 16);
      const syncPenalty = (p.syncMode === 'none') ? 0.01 : (p.syncMode === 'fdatasync') ? 0.04 : 0.06;
      return ops * syncPenalty * (16 / Math.max(1, bufSize));
    },
    counters: (p, ops) => {
      const bufSize = Number(p.bufferSize ?? 16);
      return {
        log_entries: ops,
        log_flushes: Math.ceil(ops / (bufSize * 100)),
        bytes_written: ops * Math.ceil(rand(50, 200)),
      };
    },
  },
};

// Fallback for unknown block types
const DEFAULT_GEN: MetricGen = {
  estimateMs: (_p, ops) => ops * rand(0.02, 0.05),
  counters: () => ({}),
};

// ---------------------------------------------------------------------------
// MockExecutionEngine
// ---------------------------------------------------------------------------

export class MockExecutionEngine {
  private cancelled = false;

  // ----- Validation -----

  validate(
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
  ): import('./types').ValidationResult {
    const errors: import('./types').ValidationError[] = [];
    const warnings: import('./types').ValidationWarning[] = [];

    // 1. Need at least one block
    if (nodes.length === 0) {
      errors.push({
        message: 'Canvas is empty — add at least one block.',
        suggestion: 'Drag a block from the palette on the left onto the canvas.',
      });
      return { valid: false, errors, warnings };
    }

    // 2. At least one storage block
    const storageNodes = nodes.filter(
      (n) => (n.data as BlockNodeData).category === 'storage',
    );
    if (storageNodes.length === 0) {
      errors.push({
        message: 'Design needs at least one storage block.',
        suggestion: 'Add a storage block (Heap Storage, B-tree, LSM Tree, or Clustered Storage).',
      });
    }

    // 3. Required input ports connected
    const connectedTargets = new Set(
      edges.map((e) => `${e.target}:${e.targetHandle}`),
    );
    for (const node of nodes) {
      const data = node.data as BlockNodeData;
      for (const port of data.inputs) {
        if (port.required && !connectedTargets.has(`${node.id}:${port.name}`)) {
          errors.push({
            nodeId: node.id,
            message: `"${data.label}" is missing required input "${port.name}".`,
            suggestion: `Connect a compatible block's output to the "${port.name}" input.`,
          });
        }
      }
    }

    // 4. Cycle detection via topo-sort
    const nodeIds = nodes.map((n) => n.id);
    const sorted = topoSort(nodeIds, edges);
    if (!sorted) {
      errors.push({
        message: 'Design contains a cycle — connections must form a DAG.',
        suggestion: 'Remove one of the connections to break the cycle.',
      });
    }

    // 5. Warnings for disconnected blocks
    const connectedIds = new Set<string>();
    for (const e of edges) {
      connectedIds.add(e.source);
      connectedIds.add(e.target);
    }
    for (const node of nodes) {
      if (!connectedIds.has(node.id) && nodes.length > 1) {
        warnings.push({
          nodeId: node.id,
          message: `"${(node.data as BlockNodeData).label}" is disconnected from the design.`,
          suggestion: 'Connect it to other blocks or remove it.',
        });
      }
    }

    // 6. Warnings for small buffer sizes
    for (const node of nodes) {
      const data = node.data as BlockNodeData;
      if (data.category === 'buffer') {
        const size = Number(data.parameters.size ?? 0);
        if (size > 0 && size < 64) {
          warnings.push({
            nodeId: node.id,
            message: `"${data.label}" has a small buffer size (${size} MB).`,
            suggestion: 'Consider increasing the buffer size for better cache performance.',
          });
        }
      }
    }

    return { valid: errors.length === 0, errors, warnings };
  }

  // ----- Execution -----

  cancel(): void {
    this.cancelled = true;
  }

  async execute(
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
    workload: Workload,
    onProgress: ProgressCallback,
  ): Promise<ExecutionResult> {
    this.cancelled = false;

    // Topo-sort to determine execution order
    const nodeIds = nodes.map((n) => n.id);
    const order = topoSort(nodeIds, edges) ?? nodeIds;

    const nodeMap = new Map(nodes.map((n) => [n.id, n]));
    const totalOps = workload.totalOperations;
    const readRatio =
      workload.operations.reduce(
        (s, op) => s + (op.type === 'SELECT' || op.type === 'SCAN' ? op.weight : 0),
        0,
      ) / 100;

    const blockMetrics: BlockMetrics[] = [];
    let totalExecTime = 0;

    // Execute each block in order
    for (let i = 0; i < order.length; i++) {
      if (this.cancelled) {
        return {
          success: false,
          duration: totalExecTime,
          metrics: this.emptyMetrics(totalOps),
          blockMetrics,
          errors: ['Execution cancelled by user.'],
        };
      }

      const nodeId = order[i];
      const node = nodeMap.get(nodeId);
      if (!node) continue;

      const data = node.data as BlockNodeData;
      const progress = Math.round(((i + 1) / order.length) * 100);

      onProgress({
        phase: 'executing',
        progress,
        currentBlock: nodeId,
        message: `Running ${data.label}...`,
      });

      const gen = METRIC_GENERATORS[data.blockType] ?? DEFAULT_GEN;
      const execMs = gen.estimateMs(data.parameters as Record<string, unknown>, totalOps, readRatio);
      const counters = gen.counters(data.parameters as Record<string, unknown>, totalOps, readRatio);

      totalExecTime += execMs;

      blockMetrics.push({
        blockId: nodeId,
        blockType: data.blockType,
        blockName: data.label,
        executionTime: Math.round(execMs * 100) / 100,
        percentage: 0, // fill after we know total
        counters,
      });

      // Simulate real-time delay (scaled down: ~50–200ms per block)
      const delayMs = clamp(50 + order.length * 10, 50, 200);
      await sleep(delayMs);
    }

    // Fill in percentages
    for (const bm of blockMetrics) {
      bm.percentage =
        totalExecTime > 0
          ? Math.round((bm.executionTime / totalExecTime) * 1000) / 10
          : 0;
    }

    // Aggregate metrics
    onProgress({
      phase: 'aggregating',
      progress: 100,
      currentBlock: null,
      message: 'Computing final metrics...',
    });
    await sleep(100);

    const durationSec = totalExecTime / 1000;
    const throughput = durationSec > 0 ? Math.round(totalOps / durationSec) : totalOps;
    const failedOps = Math.ceil(totalOps * rand(0.0001, 0.005));
    const latency = this.generateLatency(totalExecTime, totalOps);

    return {
      success: true,
      duration: Math.round(totalExecTime * 100) / 100,
      metrics: {
        throughput,
        latency,
        totalOperations: totalOps,
        successfulOperations: totalOps - failedOps,
        failedOperations: failedOps,
      },
      blockMetrics,
    };
  }

  // ----- Private helpers -----

  private generateLatency(totalMs: number, totalOps: number): LatencyMetrics {
    const avgMs = totalOps > 0 ? totalMs / totalOps : 0;
    return {
      avg: Math.round(avgMs * 1000) / 1000,
      p50: Math.round(avgMs * 0.8 * 1000) / 1000,
      p95: Math.round(avgMs * 2.5 * 1000) / 1000,
      p99: Math.round(avgMs * 5.0 * 1000) / 1000,
    };
  }

  private emptyMetrics(totalOps: number): import('./types').ExecutionMetrics {
    return {
      throughput: 0,
      latency: { avg: 0, p50: 0, p95: 0, p99: 0 },
      totalOperations: totalOps,
      successfulOperations: 0,
      failedOperations: 0,
    };
  }
}
