// ---------------------------------------------------------------------------
// Validation types
// ---------------------------------------------------------------------------

export interface ValidationError {
  nodeId?: string;
  message: string;
  suggestion?: string;
}

export interface ValidationWarning {
  nodeId?: string;
  message: string;
  suggestion?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

// ---------------------------------------------------------------------------
// Execution metrics
// ---------------------------------------------------------------------------

export interface LatencyMetrics {
  avg: number;
  p50: number;
  p95: number;
  p99: number;
}

export interface ExecutionMetrics {
  throughput: number; // ops/sec
  latency: LatencyMetrics;
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
}

export interface BlockMetrics {
  blockId: string;
  blockType: string;
  blockName: string;
  executionTime: number; // ms
  percentage: number; // of total time
  counters: Record<string, number>;
}

export interface ExecutionResult {
  success: boolean;
  duration: number; // ms
  metrics: ExecutionMetrics;
  blockMetrics: BlockMetrics[];
  errors?: string[];
}

// ---------------------------------------------------------------------------
// Progress reporting
// ---------------------------------------------------------------------------

export interface ProgressReport {
  phase: 'validating' | 'executing' | 'aggregating';
  progress: number; // 0–100
  currentBlock: string | null;
  message: string;
}

export type ProgressCallback = (report: ProgressReport) => void;
