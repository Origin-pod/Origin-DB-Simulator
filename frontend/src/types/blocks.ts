import type { BlockCategory, PortDefinition, ParameterDefinition } from './index';

/**
 * Static block type definition - describes a type of block that can be instantiated
 */
export interface BlockDefinition {
  type: string;
  name: string;
  description: string;
  category: BlockCategory;
  icon: string;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  parameters: ParameterDefinition[];
  documentation?: BlockDocumentation;
  references?: BlockReference[];
  metricDefinitions?: BlockMetricInfo[];
}

export interface BlockDocumentation {
  summary: string;
  details?: string;
  examples?: string[];
  seeAlso?: string[];
  // Rich fields populated from WASM block metadata
  overview?: string;
  algorithm?: string;
  complexity?: { time: string; space: string };
  useCases?: string[];
  tradeoffs?: string[];
}

export interface BlockReference {
  refType: 'Paper' | 'Book' | 'Blog' | 'Implementation';
  title: string;
  url?: string;
  citation?: string;
}

export interface BlockMetricInfo {
  id: string;
  name: string;
  type: string;
  unit: string;
  description: string;
}

/**
 * Category metadata for display purposes
 */
export interface CategoryInfo {
  id: BlockCategory;
  name: string;
  description: string;
  icon: string;
  color: string;
}

export const CATEGORIES: CategoryInfo[] = [
  {
    id: 'storage',
    name: 'Storage',
    description: 'Data storage mechanisms',
    icon: 'HardDrive',
    color: '#8B5CF6',
  },
  {
    id: 'index',
    name: 'Index',
    description: 'Index structures for fast lookups',
    icon: 'Binary',
    color: '#3B82F6',
  },
  {
    id: 'buffer',
    name: 'Buffer',
    description: 'Memory management and caching',
    icon: 'Layers',
    color: '#14B8A6',
  },
  {
    id: 'execution',
    name: 'Execution',
    description: 'Query execution operators',
    icon: 'Cpu',
    color: '#EC4899',
  },
  {
    id: 'concurrency',
    name: 'Concurrency',
    description: 'Locking and isolation',
    icon: 'Lock',
    color: '#F59E0B',
  },
  {
    id: 'transaction',
    name: 'Transaction',
    description: 'Transaction management',
    icon: 'GitBranch',
    color: '#6366F1',
  },
  {
    id: 'compression',
    name: 'Compression',
    description: 'Data compression techniques',
    icon: 'Archive',
    color: '#84CC16',
  },
  {
    id: 'partitioning',
    name: 'Partitioning',
    description: 'Data partitioning strategies',
    icon: 'Grid3x3',
    color: '#F97316',
  },
  {
    id: 'optimization',
    name: 'Optimization',
    description: 'Query optimization components',
    icon: 'Sparkles',
    color: '#06B6D4',
  },
  {
    id: 'distribution',
    name: 'Distribution',
    description: 'Distributed system components',
    icon: 'Network',
    color: '#A855F7',
  },
];

/**
 * Block Registry - all available block definitions
 */
