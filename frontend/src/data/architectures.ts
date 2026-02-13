import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import { CATEGORY_COLORS } from '@/types';
import { getBlockDefinition } from '@/types/blocks';
import type { Workload, OperationType } from '@/stores/workloadStore';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ArchitectureCategory =
  | 'relational'
  | 'nosql-wide'
  | 'nosql-document'
  | 'nosql-kv'
  | 'embedded'
  | 'storage-engine';

export const ARCHITECTURE_CATEGORY_LABELS: Record<ArchitectureCategory, string> = {
  relational: 'Relational',
  'nosql-wide': 'NoSQL Wide-Column',
  'nosql-document': 'NoSQL Document',
  'nosql-kv': 'NoSQL Key-Value',
  embedded: 'Embedded',
  'storage-engine': 'Storage Engine',
};

export interface ArchitectureAnnotation {
  blockType: string;
  title: string;
  explanation: string;
  realWorldDetail?: string;
}

export interface DBArchitecture {
  id: string;
  name: string;
  subtitle: string;
  logo: string;
  category: ArchitectureCategory;
  description: string;
  whyThisArchitecture: string;
  keyInsight: string;
  concepts: string[];
  nodes: Node<BlockNodeData>[];
  edges: Edge[];
  workload: Workload;
  annotations: ArchitectureAnnotation[];
}

// ---------------------------------------------------------------------------
// Helper (similar to templates.ts)
// ---------------------------------------------------------------------------

let nodeSeq = 0;

