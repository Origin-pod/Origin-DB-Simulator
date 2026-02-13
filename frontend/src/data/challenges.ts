// ---------------------------------------------------------------------------
// Learning Challenges — guided experiments that teach database concepts
// ---------------------------------------------------------------------------

import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from '@/stores/workloadStore';
import { CATEGORY_COLORS } from '@/types';
import { getBlockDefinition } from '@/types/blocks';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type Difficulty = 'beginner' | 'intermediate' | 'advanced';

export interface SuccessCriteria {
  type: 'run_execution' | 'check_throughput' | 'has_block' | 'compare_runs';
  /** For check_throughput: minimum ops/sec required */
  threshold?: number;
  /** For has_block: block type that must be present */
  blockType?: string;
  /** For compare_runs: improvement factor vs previous step */
  improvementFactor?: number;
}

export interface ChallengeStep {
  instruction: string;
  hint?: string;
  /** Pre-load this canvas state (replaces current design) */
  scaffold?: {
    nodes: Node<BlockNodeData>[];
    edges: Edge[];
    workload: Workload;
  };
  /** Add these to the current canvas (user adds manually — scaffold provides the goal) */
  goalDescription?: string;
  successCriteria?: SuccessCriteria;
  /** Shown after completing this step */
  educationalPayoff: string;
}

export interface Challenge {
  id: string;
  title: string;
  subtitle: string;
  difficulty: Difficulty;
  estimatedMinutes: number;
  concepts: string[];
  steps: ChallengeStep[];
}

// ---------------------------------------------------------------------------
// Node/Edge builders (same pattern as templates.ts)
// ---------------------------------------------------------------------------

let seq = 0;

function node(
  blockType: string,
  x: number,
  y: number,
  overrides?: Record<string, string | number | boolean>,
): Node<BlockNodeData> {
  const def = getBlockDefinition(blockType);
  if (!def) throw new Error(`Unknown block: ${blockType}`);

  const parameters: Record<string, string | number | boolean> = {};
  for (const p of def.parameters) parameters[p.name] = p.default;
  if (overrides) Object.assign(parameters, overrides);

  const id = `ch-${blockType}-${++seq}`;
  return {
    id,
    type: 'blockNode',
    position: { x, y },
    data: {
      blockType,
      label: def.name,
      category: def.category,
      icon: def.icon,
      color: CATEGORY_COLORS[def.category],
      inputs: def.inputs,
      outputs: def.outputs,
      parameters,
      state: 'idle',
    },
  };
}

function edge(
  src: Node<BlockNodeData>,
  srcHandle: string,
  tgt: Node<BlockNodeData>,
  tgtHandle: string,
): Edge {
  return {
    id: `e-${src.id}-${tgt.id}`,
    source: src.id,
    target: tgt.id,
    sourceHandle: srcHandle,
    targetHandle: tgtHandle,
    type: 'smoothstep',
    style: { strokeWidth: 2, stroke: '#94A3B8' },
  };
}

// ---------------------------------------------------------------------------
// Workload helpers
// ---------------------------------------------------------------------------

const READ_HEAVY_WORKLOAD: Workload = {
  name: 'Read-Heavy',
  operations: [
    { id: 'op-1', type: 'SELECT', weight: 90, template: 'SELECT * FROM {table} WHERE id = ?' },
    { id: 'op-2', type: 'INSERT', weight: 10, template: 'INSERT INTO {table} VALUES (?)' },
  ],
  distribution: 'zipfian',
  concurrency: 50,
  totalOperations: 10000,
};

const WRITE_HEAVY_WORKLOAD: Workload = {
  name: 'Write-Heavy',
  operations: [
    { id: 'op-1', type: 'INSERT', weight: 80, template: 'INSERT INTO {table} VALUES (?)' },
    { id: 'op-2', type: 'SELECT', weight: 20, template: 'SELECT * FROM {table} WHERE id = ?' },
  ],
  distribution: 'latest',
  concurrency: 100,
  totalOperations: 20000,
};