export const BLOCK_REGISTRY: BlockDefinition[] = [
  // ============== STORAGE BLOCKS ==============
  {
    type: 'schema_definition',
    name: 'Schema Definition',
    description: 'Define table structure with columns and types',
    category: 'storage',
    icon: 'Database',
    inputs: [],
    outputs: [
      {
        name: 'schema',
        type: 'output',
        dataType: 'Schema',
        description: 'Table schema definition',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'tableName',
        type: 'string',
        default: 'users',
        description: 'Name of the table',
        constraints: { pattern: '^[a-zA-Z_][a-zA-Z0-9_]*$' },
        uiHint: 'input',
      },
      {
        name: 'columns',
        type: 'string',
        default: 'id:int,name:varchar,email:varchar',
        description: 'Column definitions (name:type pairs)',
        uiHint: 'input',
      },
    ],
    documentation: {
      summary: 'Defines the structure of a database table',
      details: 'The schema definition block is the starting point for most database designs. It specifies the columns, their data types, and constraints.',
      examples: ['id:int,name:varchar(255),created_at:timestamp'],
    },
  },
  {
    type: 'heap_storage',
    name: 'Heap File Storage',
    description: 'Unordered page-based storage for records',
    category: 'storage',
    icon: 'HardDrive',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to store',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'stored',
        type: 'output',
        dataType: 'DataStream',
        description: 'Access to stored records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'pageSize',
        type: 'number',
        default: 8192,
        description: 'Size of each page in bytes',
        constraints: { min: 1024, max: 65536, step: 1024 },
        uiHint: 'slider',
      },
      {
        name: 'fillFactor',
        type: 'number',
        default: 90,
        description: 'Percentage of page to fill before creating new page',
        constraints: { min: 50, max: 100, step: 5 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Simple heap file storage without ordering',
      details: 'Heap storage is the simplest form of storage. Records are inserted in any available space. Good for write-heavy workloads but requires full scans for queries.',
    },
  },
  {
    type: 'clustered_storage',
    name: 'Clustered Storage',
    description: 'Records stored in sorted order by key',
    category: 'storage',
    icon: 'Database',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to store',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'stored',
        type: 'output',
        dataType: 'DataStream',
        description: 'Access to stored records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'clusterKey',
        type: 'string',
        default: 'id',
        description: 'Column to cluster by',
        uiHint: 'input',
      },
      {
        name: 'pageSize',
        type: 'number',
        default: 8192,
        description: 'Size of each page in bytes',
        constraints: { min: 1024, max: 65536, step: 1024 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Storage with records physically sorted by cluster key',
      details: 'Clustered storage keeps records in sorted order, which improves range query performance but may slow down inserts.',
    },
  },
  {
    type: 'columnar_storage',
    name: 'Columnar Storage',
    description: 'Column-oriented storage for analytical workloads',
    category: 'storage',
    icon: 'Grid3x3',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to store in columnar format',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'projected',
        type: 'output',
        dataType: 'DataStream',
        description: 'Access to projected columns',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'projection',
        type: 'string',
        default: '',
        description: 'Comma-separated columns to read (empty = all)',
        uiHint: 'input',
      },
    ],
    documentation: {
      summary: 'Column-oriented storage for analytical workloads',
      details: 'Stores data in column-major format — each column is a contiguous array. Ideal for OLAP queries that scan many rows but only a few columns, with excellent compression ratios from low-cardinality data.',
    },
  },
  {
    type: 'lsm_tree',
    name: 'LSM Tree Storage',
    description: 'Log-structured merge tree for write-optimized storage',
    category: 'storage',
    icon: 'Layers',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to store',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'stored',
        type: 'output',
        dataType: 'DataStream',
        description: 'Access to stored records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'memtableSize',
        type: 'number',
        default: 64,
        description: 'Size of in-memory table (MB)',
        constraints: { min: 1, max: 512, step: 1 },
        uiHint: 'slider',
      },
      {
        name: 'levelMultiplier',
        type: 'number',
        default: 10,
        description: 'Size ratio between levels',
        constraints: { min: 2, max: 20, step: 1 },
        uiHint: 'slider',
      },
      {
        name: 'bloomFilterBits',
        type: 'number',
        default: 10,
        description: 'Bits per key for bloom filter',
        constraints: { min: 1, max: 20, step: 1 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Write-optimized storage using log-structured merge trees',
      details: 'LSM trees batch writes in memory and periodically flush to disk. Excellent for write-heavy workloads but may have higher read amplification.',
    },
  },

  // ============== INDEX BLOCKS ==============
  {
    type: 'btree_index',
    name: 'B-tree Index',
    description: 'Balanced tree index for range queries',
    category: 'index',
    icon: 'Binary',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to index',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'lookup_results',
        type: 'output',
        dataType: 'SingleValue',
        description: 'Index lookup results',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'keyColumn',
        type: 'string',
        default: 'id',
        description: 'Column to index',
        uiHint: 'input',
      },
      {
        name: 'fanout',
        type: 'number',
        default: 128,
        description: 'Number of children per node',
        constraints: { min: 4, max: 512, step: 4 },
        uiHint: 'slider',
      },
      {
        name: 'unique',
        type: 'boolean',
        default: false,
        description: 'Enforce unique values',
        uiHint: 'checkbox',
      },
    ],
    documentation: {
      summary: 'B-tree index for efficient point and range lookups',
      details: 'B-trees are the most common index structure in databases. They provide O(log n) lookups and support range queries efficiently.',
    },
  },
  {
    type: 'hash_index',
    name: 'Hash Index',
    description: 'Hash-based index for point lookups',
    category: 'index',
    icon: 'Hash',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to index',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'lookup_results',
        type: 'output',
        dataType: 'SingleValue',
        description: 'Index lookup results',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'keyColumn',
        type: 'string',
        default: 'id',
        description: 'Column to index',
        uiHint: 'input',
      },
      {
        name: 'buckets',
        type: 'number',
        default: 1024,
        description: 'Number of hash buckets',
        constraints: { min: 16, max: 65536, step: 16 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Hash index for O(1) point lookups',
      details: 'Hash indexes provide constant-time lookups but do not support range queries. Best for equality predicates.',
    },
  },
  {
    type: 'covering_index',
    name: 'Covering Index',
    description: 'Index that includes additional columns',
    category: 'index',
    icon: 'FileStack',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to index',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'index_results',
        type: 'output',
        dataType: 'DataStream',
        description: 'Index lookup with included columns',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'keyColumns',
        type: 'string',
        default: 'id',
        description: 'Columns to index (comma-separated)',
        uiHint: 'input',
      },
      {
        name: 'includeColumns',
        type: 'string',
        default: 'name,email',
        description: 'Additional columns to include',
        uiHint: 'input',
      },
    ],
    documentation: {
      summary: 'Index with additional columns to avoid table lookups',
      details: 'Covering indexes include extra columns so queries can be answered from the index alone without accessing the main table.',
    },
  },

  // ============== BUFFER BLOCKS ==============
  {
    type: 'lru_buffer',
    name: 'LRU Buffer Pool',
    description: 'Page cache with LRU eviction policy',
    category: 'buffer',
    icon: 'Layers',
    inputs: [
      {
        name: 'requests',
        type: 'input',
        dataType: 'DataStream',
        description: 'Page requests',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'pages',
        type: 'output',
        dataType: 'DataStream',
        description: 'Cached pages',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'size',
        type: 'number',
        default: 128,
        description: 'Buffer pool size (MB)',
        constraints: { min: 1, max: 4096, step: 1 },
        uiHint: 'slider',
      },
      {
        name: 'pageSize',
        type: 'number',
        default: 8192,
        description: 'Page size (bytes)',
        constraints: { min: 1024, max: 65536, step: 1024 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Buffer pool with Least Recently Used eviction',
      details: 'LRU evicts the least recently accessed pages when the buffer is full. Simple and effective for most workloads.',
    },
  },
  {
    type: 'clock_buffer',
    name: 'Clock Buffer Pool',
    description: 'Page cache with clock eviction (approximates LRU)',
    category: 'buffer',
    icon: 'Clock',
    inputs: [
      {
        name: 'requests',
        type: 'input',
        dataType: 'DataStream',
        description: 'Page requests',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'pages',
        type: 'output',
        dataType: 'DataStream',
        description: 'Cached pages',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'size',
        type: 'number',
        default: 128,
        description: 'Buffer pool size (MB)',
        constraints: { min: 1, max: 4096, step: 1 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Buffer pool with Clock eviction algorithm',
      details: 'Clock is an approximation of LRU that is more efficient to implement. Uses a circular buffer with reference bits.',
    },
  },

  // ============== EXECUTION BLOCKS ==============
  {
    type: 'sequential_scan',
    name: 'Sequential Scan',
    description: 'Full table scan operator',
    category: 'execution',
    icon: 'Cpu',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Storage to scan',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'results',
        type: 'output',
        dataType: 'DataStream',
        description: 'Scanned records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'prefetchPages',
        type: 'number',
        default: 32,
        description: 'Number of pages to prefetch',
        constraints: { min: 1, max: 128, step: 1 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Reads all records from storage sequentially',
      details: 'Sequential scan reads every page in the table. Efficient for reading large portions of data but slow for selective queries.',
    },
  },
  {
    type: 'index_scan',
    name: 'Index Scan',
    description: 'Uses index for selective lookups',
    category: 'execution',
    icon: 'Search',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to scan',
        required: true,
      },
      {
        name: 'index_results',
        type: 'input',
        dataType: 'SingleValue',
        description: 'Index lookup results',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'results',
        type: 'output',
        dataType: 'DataStream',
        description: 'Matching records',
        required: false,
      },
    ],
    parameters: [],
    documentation: {
      summary: 'Uses an index to find matching records',
      details: 'Index scan first looks up matching record IDs in the index, then fetches those records from storage.',
    },
  },
  {
    type: 'filter',
    name: 'Filter',
    description: 'Filters records based on predicate',
    category: 'execution',
    icon: 'Filter',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to filter',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'results',
        type: 'output',
        dataType: 'DataStream',
        description: 'Filtered records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'predicate',
        type: 'string',
        default: 'age > 18',
        description: 'Filter condition',
        uiHint: 'input',
      },
    ],
    documentation: {
      summary: 'Filters records matching a predicate',
      details: 'The filter operator evaluates a predicate on each record and only passes through matching records.',
    },
  },
  {
    type: 'sort',
    name: 'Sort',
    description: 'Sorts records by specified columns',
    category: 'execution',
    icon: 'ArrowUpDown',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Records to sort',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'sorted',
        type: 'output',
        dataType: 'DataStream',
        description: 'Sorted records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'sortColumns',
        type: 'string',
        default: 'id ASC',
        description: 'Columns to sort by',
        uiHint: 'input',
      },
      {
        name: 'memoryLimit',
        type: 'number',
        default: 256,
        description: 'Memory limit for sorting (MB)',
        constraints: { min: 1, max: 4096, step: 1 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Sorts records by one or more columns',
      details: 'If data fits in memory, uses quicksort. Otherwise, uses external merge sort with temporary files.',
    },
  },
  {
    type: 'hash_join',
    name: 'Hash Join',
    description: 'Joins two inputs using hashing',
    category: 'execution',
    icon: 'Merge',
    inputs: [
      {
        name: 'build',
        type: 'input',
        dataType: 'DataStream',
        description: 'Build side input (smaller relation)',
        required: true,
      },
      {
        name: 'probe',
        type: 'input',
        dataType: 'DataStream',
        description: 'Probe side input (larger relation)',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'joined',
        type: 'output',
        dataType: 'DataStream',
        description: 'Joined records',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'joinKey',
        type: 'string',
        default: 'id',
        description: 'Column to join on',
        uiHint: 'input',
      },
      {
        name: 'buildMemory',
        type: 'number',
        default: 256,
        description: 'Memory for hash table (MB)',
        constraints: { min: 1, max: 4096, step: 1 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Joins two inputs using a hash table',
      details: 'Builds a hash table from the smaller input (left), then probes it with records from the larger input (right).',
    },
  },

  // ============== CONCURRENCY BLOCKS ==============
  {
    type: 'row_lock',
    name: 'Row-Level Locking',
    description: 'Fine-grained locking at row level',
    category: 'concurrency',
    icon: 'Lock',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'Transaction',
        description: 'Operations requiring locks',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'committed',
        type: 'output',
        dataType: 'Transaction',
        description: 'Operations with locks acquired',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'lockTimeout',
        type: 'number',
        default: 5000,
        description: 'Lock wait timeout (ms)',
        constraints: { min: 100, max: 60000, step: 100 },
        uiHint: 'slider',
      },
      {
        name: 'deadlockDetection',
        type: 'boolean',
        default: true,
        description: 'Enable deadlock detection',
        uiHint: 'checkbox',
      },
    ],
    documentation: {
      summary: 'Provides row-level locking for concurrent access',
      details: 'Row-level locks allow high concurrency by only blocking access to specific rows being modified.',
    },
  },
  {
    type: 'mvcc',
    name: 'MVCC',
    description: 'Multi-version concurrency control',
    category: 'concurrency',
    icon: 'GitBranch',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'Transaction',
        description: 'Operations to coordinate',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'visible',
        type: 'output',
        dataType: 'Transaction',
        description: 'Version-controlled operations',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'isolationLevel',
        type: 'enum',
        default: 'snapshot',
        description: 'Transaction isolation level',
        constraints: { options: ['read_committed', 'snapshot', 'serializable'] },
        uiHint: 'select',
      },
      {
        name: 'gcInterval',
        type: 'number',
        default: 1000,
        description: 'Version garbage collection interval (ms)',
        constraints: { min: 100, max: 60000, step: 100 },
        uiHint: 'slider',
      },
    ],
    documentation: {
      summary: 'Multi-version concurrency control for snapshot isolation',
      details: 'MVCC maintains multiple versions of records, allowing readers to see a consistent snapshot without blocking writers.',
    },
  },

  // ============== TRANSACTION BLOCKS ==============
  {
    type: 'wal',
    name: 'Write-Ahead Log',
    description: 'Durability through write-ahead logging',
    category: 'transaction',
    icon: 'FileText',
    inputs: [
      {
        name: 'records',
        type: 'input',
        dataType: 'DataStream',
        description: 'Write operations to log',
        required: true,
      },
    ],
    outputs: [
      {
        name: 'logged',
        type: 'output',
        dataType: 'DataStream',
        description: 'Durably logged writes',
        required: false,
      },
    ],
    parameters: [
      {
        name: 'bufferSize',
        type: 'number',
        default: 16,
        description: 'WAL buffer size (MB)',
        constraints: { min: 1, max: 256, step: 1 },
        uiHint: 'slider',
      },
      {
        name: 'syncMode',
        type: 'enum',
        default: 'fsync',
        description: 'Sync mode for durability',
        constraints: { options: ['none', 'fsync', 'fdatasync'] },
        uiHint: 'select',
      },
    ],
    documentation: {
      summary: 'Write-ahead logging for crash recovery',
      details: 'WAL writes all changes to a log before applying them to data files. This ensures durability and enables crash recovery.',
    },
  },
];