function makeNode(
  blockType: string,
  x: number,
  y: number,
  paramOverrides?: Record<string, string | number | boolean>,
): Node<BlockNodeData> {
  const def = getBlockDefinition(blockType);
  if (!def) throw new Error(`Unknown block type for architecture: ${blockType}`);

  const parameters: Record<string, string | number | boolean> = {};
  for (const p of def.parameters) {
    parameters[p.name] = p.default;
  }
  if (paramOverrides) {
    Object.assign(parameters, paramOverrides);
  }

  const id = `arch-${blockType}-${++nodeSeq}`;
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

const DEFAULT_TEMPLATES: Record<OperationType, string> = {
  INSERT: 'INSERT INTO {table} VALUES (?)',
  SELECT: 'SELECT * FROM {table} WHERE id = ?',
  UPDATE: 'UPDATE {table} SET col = ? WHERE id = ?',
  DELETE: 'DELETE FROM {table} WHERE id = ?',
  SCAN: 'SELECT * FROM {table} WHERE id BETWEEN ? AND ?',
};

function makeWorkload(
  ops: { type: OperationType; weight: number }[],
  distribution: Workload['distribution'],
  totalOperations: number,
  concurrency = 100,
): Workload {
  return {
    name: 'Architecture Workload',
    operations: ops.map((op, i) => ({
      id: `arch-op-${i + 1}`,
      type: op.type,
      weight: op.weight,
      template: DEFAULT_TEMPLATES[op.type],
    })),
    distribution,
    concurrency,
    totalOperations,
  };
}

// ---------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------

function buildPostgreSQL(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'users', columns: 'id:int,name:varchar,email:varchar' });
  const heap = makeNode('heap_storage', 250, 200, { pageSize: 8192, fillFactor: 90 });
  const btree = makeNode('btree_index', 250, 0, { keyColumn: 'id', fanout: 128 });
  const buffer = makeNode('lru_buffer', 500, 100, { size: 128 });
  const wal = makeNode('wal', 500, 300, { bufferSize: 16, syncMode: 'fsync' });
  const mvcc = makeNode('mvcc', 750, 200, { isolationLevel: 'snapshot', gcInterval: 1000 });

  return {
    id: 'postgresql',
    name: 'PostgreSQL',
    subtitle: 'The world\'s most advanced open-source RDBMS',
    logo: '🐘',
    category: 'relational',
    description: 'PostgreSQL uses a heap-based storage engine with B-tree indexes, a shared buffer pool with clock-sweep eviction, write-ahead logging for durability, and MVCC for concurrency control.',
    whyThisArchitecture: 'PostgreSQL chose heap storage (not clustered) so that multiple indexes can be created without reorganizing data. MVCC avoids reader-writer contention entirely — readers never block writers and vice versa. The WAL ensures crash recovery without sacrificing performance.',
    keyInsight: 'MVCC + WAL = readers never block writers, and no data is lost on crash. This is why PostgreSQL handles concurrent workloads so well.',
    concepts: ['Heap Storage', 'B-tree', 'MVCC', 'WAL', 'Buffer Pool'],
    nodes: [schema, heap, btree, buffer, wal, mvcc],
    edges: [
      makeEdge(schema, 'schema', heap, 'records'),
      makeEdge(heap, 'stored', btree, 'records'),
      makeEdge(heap, 'stored', buffer, 'requests'),
      makeEdge(heap, 'stored', wal, 'records'),
      makeEdge(wal, 'logged', mvcc, 'records'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 60 }, { type: 'INSERT', weight: 20 }, { type: 'UPDATE', weight: 15 }, { type: 'DELETE', weight: 5 }],
      'zipfian', 1000,
    ),
    annotations: [
      {
        blockType: 'heap_storage',
        title: 'Why Heap Storage?',
        explanation: 'PostgreSQL stores records unordered in heap files. Unlike InnoDB\'s clustered index, this allows multiple secondary indexes without data duplication.',
        realWorldDetail: 'PostgreSQL pages are 8KB by default. The fill factor (90%) leaves room for HOT updates — updates that don\'t change indexed columns can be stored on the same page.',
      },
      {
        blockType: 'btree_index',
        title: 'B-tree: The Default Index',
        explanation: 'PostgreSQL creates a B-tree index for every primary key and UNIQUE constraint. B-trees support both equality and range queries efficiently.',
        realWorldDetail: 'PostgreSQL B-trees have a typical fanout of ~300 entries per page, so even 100M rows need only 3–4 levels of tree.',
      },
      {
        blockType: 'mvcc',
        title: 'MVCC: No Read Locks',
        explanation: 'Each transaction sees a snapshot of the database. Old row versions are kept until no transaction needs them. This is why PostgreSQL needs VACUUM — to clean up dead tuples.',
        realWorldDetail: 'PostgreSQL stores version info (xmin/xmax) directly in each tuple header, unlike MySQL which uses an undo log.',
      },
      {
        blockType: 'wal',
        title: 'Write-Ahead Log',
        explanation: 'Every change is written to the WAL before modifying data pages. On crash, PostgreSQL replays the WAL to recover. The WAL is also used for streaming replication.',
        realWorldDetail: 'PostgreSQL\'s WAL uses 16MB segment files by default. The checkpointer periodically writes dirty pages to disk so the WAL can be recycled.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// MySQL / InnoDB
// ---------------------------------------------------------------------------

function buildMySQL(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'orders', columns: 'id:int,customer_id:int,total:decimal' });
  const clustered = makeNode('clustered_storage', 250, 200, { clusterKey: 'id', pageSize: 16384 });
  const btree = makeNode('btree_index', 250, 0, { keyColumn: 'customer_id', fanout: 128 });
  const buffer = makeNode('lru_buffer', 500, 100, { size: 128 });
  const wal = makeNode('wal', 500, 300, { bufferSize: 16, syncMode: 'fsync' });
  const lock = makeNode('row_lock', 750, 200, { lockTimeout: 5000, deadlockDetection: true });

  return {
    id: 'mysql-innodb',
    name: 'MySQL / InnoDB',
    subtitle: 'The most popular open-source database',
    logo: '🐬',
    category: 'relational',
    description: 'InnoDB uses clustered storage where the primary key B-tree IS the data. Secondary indexes store the primary key as a pointer. Combined with row-level locking and a large buffer pool.',
    whyThisArchitecture: 'InnoDB chose clustered indexes because primary key lookups are extremely fast — the data is right there in the leaf. The tradeoff is that secondary index lookups need a second lookup (back to the clustered index) unless the query is covered.',
    keyInsight: 'In InnoDB, the data IS the index. The primary key B-tree\'s leaf nodes contain the actual row data, making PK lookups a single traversal.',
    concepts: ['Clustered Index', 'B-tree', 'Row Locking', 'WAL', 'Buffer Pool'],
    nodes: [schema, clustered, btree, buffer, wal, lock],
    edges: [
      makeEdge(schema, 'schema', clustered, 'records'),
      makeEdge(clustered, 'stored', btree, 'records'),
      makeEdge(clustered, 'stored', buffer, 'requests'),
      makeEdge(clustered, 'stored', wal, 'records'),
      makeEdge(wal, 'logged', lock, 'records'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 50 }, { type: 'INSERT', weight: 25 }, { type: 'UPDATE', weight: 20 }, { type: 'DELETE', weight: 5 }],
      'zipfian', 1000,
    ),
    annotations: [
      {
        blockType: 'clustered_storage',
        title: 'Clustered Index = Data IS the Index',
        explanation: 'InnoDB stores rows sorted by primary key in a B-tree structure. The leaf pages of the primary key index contain the actual row data.',
        realWorldDetail: 'InnoDB pages are 16KB (double PostgreSQL\'s 8KB). Auto-increment PKs are ideal because they append to the end of the B-tree without page splits.',
      },
      {
        blockType: 'btree_index',
        title: 'Secondary Indexes Point to PK',
        explanation: 'Secondary indexes store the primary key value (not a physical pointer). Lookups through secondary indexes require a "bookmark lookup" back to the clustered index.',
        realWorldDetail: 'This means wider primary keys make ALL secondary indexes larger. Using UUID as PK is expensive in InnoDB.',
      },
      {
        blockType: 'row_lock',
        title: 'Row-Level Locking (not MVCC for writes)',
        explanation: 'InnoDB uses row-level locks for writes and MVCC for reads. Writers block other writers but not readers. Deadlock detection runs automatically.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------

function buildSQLite(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 150, { tableName: 'data', columns: 'id:int,value:text' });
  const btree = makeNode('btree_index', 300, 150, { keyColumn: 'id', fanout: 64 });
  const wal = makeNode('wal', 550, 150, { bufferSize: 1, syncMode: 'fsync' });

  return {
    id: 'sqlite',
    name: 'SQLite',
    subtitle: 'The most widely deployed database in the world',
    logo: '🪶',
    category: 'embedded',
    description: 'SQLite stores the entire database in a single file. Tables are B-trees where the rowid is the key. WAL mode enables concurrent readers with a single writer.',
    whyThisArchitecture: 'SQLite prioritizes simplicity and portability over concurrency. A single file means zero configuration. The B-tree is the storage — there\'s no separate heap. WAL mode was added later to allow readers during writes.',
    keyInsight: 'SQLite\'s genius is that the B-tree IS the storage engine. Every table is a B-tree keyed by rowid. This eliminates the need for a separate storage layer.',
    concepts: ['B-tree Storage', 'WAL', 'Single-File', 'Embedded'],
    nodes: [schema, btree, wal],
    edges: [
      makeEdge(schema, 'schema', btree, 'records'),
      makeEdge(btree, 'lookup_results', wal, 'records'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 70 }, { type: 'INSERT', weight: 20 }, { type: 'UPDATE', weight: 10 }],
      'uniform', 500,
    ),
    annotations: [
      {
        blockType: 'btree_index',
        title: 'B-tree IS the Storage',
        explanation: 'In SQLite, every table is stored as a B-tree. The leaf pages contain the actual row data. There is no separate heap file.',
        realWorldDetail: 'SQLite uses 4KB pages by default (configurable). "WITHOUT ROWID" tables use a separate B-tree keyed by the declared PRIMARY KEY.',
      },
      {
        blockType: 'wal',
        title: 'WAL Mode',
        explanation: 'In WAL mode, changes are appended to a separate WAL file. Readers see the database as it was before the write started. Checkpointing merges the WAL back into the main file.',
        realWorldDetail: 'WAL mode allows concurrent readers with one writer. Before WAL mode, SQLite used rollback journals which locked the entire database during writes.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Cassandra
// ---------------------------------------------------------------------------

function buildCassandra(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'events', columns: 'partition_key:text,timestamp:bigint,data:text' });
  const lsm = makeNode('lsm_tree', 250, 200, { memtableSize: 64, levelMultiplier: 10 });
  const bloom = makeNode('bloom_filter', 500, 50, { num_bits: 100000, num_hash_functions: 10 });
  const partitioner = makeNode('hash_partitioner', 500, 200, { num_partitions: 8 });
  const replication = makeNode('replication', 750, 200, { replication_factor: 3, consistency_level: 'quorum' });

  return {
    id: 'cassandra',
    name: 'Apache Cassandra',
    subtitle: 'Distributed wide-column store for massive scale',
    logo: '👁',
    category: 'nosql-wide',
    description: 'Cassandra combines LSM-trees for fast writes, Bloom filters to avoid unnecessary reads, hash partitioning for data distribution, and tunable replication for fault tolerance.',
    whyThisArchitecture: 'Cassandra was designed at Facebook for inbox search — a write-heavy, distributed workload. LSM-trees make writes sequential (fast on disks). Hash partitioning distributes data evenly. Tunable consistency lets you choose between speed and correctness per query.',
    keyInsight: 'Write-optimized everywhere: LSM-trees make writes sequential, hash partitioning avoids hotspots, and Bloom filters reduce read amplification. The tradeoff is complexity in reads and compaction.',
    concepts: ['LSM-Tree', 'Bloom Filter', 'Hash Partitioning', 'Replication', 'Tunable Consistency'],
    nodes: [schema, lsm, bloom, partitioner, replication],
    edges: [
      makeEdge(schema, 'schema', lsm, 'records'),
      makeEdge(lsm, 'stored', bloom, 'requests'),
      makeEdge(lsm, 'stored', partitioner, 'records'),
      makeEdge(partitioner, 'partitioned', replication, 'requests'),
    ],
    workload: makeWorkload(
      [{ type: 'INSERT', weight: 60 }, { type: 'SELECT', weight: 30 }, { type: 'UPDATE', weight: 10 }],
      'uniform', 1000,
    ),
    annotations: [
      {
        blockType: 'lsm_tree',
        title: 'LSM-Tree: Write-Optimized',
        explanation: 'Writes go to an in-memory memtable, then flush to immutable SSTables on disk. This turns random writes into sequential I/O.',
        realWorldDetail: 'Cassandra\'s memtable flushes at ~128MB by default. SSTables are compacted using Size-Tiered (write-optimized) or Leveled (read-optimized) compaction.',
      },
      {
        blockType: 'bloom_filter',
        title: 'Bloom Filter Per SSTable',
        explanation: 'Before reading an SSTable from disk, Cassandra checks the Bloom filter. If it says "no", the SSTable is skipped entirely.',
        realWorldDetail: 'Cassandra uses ~10 bits per key by default, giving a false positive rate of ~1%. This dramatically reduces read amplification in LSM-trees.',
      },
      {
        blockType: 'hash_partitioner',
        title: 'Murmur3 Partitioning',
        explanation: 'Every row\'s partition key is hashed to determine which node stores it. This distributes data evenly across the cluster.',
        realWorldDetail: 'Cassandra uses consistent hashing with virtual nodes (vnodes). Each node owns multiple token ranges for better load balancing.',
      },
      {
        blockType: 'replication',
        title: 'Tunable Consistency',
        explanation: 'You choose consistency per query: ONE (fast but may be stale), QUORUM (balanced), ALL (strong but slow). QUORUM read + QUORUM write guarantees linearizability.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// RocksDB
// ---------------------------------------------------------------------------

function buildRocksDB(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'kv_store', columns: 'key:bytes,value:bytes' });
  const lsm = makeNode('lsm_tree', 250, 200, { memtableSize: 64, levelMultiplier: 10, bloomFilterBits: 10 });
  const bloom = makeNode('bloom_filter', 500, 50, { num_bits: 100000, num_hash_functions: 10 });
  const buffer = makeNode('lru_buffer', 500, 200, { size: 256 });
  const dict = makeNode('dictionary_encoding', 500, 350, { max_dictionary_size: 16384 });

  return {
    id: 'rocksdb',
    name: 'RocksDB',
    subtitle: 'Embeddable persistent key-value store for fast storage',
    logo: '🪨',
    category: 'storage-engine',
    description: 'RocksDB is an LSM-tree engine optimized for fast storage (SSDs). It adds Bloom filters per SSTable, a block cache, and dictionary-based compression to minimize I/O and storage.',
    whyThisArchitecture: 'Facebook built RocksDB to optimize LevelDB for SSDs and large datasets. LSM-trees provide fast writes, Bloom filters reduce read amplification, block cache accelerates hot reads, and compression reduces storage costs.',
    keyInsight: 'RocksDB is an LSM-tree engine with aggressive optimization at every layer: Bloom filters skip SSTables, block cache avoids disk reads, and dictionary compression saves 30-50% storage.',
    concepts: ['LSM-Tree', 'Bloom Filter', 'Block Cache', 'Compression', 'Compaction'],
    nodes: [schema, lsm, bloom, buffer, dict],
    edges: [
      makeEdge(schema, 'schema', lsm, 'records'),
      makeEdge(lsm, 'stored', bloom, 'requests'),
      makeEdge(lsm, 'stored', buffer, 'requests'),
      makeEdge(lsm, 'stored', dict, 'records'),
    ],
    workload: makeWorkload(
      [{ type: 'INSERT', weight: 50 }, { type: 'SELECT', weight: 40 }, { type: 'UPDATE', weight: 10 }],
      'zipfian', 1000,
    ),
    annotations: [
      {
        blockType: 'lsm_tree',
        title: 'LSM-Tree with Leveled Compaction',
        explanation: 'RocksDB defaults to leveled compaction: each level is 10x the size of the previous. This limits read amplification to ~10x at the cost of more write amplification.',
        realWorldDetail: 'RocksDB supports multiple compaction styles: Level (default), Universal (write-optimized), and FIFO (TTL-based).',
      },
      {
        blockType: 'bloom_filter',
        title: 'Bloom Filters Reduce Read I/O',
        explanation: 'Each SSTable has a Bloom filter. A point query checks filters from newest to oldest level, skipping SSTables that definitely don\'t have the key.',
        realWorldDetail: 'With 10 bits/key, the false positive rate is ~1%. RocksDB also supports "prefix Bloom filters" for range-prefix queries.',
      },
      {
        blockType: 'dictionary_encoding',
        title: 'Compression with Shared Dictionary',
        explanation: 'RocksDB compresses SSTable blocks using ZSTD with a shared dictionary trained on sample data. This achieves much better compression than per-block compression alone.',
        realWorldDetail: 'Dictionary compression can improve ratios by 30-50% for structured data. The dictionary is stored in the SSTable metadata.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Redis
// ---------------------------------------------------------------------------

function buildRedis(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 150, { tableName: 'cache', columns: 'key:text,value:text' });
  const heap = makeNode('heap_storage', 250, 150, { pageSize: 4096, fillFactor: 100 });
  const hash = makeNode('hash_index', 500, 50, { keyColumn: 'key', buckets: 65536 });
  const wal = makeNode('wal', 500, 250, { bufferSize: 1, syncMode: 'fsync' });

  return {
    id: 'redis',
    name: 'Redis',
    subtitle: 'In-memory data structure store',
    logo: '🔴',
    category: 'nosql-kv',
    description: 'Redis keeps all data in memory with a hash table for O(1) key lookups. An append-only file (AOF) or RDB snapshots provide optional persistence.',
    whyThisArchitecture: 'Redis was designed for speed above all else. In-memory storage eliminates disk I/O. A hash table gives O(1) lookups. Persistence is optional and asynchronous, trading durability for performance.',
    keyInsight: 'Everything in memory + hash table = O(1) for reads AND writes. Redis trades durability for speed, then offers optional persistence when you need it.',
    concepts: ['In-Memory', 'Hash Table', 'AOF', 'Single-Threaded'],
    nodes: [schema, heap, hash, wal],
    edges: [
      makeEdge(schema, 'schema', heap, 'records'),
      makeEdge(heap, 'stored', hash, 'records'),
      makeEdge(heap, 'stored', wal, 'records'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 70 }, { type: 'INSERT', weight: 20 }, { type: 'UPDATE', weight: 10 }],
      'zipfian', 1000,
    ),
    annotations: [
      {
        blockType: 'heap_storage',
        title: 'Everything in Memory',
        explanation: 'Redis stores all data in RAM. There are no disk reads for queries. The dataset must fit in memory (or use Redis Cluster to shard).',
        realWorldDetail: 'Redis uses jemalloc for memory allocation and can achieve ~100K ops/sec on a single core.',
      },
      {
        blockType: 'hash_index',
        title: 'Hash Table = O(1) Lookups',
        explanation: 'Redis\'s main dictionary is a hash table that provides O(1) average-case lookups, inserts, and deletes.',
        realWorldDetail: 'Redis uses progressive rehashing — when the table needs to grow, it incrementally migrates entries across requests to avoid latency spikes.',
      },
      {
        blockType: 'wal',
        title: 'Append-Only File (AOF)',
        explanation: 'The AOF logs every write command. On restart, Redis replays the AOF to rebuild state. Slower than RDB snapshots but loses less data.',
        realWorldDetail: 'AOF can be configured with fsync every second (default), every write, or never. Redis 7+ uses Multi-Part AOF for faster recovery.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// MongoDB
// ---------------------------------------------------------------------------

function buildMongoDB(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'documents', columns: '_id:objectid,data:json' });
  const heap = makeNode('heap_storage', 250, 200, { pageSize: 4096, fillFactor: 90 });
  const btree = makeNode('btree_index', 250, 0, { keyColumn: '_id', fanout: 128 });
  const buffer = makeNode('lru_buffer', 500, 100, { size: 256 });
  const wal = makeNode('wal', 500, 300, { bufferSize: 16, syncMode: 'fsync' });
  const replication = makeNode('replication', 750, 200, { replication_factor: 3, consistency_level: 'quorum' });

  return {
    id: 'mongodb',
    name: 'MongoDB',
    subtitle: 'Document database with flexible schema',
    logo: '🍃',
    category: 'nosql-document',
    description: 'MongoDB stores BSON documents in WiredTiger (B-tree storage engine). Indexes are B-trees. Replica sets provide automatic failover with configurable write concern.',
    whyThisArchitecture: 'MongoDB prioritizes developer experience with flexible schemas and powerful queries. WiredTiger provides MVCC and compression. Replica sets handle failover automatically.',
    keyInsight: 'MongoDB is a document database built on a traditional B-tree storage engine (WiredTiger). The document model gives flexibility, but under the hood it\'s using the same proven structures as relational databases.',
    concepts: ['Document Store', 'B-tree', 'Replica Sets', 'WAL', 'WiredTiger'],
    nodes: [schema, heap, btree, buffer, wal, replication],
    edges: [
      makeEdge(schema, 'schema', heap, 'records'),
      makeEdge(heap, 'stored', btree, 'records'),
      makeEdge(heap, 'stored', buffer, 'requests'),
      makeEdge(heap, 'stored', wal, 'records'),
      makeEdge(wal, 'logged', replication, 'requests'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 50 }, { type: 'INSERT', weight: 30 }, { type: 'UPDATE', weight: 15 }, { type: 'DELETE', weight: 5 }],
      'zipfian', 1000,
    ),
    annotations: [
      {
        blockType: 'heap_storage',
        title: 'WiredTiger Storage Engine',
        explanation: 'MongoDB switched from the original MMAPv1 to WiredTiger in 3.2. WiredTiger provides document-level concurrency control and compression.',
        realWorldDetail: 'WiredTiger uses snappy compression by default, reducing storage by 50-80% for typical JSON documents.',
      },
      {
        blockType: 'btree_index',
        title: 'B-tree Indexes on Any Field',
        explanation: 'MongoDB supports B-tree indexes on any document field, including nested fields and arrays. The _id field is always indexed.',
        realWorldDetail: 'Compound indexes support multiple fields. Multikey indexes automatically index each element of an array field.',
      },
      {
        blockType: 'replication',
        title: 'Replica Sets',
        explanation: 'A replica set has one primary and multiple secondaries. Writes go to the primary and are replicated to secondaries. Automatic failover elects a new primary if the current one fails.',
        realWorldDetail: 'Write concern "majority" waits for a majority of replicas to acknowledge. Read preference can direct reads to secondaries for scaling.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// DynamoDB
// ---------------------------------------------------------------------------

function buildDynamoDB(): DBArchitecture {
  nodeSeq = 0;
  const schema = makeNode('schema_definition', 0, 200, { tableName: 'items', columns: 'pk:text,sk:text,data:json' });
  const lsm = makeNode('lsm_tree', 250, 200, { memtableSize: 64, levelMultiplier: 10 });
  const partitioner = makeNode('hash_partitioner', 500, 100, { num_partitions: 16 });
  const replication = makeNode('replication', 500, 300, { replication_factor: 3, consistency_level: 'quorum' });
  const buffer = makeNode('lru_buffer', 750, 200, { size: 128 });

  return {
    id: 'dynamodb',
    name: 'Amazon DynamoDB',
    subtitle: 'Fully managed NoSQL with single-digit ms latency at any scale',
    logo: '⚡',
    category: 'nosql-kv',
    description: 'DynamoDB partitions data by hashing the partition key, replicates across 3 AZs automatically, and uses LSM-tree-like storage for fast writes. DAX adds an in-memory cache layer.',
    whyThisArchitecture: 'DynamoDB was designed for predictable performance at any scale. Hash partitioning eliminates hotspots (if you choose your partition key well). Automatic 3-way replication across AZs provides 99.999% availability.',
    keyInsight: 'DynamoDB achieves "infinite scale" by automatically splitting partitions. Each partition is an LSM-tree replicated across 3 AZs. The partition key design is the single most important decision.',
    concepts: ['Hash Partitioning', 'LSM-Tree', 'Replication', 'Managed Service'],
    nodes: [schema, lsm, partitioner, replication, buffer],
    edges: [
      makeEdge(schema, 'schema', lsm, 'records'),
      makeEdge(lsm, 'stored', partitioner, 'records'),
      makeEdge(partitioner, 'partitioned', replication, 'requests'),
      makeEdge(replication, 'replicated', buffer, 'requests'),
    ],
    workload: makeWorkload(
      [{ type: 'SELECT', weight: 50 }, { type: 'INSERT', weight: 35 }, { type: 'UPDATE', weight: 15 }],
      'uniform', 1000,
    ),
    annotations: [
      {
        blockType: 'hash_partitioner',
        title: 'Partition Key = Scale Unit',
        explanation: 'DynamoDB hashes the partition key to determine which storage partition holds the item. Each partition handles up to 3000 RCU or 1000 WCU.',
        realWorldDetail: 'Hot partition keys (e.g., today\'s date) can throttle your table. Adaptive capacity helps but a good key design is still critical.',
      },
      {
        blockType: 'replication',
        title: 'Automatic 3-AZ Replication',
        explanation: 'Every write is replicated to 3 AZs before being acknowledged (for standard tables). This provides 99.999% availability and 11 9\'s of durability.',
        realWorldDetail: 'Strongly consistent reads always go to the leader replica. Eventually consistent reads (default) can go to any replica and are 50% cheaper.',
      },
      {
        blockType: 'lsm_tree',
        title: 'LSM-Tree Storage',
        explanation: 'Each partition uses an LSM-tree-like structure for fast writes. The memtable absorbs writes in memory, then flushes to SSTables on disk.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

export function getArchitectures(): DBArchitecture[] {
  return [
    buildPostgreSQL(),
    buildMySQL(),
    buildSQLite(),
    buildCassandra(),
    buildRocksDB(),
    buildRedis(),
    buildMongoDB(),
    buildDynamoDB(),
  ];
}

export function getArchitectureById(id: string): DBArchitecture | undefined {
  return getArchitectures().find((a) => a.id === id);
}