const BALANCED_WORKLOAD: Workload = {
  name: 'Balanced OLTP',
  operations: [
    { id: 'op-1', type: 'SELECT', weight: 50, template: 'SELECT * FROM {table} WHERE id = ?' },
    { id: 'op-2', type: 'UPDATE', weight: 30, template: 'UPDATE {table} SET col = ? WHERE id = ?' },
    { id: 'op-3', type: 'INSERT', weight: 20, template: 'INSERT INTO {table} VALUES (?)' },
  ],
  distribution: 'zipfian',
  concurrency: 100,
  totalOperations: 10000,
};

// ---------------------------------------------------------------------------
// Challenge 1: Why Do Databases Use Indexes?
// ---------------------------------------------------------------------------

function buildChallenge1(): Challenge {
  const schema1 = node('schema_definition', 50, 100, {
    tableName: 'users',
    columns: 'id:int,name:varchar,email:varchar',
  });
  const heap1 = node('heap_storage', 320, 100);
  const scan1 = node('sequential_scan', 600, 100);

  const schema2 = node('schema_definition', 50, 100, {
    tableName: 'users',
    columns: 'id:int,name:varchar,email:varchar',
  });
  const heap2 = node('heap_storage', 320, 100);
  const btree = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const buffer = node('lru_buffer', 600, 100, { size: 64 });

  return {
    id: 'why-indexes',
    title: 'Why Do Databases Use Indexes?',
    subtitle: 'Discover why every database creates indexes on primary keys',
    difficulty: 'beginner',
    estimatedMinutes: 5,
    concepts: ['B-tree index', 'Sequential scan', 'O(n) vs O(log n)'],
    steps: [
      {
        instruction:
          'Start with a simple design: Schema → Heap Storage → Sequential Scan. ' +
          'This is the simplest possible database — no indexes, no caching. ' +
          'Click "Run" to execute the read-heavy workload.',
        scaffold: {
          nodes: [schema1, heap1, scan1],
          edges: [
            edge(schema1, 'schema', heap1, 'records'),
            edge(heap1, 'stored', scan1, 'records'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Notice the throughput and latency. Every single read has to scan ALL records ' +
          'from the first page to the last — this is O(n) sequential scanning. ' +
          'Look at how much time the storage block consumed.',
      },
      {
        instruction:
          'Now add a B-tree index and an LRU buffer pool. ' +
          'The design is pre-loaded — click "Run" with the SAME workload.',
        scaffold: {
          nodes: [schema2, heap2, btree, buffer],
          edges: [
            edge(schema2, 'schema', heap2, 'records'),
            edge(heap2, 'stored', btree, 'records'),
            edge(heap2, 'stored', buffer, 'requests'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Dramatic improvement! The B-tree index reduced lookups from O(n) — scanning every record — ' +
          'to O(log n) — following a tree path. With fanout 128, even a million records only needs ' +
          '~3 tree levels (log₁₂₈(1,000,000) ≈ 2.8). ' +
          'This is exactly why PostgreSQL creates a B-tree index on every primary key automatically.',
      },
      {
        instruction:
          'Compare the two runs in the metrics. How much faster was the indexed version? ' +
          'Look at the block breakdown — where did the time shift?',
        goalDescription: 'Review the metrics dashboard and insights panel',
        educationalPayoff:
          'Congratulations! You just discovered the most fundamental optimization in databases. ' +
          'Indexes trade extra storage space and slower writes for dramatically faster reads. ' +
          'Every production database — PostgreSQL, MySQL, SQLite, SQL Server — uses B-tree indexes ' +
          'as their default index structure.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Challenge 2: B-tree vs Hash — When to Use Which?
// ---------------------------------------------------------------------------

function buildChallenge2(): Challenge {
  const schema1 = node('schema_definition', 50, 100, {
    tableName: 'sessions',
    columns: 'id:int,user_id:int,token:varchar',
  });
  const heap1 = node('heap_storage', 320, 100);
  const btree1 = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const buffer1 = node('lru_buffer', 600, 100, { size: 64 });

  const schema2 = node('schema_definition', 50, 100, {
    tableName: 'sessions',
    columns: 'id:int,user_id:int,token:varchar',
  });
  const heap2 = node('heap_storage', 320, 100);
  const hash = node('hash_index', 320, 280, { keyColumn: 'id', buckets: 2048 });
  const buffer2 = node('lru_buffer', 600, 100, { size: 64 });

  return {
    id: 'btree-vs-hash',
    title: 'B-tree vs Hash Index',
    subtitle: 'Learn when to use each index type',
    difficulty: 'beginner',
    estimatedMinutes: 7,
    concepts: ['B-tree', 'Hash index', 'Point vs range queries'],
    steps: [
      {
        instruction:
          'Run this design with a B-tree index. The workload is point lookups (WHERE id = ?).',
        scaffold: {
          nodes: [schema1, heap1, btree1, buffer1],
          edges: [
            edge(schema1, 'schema', heap1, 'records'),
            edge(heap1, 'stored', btree1, 'records'),
            edge(heap1, 'stored', buffer1, 'requests'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'The B-tree performs well for point lookups. Each lookup traverses log(n) tree levels. ' +
          'Note the tree depth in the counters — that is exactly how many page reads each lookup needs.',
      },
      {
        instruction:
          'Now try the same workload with a hash index instead of a B-tree.',
        scaffold: {
          nodes: [schema2, heap2, hash, buffer2],
          edges: [
            edge(schema2, 'schema', heap2, 'records'),
            edge(heap2, 'stored', hash, 'records'),
            edge(heap2, 'stored', buffer2, 'requests'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'The hash index gives O(1) lookups — compute the hash, go directly to the bucket. ' +
          'For pure point lookups (WHERE id = ?), hash indexes can be faster than B-trees. ' +
          'But there is a tradeoff...',
      },
      {
        instruction:
          'Think about this: what if your workload had range queries ' +
          '(WHERE id BETWEEN 100 AND 200)? Hash indexes cannot answer range queries ' +
          'because hashing destroys key ordering. B-trees maintain sorted order, ' +
          'so they can scan a range efficiently.',
        goalDescription: 'Reflect on the B-tree vs hash tradeoff',
        educationalPayoff:
          'B-trees: O(log n) point lookups, efficient range scans, sorted output. ' +
          'Hash indexes: O(1) point lookups, no range support, no sorted output. ' +
          'This is why PostgreSQL defaults to B-tree — it handles both cases. ' +
          'Hash indexes in PostgreSQL were not even crash-safe until version 10!',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Challenge 3: The Buffer Pool Effect
// ---------------------------------------------------------------------------

function buildChallenge3(): Challenge {
  const schema1 = node('schema_definition', 50, 100, {
    tableName: 'products',
    columns: 'id:int,name:varchar,price:decimal',
  });
  const heap1 = node('heap_storage', 320, 100);
  const btree1 = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const scan1 = node('sequential_scan', 600, 100);

  const schema2 = node('schema_definition', 50, 100, {
    tableName: 'products',
    columns: 'id:int,name:varchar,price:decimal',
  });
  const heap2 = node('heap_storage', 320, 100);
  const btree2 = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const smallBuffer = node('lru_buffer', 600, 100, { size: 4 });

  const schema3 = node('schema_definition', 50, 100, {
    tableName: 'products',
    columns: 'id:int,name:varchar,price:decimal',
  });
  const heap3 = node('heap_storage', 320, 100);
  const btree3 = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const largeBuffer = node('lru_buffer', 600, 100, { size: 512 });

  return {
    id: 'buffer-pool',
    title: 'The Buffer Pool Effect',
    subtitle: 'See how caching transforms database performance',
    difficulty: 'intermediate',
    estimatedMinutes: 8,
    concepts: ['Buffer pool', 'Cache hit rate', 'LRU eviction', 'Working set'],
    steps: [
      {
        instruction:
          'Run this design WITHOUT a buffer pool. Every read goes directly to storage.',
        scaffold: {
          nodes: [schema1, heap1, btree1, scan1],
          edges: [
            edge(schema1, 'schema', heap1, 'records'),
            edge(heap1, 'stored', btree1, 'records'),
            edge(heap1, 'stored', scan1, 'records'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Without a buffer pool, every operation reads from "disk". ' +
          'In real systems, a disk read takes ~10ms (HDD) or ~0.1ms (SSD), ' +
          'while a memory read takes ~0.0001ms. That is a 1000x difference.',
      },
      {
        instruction:
          'Add a tiny buffer pool (4 MB). This is intentionally small to show partial caching.',
        scaffold: {
          nodes: [schema2, heap2, btree2, smallBuffer],
          edges: [
            edge(schema2, 'schema', heap2, 'records'),
            edge(heap2, 'stored', btree2, 'records'),
            edge(heap2, 'stored', smallBuffer, 'requests'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Some improvement! But check the cache hit rate. With only 4 MB, ' +
          'the buffer pool is too small to hold the "working set" — the hot data ' +
          'that gets accessed repeatedly. The LRU policy keeps evicting pages you need.',
      },
      {
        instruction:
          'Now try with a large buffer pool (512 MB). Watch the hit rate.',
        scaffold: {
          nodes: [schema3, heap3, btree3, largeBuffer],
          edges: [
            edge(schema3, 'schema', heap3, 'records'),
            edge(heap3, 'stored', btree3, 'records'),
            edge(heap3, 'stored', largeBuffer, 'requests'),
          ],
          workload: READ_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'The larger the buffer pool relative to your data, the higher the hit rate. ' +
          'Most production databases aim for 99%+ hit rates. ' +
          'PostgreSQL recommends setting shared_buffers to 25% of total RAM. ' +
          'A well-tuned MySQL InnoDB buffer pool holds the entire working set in memory, ' +
          'making reads essentially free.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Challenge 4: LSM vs B-tree — Write vs Read Optimization
// ---------------------------------------------------------------------------

function buildChallenge4(): Challenge {
  const schema1 = node('schema_definition', 50, 100, {
    tableName: 'events',
    columns: 'id:int,timestamp:bigint,type:varchar,data:text',
  });
  const heap = node('heap_storage', 320, 100);
  const btree = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 128 });
  const buffer1 = node('lru_buffer', 600, 100, { size: 128 });

  const schema2 = node('schema_definition', 50, 100, {
    tableName: 'events',
    columns: 'id:int,timestamp:bigint,type:varchar,data:text',
  });
  const lsm = node('lsm_tree', 320, 100, { memtableSize: 64, levelMultiplier: 10, bloomFilterBits: 10 });
  const filter = node('filter', 600, 100, { predicate: 'type = "error"' });

  return {
    id: 'lsm-vs-btree',
    title: 'LSM vs B-tree',
    subtitle: 'Understand the write-optimized vs read-optimized tradeoff',
    difficulty: 'intermediate',
    estimatedMinutes: 10,
    concepts: ['LSM tree', 'Write amplification', 'Compaction', 'Memtable'],
    steps: [
      {
        instruction:
          'Run this traditional B-tree design with a write-heavy workload (80% inserts).',
        scaffold: {
          nodes: [schema1, heap, btree, buffer1],
          edges: [
            edge(schema1, 'schema', heap, 'records'),
            edge(heap, 'stored', btree, 'records'),
            edge(heap, 'stored', buffer1, 'requests'),
          ],
          workload: WRITE_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Every insert into a B-tree must: find the right leaf page, insert the key in sorted order, ' +
          'and potentially split pages. This is random I/O — each write goes to a different place on disk.',
      },
      {
        instruction:
          'Now try an LSM tree with the SAME write-heavy workload.',
        scaffold: {
          nodes: [schema2, lsm, filter],
          edges: [
            edge(schema2, 'schema', lsm, 'records'),
            edge(lsm, 'stored', filter, 'records'),
          ],
          workload: WRITE_HEAVY_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'The LSM tree writes to an in-memory memtable (fast!) and only flushes to disk ' +
          'when the memtable is full. Disk writes are sequential (append-only), which is much faster ' +
          'than B-tree\'s random writes. Check the compaction and flush counters.',
      },
      {
        instruction:
          'Look at the counters: memtable_flushes, compactions, levels_used. ' +
          'The LSM tree trades read performance for write performance. ' +
          'Reads may need to check multiple levels (read amplification).',
        goalDescription: 'Compare the two approaches in the insights panel',
        educationalPayoff:
          'B-tree: Good for reads (O(log n)), slower for writes (random I/O, page splits). ' +
          'LSM tree: Excellent for writes (sequential I/O), slower for reads (check multiple levels). ' +
          'This is why PostgreSQL (B-tree) excels at OLTP, while RocksDB and Cassandra (LSM) ' +
          'are used for write-heavy workloads like logging and time-series data.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Challenge 5: Building PostgreSQL
// ---------------------------------------------------------------------------

function buildChallenge5(): Challenge {
  // Step 1: Just storage
  const s1_schema = node('schema_definition', 50, 100, {
    tableName: 'accounts',
    columns: 'id:int,balance:decimal,name:varchar,updated_at:timestamp',
  });
  const s1_heap = node('heap_storage', 320, 100, { pageSize: 8192 });

  // Step 2: Add index
  const s2_schema = node('schema_definition', 50, 100, {
    tableName: 'accounts',
    columns: 'id:int,balance:decimal,name:varchar,updated_at:timestamp',
  });
  const s2_heap = node('heap_storage', 320, 100, { pageSize: 8192 });
  const s2_btree = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 256, unique: true });

  // Step 3: Add buffer pool
  const s3_schema = node('schema_definition', 50, 100, {
    tableName: 'accounts',
    columns: 'id:int,balance:decimal,name:varchar,updated_at:timestamp',
  });
  const s3_heap = node('heap_storage', 320, 100, { pageSize: 8192 });
  const s3_btree = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 256, unique: true });
  const s3_buffer = node('lru_buffer', 600, 100, { size: 256, pageSize: 8192 });

  // Step 4: Full stack with WAL
  const s4_schema = node('schema_definition', 50, 100, {
    tableName: 'accounts',
    columns: 'id:int,balance:decimal,name:varchar,updated_at:timestamp',
  });
  const s4_heap = node('heap_storage', 320, 100, { pageSize: 8192 });
  const s4_btree = node('btree_index', 320, 280, { keyColumn: 'id', fanout: 256, unique: true });
  const s4_buffer = node('lru_buffer', 600, 100, { size: 256, pageSize: 8192 });
  const s4_wal = node('wal', 600, 280, { bufferSize: 16, syncMode: 'fsync' });

  return {
    id: 'building-postgresql',
    title: 'Building PostgreSQL',
    subtitle: 'Reconstruct the PostgreSQL storage stack, piece by piece',
    difficulty: 'advanced',
    estimatedMinutes: 15,
    concepts: ['PostgreSQL architecture', 'Heap files', 'B-tree', 'Buffer pool', 'WAL'],
    steps: [
      {
        instruction:
          'Start with just the heap — PostgreSQL stores rows in 8KB pages in heap files. ' +
          'Run a balanced OLTP workload.',
        scaffold: {
          nodes: [s1_schema, s1_heap],
          edges: [edge(s1_schema, 'schema', s1_heap, 'records')],
          workload: BALANCED_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'This is the bare minimum — just heap files. In real PostgreSQL, every table ' +
          'is stored as a heap file. Without indexes, every query is a sequential scan. ' +
          'Note the throughput — we will improve it step by step.',
      },
      {
        instruction:
          'PostgreSQL creates a B-tree index on every primary key (CREATE TABLE ... PRIMARY KEY). ' +
          'Add the B-tree with fanout 256 (PostgreSQL uses ~300). Run the same workload.',
        scaffold: {
          nodes: [s2_schema, s2_heap, s2_btree],
          edges: [
            edge(s2_schema, 'schema', s2_heap, 'records'),
            edge(s2_heap, 'stored', s2_btree, 'records'),
          ],
          workload: BALANCED_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'Big improvement! The B-tree index is why PostgreSQL can handle millions of point queries per second. ' +
          'With fanout ~300, even a billion-row table needs only 4 B-tree levels. ' +
          'In PostgreSQL, this is managed by the "nbtree" access method module.',
      },
      {
        instruction:
          'PostgreSQL uses shared_buffers (an LRU-like buffer pool) to cache frequently ' +
          'accessed pages in memory. Add a 256MB buffer pool.',
        scaffold: {
          nodes: [s3_schema, s3_heap, s3_btree, s3_buffer],
          edges: [
            edge(s3_schema, 'schema', s3_heap, 'records'),
            edge(s3_heap, 'stored', s3_btree, 'records'),
            edge(s3_heap, 'stored', s3_buffer, 'requests'),
          ],
          workload: BALANCED_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'The buffer pool is the second biggest performance lever. PostgreSQL\'s shared_buffers ' +
          'caches both data and index pages. A well-sized buffer pool (typically 25% of RAM) ' +
          'achieves 99%+ hit rates, making most reads avoid disk entirely. ' +
          'PostgreSQL actually uses a Clock eviction policy (approximation of LRU).',
      },
      {
        instruction:
          'Finally, add a Write-Ahead Log (WAL) for crash recovery. ' +
          'PostgreSQL writes every change to the WAL BEFORE modifying data files. ' +
          'This guarantees durability even if the server crashes mid-write.',
        scaffold: {
          nodes: [s4_schema, s4_heap, s4_btree, s4_buffer, s4_wal],
          edges: [
            edge(s4_schema, 'schema', s4_heap, 'records'),
            edge(s4_heap, 'stored', s4_btree, 'records'),
            edge(s4_heap, 'stored', s4_buffer, 'requests'),
            edge(s4_schema, 'schema', s4_wal, 'records'),
          ],
          workload: BALANCED_WORKLOAD,
        },
        successCriteria: { type: 'run_execution' },
        educationalPayoff:
          'You just built the core PostgreSQL storage engine! The real PostgreSQL has more ' +
          '(MVCC for concurrency, the query planner, VACUUM for garbage collection) ' +
          'but these four components — heap files, B-tree indexes, shared_buffers, and WAL — ' +
          'are the foundation that every query touches. ' +
          'This architecture has been refined over 30+ years and powers some of the world\'s ' +
          'largest databases.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// All challenges
// ---------------------------------------------------------------------------

export function getChallenges(): Challenge[] {
  seq = 0;
  return [
    buildChallenge1(),
    buildChallenge2(),
    buildChallenge3(),
    buildChallenge4(),
    buildChallenge5(),
  ];
}

export const DIFFICULTY_LABELS: Record<Difficulty, string> = {
  beginner: 'Beginner',
  intermediate: 'Intermediate',
  advanced: 'Advanced',
};

export const DIFFICULTY_COLORS: Record<Difficulty, string> = {
  beginner: '#10B981',
  intermediate: '#F59E0B',
  advanced: '#EF4444',
};