/**
 * Get blocks by category
 */
export function getBlocksByCategory(category: BlockCategory): BlockDefinition[] {
  return BLOCK_REGISTRY.filter((block) => block.category === category);
}

/**
 * Get block definition by type
 */
export function getBlockDefinition(type: string): BlockDefinition | undefined {
  return BLOCK_REGISTRY.find((block) => block.type === type);
}

/**
 * Get all categories that have blocks
 */
export function getActiveCategories(): CategoryInfo[] {
  const activeIds = new Set(BLOCK_REGISTRY.map((b) => b.category));
  return CATEGORIES.filter((c) => activeIds.has(c.id));
}

/**
 * Search blocks by name or description
 */
export function searchBlocks(query: string): BlockDefinition[] {
  const lowerQuery = query.toLowerCase();
  return BLOCK_REGISTRY.filter((block) => {
    if (block.name.toLowerCase().includes(lowerQuery)) return true;
    if (block.description.toLowerCase().includes(lowerQuery)) return true;
    const doc = block.documentation;
    if (doc?.overview?.toLowerCase().includes(lowerQuery)) return true;
    if (doc?.algorithm?.toLowerCase().includes(lowerQuery)) return true;
    if (doc?.useCases?.some((uc) => uc.toLowerCase().includes(lowerQuery))) return true;
    if (doc?.tradeoffs?.some((t) => t.toLowerCase().includes(lowerQuery))) return true;
    if (doc?.examples?.some((ex) => ex.toLowerCase().includes(lowerQuery))) return true;
    if (block.references?.some((r) => r.title.toLowerCase().includes(lowerQuery))) return true;
    if (block.references?.some((r) => r.citation?.toLowerCase().includes(lowerQuery))) return true;
    return false;
  });
}
