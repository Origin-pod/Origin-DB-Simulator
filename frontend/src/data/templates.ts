import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import { CATEGORY_COLORS } from '@/types';
import { getBlockDefinition } from '@/types/blocks';
import type { Workload } from '@/stores/workloadStore';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TemplateCategory = 'oltp' | 'olap' | 'write-heavy' | 'read-heavy';

export interface DesignTemplate {
  id: string;
  name: string;
  description: string;
  category: TemplateCategory;
  tags: string[];
  nodes: Node<BlockNodeData>[];
  edges: Edge[];
  workload: Workload;
}

// ---------------------------------------------------------------------------
// Helper to build a node from a block type
// ---------------------------------------------------------------------------

let nodeSeq = 0;

function makeNode(
  blockType: string,
  x: number,
  y: number,
  paramOverrides?: Record<string, string | number | boolean>,
): Node<BlockNodeData> {
  const def = getBlockDefinition(blockType);
  if (!def) throw new Error(`Unknown block type: ${blockType}`);

  const parameters: Record<string, string | number | boolean> = {};
  for (const p of def.parameters) {
    parameters[p.name] = p.default;
  }
  if (paramOverrides) {
    Object.assign(parameters, paramOverrides);
  }

  const id = `tpl-${blockType}-${++nodeSeq}`;
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

function makeEdge(
  sourceNode: Node<BlockNodeData>,
  sourceHandle: string,
  targetNode: Node<BlockNodeData>,
  targetHandle: string,
): Edge {
  return {
    id: `e-${sourceNode.id}-${targetNode.id}`,
    source: sourceNode.id,
    target: targetNode.id,
    sourceHandle,
    targetHandle,
    type: 'smoothstep',
    style: { strokeWidth: 2, stroke: '#94A3B8' },
  };
}

// ---------------------------------------------------------------------------
// Template 1: OLTP Balanced
// ---------------------------------------------------------------------------

function buildOLTP(): DesignTemplate {
  const schema = makeNode('schema_definition', 50, 80, {
    tableName: 'users',
    columns: 'id:int,name:varchar,email:varchar,created_at:timestamp',
  });
  const heap = makeNode('heap_storage', 320, 80, { pageSize: 8192, fillFactor: 90 });
  const btree = makeNode('btree_index', 320, 260, { keyColumn: 'id', fanout: 128, unique: true });
  const buffer = makeNode('lru_buffer', 600, 80, { size: 128, pageSize: 8192 });
  const seqScan = makeNode('sequential_scan', 600, 260, { prefetchPages: 32 });

  const edges = [
    makeEdge(schema, 'schema', heap, 'records'),
    makeEdge(heap, 'stored', btree, 'records'),
    makeEdge(heap, 'stored', buffer, 'requests'),
    makeEdge(buffer, 'pages', seqScan, 'records'),
  ];

  return {
    id: 'tpl-oltp',
    name: 'OLTP Balanced',
    description:
      'A balanced design for mixed read/write transactional workloads. Uses heap storage with a B-tree index and LRU buffer pool.',
    category: 'oltp',
    tags: ['transactions', 'balanced', 'B-tree', 'buffer pool'],
    nodes: [schema, heap, btree, buffer, seqScan],
    edges,
    workload: {
      name: 'OLTP Mixed',
      operations: [
        { id: 'w1', type: 'SELECT', weight: 50, template: 'SELECT * FROM {table} WHERE id = ?' },
        { id: 'w2', type: 'UPDATE', weight: 30, template: 'UPDATE {table} SET col = ? WHERE id = ?' },
        { id: 'w3', type: 'INSERT', weight: 20, template: 'INSERT INTO {table} VALUES (?)' },
      ],
      distribution: 'zipfian',
      concurrency: 100,
      totalOperations: 10000,
    },
  };
}

// ---------------------------------------------------------------------------
// Template 2: Write-Heavy (Logging)
// ---------------------------------------------------------------------------

function buildWriteHeavy(): DesignTemplate {
  const schema = makeNode('schema_definition', 50, 120, {
    tableName: 'events',
    columns: 'id:int,timestamp:bigint,type:varchar,payload:text',
  });
  const lsm = makeNode('lsm_tree', 320, 50, { memtableSize: 128, levelMultiplier: 10, bloomFilterBits: 10 });
  const wal = makeNode('wal', 320, 260, { bufferSize: 32, syncMode: 'fdatasync' });
  const filter = makeNode('filter', 600, 50, { predicate: 'type = "error"' });

  const edges = [
    makeEdge(schema, 'schema', lsm, 'records'),
    makeEdge(schema, 'schema', wal, 'records'),
    makeEdge(lsm, 'stored', filter, 'records'),
  ];

  return {
    id: 'tpl-write-heavy',
    name: 'Write-Heavy (Logging)',
    description:
      'Optimized for high insert throughput. Uses an LSM tree for fast writes with a WAL for durability. Ideal for logging and event streaming.',
    category: 'write-heavy',
    tags: ['LSM tree', 'WAL', 'logging', 'insert-heavy'],
    nodes: [schema, lsm, wal, filter],
    edges,
    workload: {
      name: 'Write-Heavy',
      operations: [
        { id: 'w1', type: 'INSERT', weight: 80, template: 'INSERT INTO {table} VALUES (?)' },
        { id: 'w2', type: 'SELECT', weight: 20, template: 'SELECT * FROM {table} WHERE id = ?' },
      ],
      distribution: 'latest',
      concurrency: 200,
      totalOperations: 50000,
    },
  };
}

// ---------------------------------------------------------------------------
// Template 3: Read-Heavy (Analytics)
// ---------------------------------------------------------------------------

function buildReadHeavy(): DesignTemplate {
  const schema = makeNode('schema_definition', 50, 120, {
    tableName: 'orders',
    columns: 'id:int,customer_id:int,amount:decimal,status:varchar,created_at:timestamp',
  });
  const clustered = makeNode('clustered_storage', 320, 50, { clusterKey: 'created_at', pageSize: 16384 });
  const covering = makeNode('covering_index', 320, 260, {
    keyColumns: 'customer_id',
    includeColumns: 'amount,status',
  });
  const buffer = makeNode('lru_buffer', 600, 50, { size: 512, pageSize: 16384 });
  const sort = makeNode('sort', 600, 260, { sortColumns: 'created_at DESC', memoryLimit: 512 });

  const edges = [
    makeEdge(schema, 'schema', clustered, 'records'),
    makeEdge(clustered, 'stored', covering, 'records'),
    makeEdge(clustered, 'stored', buffer, 'requests'),
    makeEdge(buffer, 'pages', sort, 'records'),
  ];

  return {
    id: 'tpl-read-heavy',
    name: 'Read-Heavy (Analytics)',
    description:
      'Optimized for query performance. Uses clustered storage with a covering index and a large buffer pool. Great for analytics and reporting.',
    category: 'read-heavy',
    tags: ['analytics', 'covering index', 'clustered', 'large buffer'],
    nodes: [schema, clustered, covering, buffer, sort],
    edges,
    workload: {
      name: 'Read-Heavy Analytics',
      operations: [
        { id: 'w1', type: 'SELECT', weight: 70, template: 'SELECT * FROM {table} WHERE customer_id = ?' },
        { id: 'w2', type: 'SCAN', weight: 25, template: 'SELECT * FROM {table} WHERE created_at BETWEEN ? AND ?' },
        { id: 'w3', type: 'UPDATE', weight: 5, template: 'UPDATE {table} SET status = ? WHERE id = ?' },
      ],
      distribution: 'zipfian',
      concurrency: 50,
      totalOperations: 20000,
    },
  };
}

// ---------------------------------------------------------------------------
// Template 4: Concurrent MVCC
// ---------------------------------------------------------------------------

function buildConcurrentMVCC(): DesignTemplate {
  const schema = makeNode('schema_definition', 50, 120, {
    tableName: 'accounts',
    columns: 'id:int,balance:decimal,updated_at:timestamp',
  });
  const heap = makeNode('heap_storage', 320, 50, { pageSize: 8192, fillFactor: 85 });
  const mvcc = makeNode('mvcc', 320, 260, { isolationLevel: 'snapshot', gcInterval: 2000 });
  const btree = makeNode('btree_index', 600, 50, { keyColumn: 'id', fanout: 256, unique: true });
  const wal = makeNode('wal', 600, 260, { bufferSize: 16, syncMode: 'fsync' });

  const edges = [
    makeEdge(schema, 'schema', heap, 'records'),
    makeEdge(heap, 'stored', btree, 'records'),
    makeEdge(schema, 'schema', wal, 'records'),
  ];

  return {
    id: 'tpl-concurrent',
    name: 'Concurrent (MVCC)',
    description:
      'Designed for high-concurrency transactional workloads. Uses MVCC for snapshot isolation, a B-tree index, and WAL for crash recovery.',
    category: 'oltp',
    tags: ['MVCC', 'concurrency', 'snapshot isolation', 'WAL'],
    nodes: [schema, heap, mvcc, btree, wal],
    edges,
    workload: {
      name: 'Concurrent OLTP',
      operations: [
        { id: 'w1', type: 'SELECT', weight: 40, template: 'SELECT * FROM {table} WHERE id = ?' },
        { id: 'w2', type: 'UPDATE', weight: 40, template: 'UPDATE {table} SET balance = ? WHERE id = ?' },
        { id: 'w3', type: 'INSERT', weight: 20, template: 'INSERT INTO {table} VALUES (?)' },
      ],
      distribution: 'zipfian',
      concurrency: 200,
      totalOperations: 20000,
    },
  };
}

// ---------------------------------------------------------------------------
// All templates
// ---------------------------------------------------------------------------

export function getTemplates(): DesignTemplate[] {
  // Reset node sequence for deterministic IDs
  nodeSeq = 0;
  return [buildOLTP(), buildWriteHeavy(), buildReadHeavy(), buildConcurrentMVCC()];
}

export const TEMPLATE_CATEGORY_LABELS: Record<TemplateCategory, string> = {
  oltp: 'OLTP',
  olap: 'OLAP',
  'write-heavy': 'Write-Heavy',
  'read-heavy': 'Read-Heavy',
};

export const TEMPLATE_CATEGORY_COLORS: Record<TemplateCategory, string> = {
  oltp: '#3B82F6',
  olap: '#06B6D4',
  'write-heavy': '#10B981',
  'read-heavy': '#8B5CF6',
};
