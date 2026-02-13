/**
 * TypeScript types matching the Rust block-system crate.
 *
 * These types form the contract between the frontend and the WASM module.
 * They mirror the Rust types in block-system/src/core/ and must be kept
 * in sync when the Rust side changes.
 */

// ---------------------------------------------------------------------------
// Port types  (mirrors block-system/src/core/port.rs)
// ---------------------------------------------------------------------------

export type RustPortType =
  | 'DataStream'
  | 'SingleValue'
  | 'Batch'
  | 'Signal'
  | 'Transaction'
  | 'Schema'
  | 'Statistics'
  | 'Config';

export type PortDirection = 'Input' | 'Output';

export interface RustPort {
  id: string;
  name: string;
  port_type: RustPortType;
  direction: PortDirection;
  required: boolean;
  multiple: boolean;
  description: string;
}

export interface PortRef {
  blockId: string;
  portName: string;
}

export interface RustConnection {
  id: string;
  source_block_id: string;
  source_port_id: string;
  target_block_id: string;
  target_port_id: string;
  backpressure: boolean;
  buffer_size: number | null;
}

// Record / PortValue — runtime data flowing through ports
export type SignalValue = 'Start' | 'Stop' | 'Commit' | 'Abort' | { Custom: string };

export type RustRecord = Record<string, unknown>;

export type PortValue =
  | { Stream: RustRecord[] }
  | { Single: RustRecord }
  | { Batch: RustRecord[] }
  | { Signal: SignalValue }
  | 'None';

// ---------------------------------------------------------------------------
// Parameter types  (mirrors block-system/src/core/parameter.rs)
// ---------------------------------------------------------------------------

export type RustParameterType =
  | 'String'
  | 'Number'
  | 'Boolean'
  | 'Enum'
  | 'Object'
  | 'Array';

export type ParameterValue =
  | { String: string }
  | { Number: number }
  | { Integer: number }
  | { Boolean: boolean }
  | { Array: ParameterValue[] }
  | { Object: Record<string, ParameterValue> }
  | 'Null';

export interface ParameterConstraints {
  min?: number;
  max?: number;
  pattern?: string;
  allowed_values?: ParameterValue[];
  min_length?: number;
  max_length?: number;
}

export type WidgetType =
  | 'Input'
  | 'Slider'
  | 'Select'
  | 'Checkbox'
  | 'Textarea'
  | 'JsonEditor';

export interface ParameterUIHint {
  widget: WidgetType;
  step?: number;
  unit?: string;
  help_text?: string;
}

export interface RustParameter {
  id: string;
  name: string;
  param_type: RustParameterType;
  description: string;
  default_value: ParameterValue;
  required: boolean;
  constraints?: ParameterConstraints;
  ui_hint?: ParameterUIHint;
}

// ---------------------------------------------------------------------------
// Metric types  (mirrors block-system/src/core/metrics.rs)
// ---------------------------------------------------------------------------

export type MetricType = 'Counter' | 'Gauge' | 'Histogram' | 'Timing';

export type AggregationType = 'Sum' | 'Avg' | 'Min' | 'Max' | 'P50' | 'P95' | 'P99';

export interface MetricDefinition {
  id: string;
  name: string;
  metric_type: MetricType;
  unit: string;
  description: string;
  aggregations: AggregationType[];
}

export interface MetricSnapshot {
  name: string;
  type: MetricType;
  value: number | number[];
}

// ---------------------------------------------------------------------------
// Block types  (mirrors block-system/src/core/block.rs)
// ---------------------------------------------------------------------------

/**
 * Rust BlockCategory — the Rust crate uses these generic categories.
 * The frontend has its own more DB-specific categories (storage, index, etc.)
 * A mapping layer translates between the two.
 */
export type RustBlockCategory =
  | 'DataSource'
  | 'Transformation'
  | 'Aggregation'
  | 'Output'
  | 'ControlFlow'
  | { Custom: string };

export interface Complexity {
  time: string;   // e.g. "O(log n)"
  space: string;  // e.g. "O(n)"
}

export interface BlockDocumentation {
  overview: string;
  algorithm: string;
  complexity: Complexity;
  use_cases: string[];
  tradeoffs: string[];
  examples: string[];
  motivation: string;
  parameter_guide: Record<string, string>;
  alternatives: WASMAlternative[];
  suggested_questions: string[];
}

export interface WASMAlternative {
  blockType: string;
  comparison: string;
}

export interface RustBlockMetadata {
  id: string;
  name: string;
  category: RustBlockCategory;
  description: string;
  version: string;
  documentation: BlockDocumentation;
  icon: string;
  color: string;
}

// ---------------------------------------------------------------------------
// Reference types  (mirrors block-system/src/core/block.rs Reference)
// ---------------------------------------------------------------------------

export type ReferenceType = 'Paper' | 'Book' | 'Blog' | 'Implementation';

export interface Reference {
  refType: ReferenceType;
  title: string;
  url: string | null;
  citation: string | null;
}

// ---------------------------------------------------------------------------
// Full block detail  (returned by get_block_detail / get_all_block_details)
// ---------------------------------------------------------------------------

export interface WASMBlockDetail {
  blockType: string;
  name: string;
  category: string;
  description: string;
  version: string;
  documentation: BlockDocumentation;
  references: Reference[];
  parameters: WASMParameterDetail[];
  metrics: WASMMetricDetail[];
  inputs: WASMPortDetail[];
  outputs: WASMPortDetail[];
  icon: string;
  color: string;
}

export interface WASMParameterDetail {
  id: string;
  name: string;
  paramType: string;
  description: string;
  defaultValue: unknown;
  required: boolean;
  constraints?: {
    min?: number;
    max?: number;
    pattern?: string;
  };
  uiHint?: {
    widget: string;
    step?: number;
    unit?: string;
    helpText?: string;
  };
}

export interface WASMMetricDetail {
  id: string;
  name: string;
  metricType: string;
  unit: string;
  description: string;
}

export interface WASMPortDetail {
  id: string;
  name: string;
  portType: string;
  direction: string;
  required: boolean;
  description: string;
}

export type BlockError =
  | { InitializationError: string }
  | { ExecutionError: string }
  | { ValidationError: string }
  | { InvalidParameter: string }
  | { InvalidInput: string }
  | { StateError: string }
  | { IoError: string };

export interface RustBlockState {
  data: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Constraint / Guarantee  (mirrors block-system/src/core/constraint.rs)
// ---------------------------------------------------------------------------

export type ConstraintType =
  | { RequiresBlock: string }
  | { RequiresFeature: string }
  | { MinimumMemory: number }
  | { MinimumDisk: number }
  | 'ThreadSafe'
  | 'AtomicOperations';

export interface RustConstraint {
  constraint_type: ConstraintType;
  description: string;
}

export type GuaranteeType =
  | 'Acid'
  | 'Durability'
  | 'Consistency'
  | 'Isolation'
  | 'Atomicity'
  | 'ThreadSafe'
  | 'Serializable'
  | 'SnapshotIsolation';

export type GuaranteeLevel = 'Strict' | 'BestEffort';

export interface RustGuarantee {
  guarantee_type: GuaranteeType;
  description: string;
  level: GuaranteeLevel;
}

// ---------------------------------------------------------------------------
// Execution types  (mirrors block-system/src/core/block.rs)
// ---------------------------------------------------------------------------

export interface RustExecutionResult {
  outputs: Record<string, PortValue>;
  metrics: Record<string, number>;
  errors: BlockError[];
}

export interface RustValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ---------------------------------------------------------------------------
// Bridge-specific types (frontend ↔ WASM boundary)
// ---------------------------------------------------------------------------

/** Sent from frontend to WASM when registering a block. */
export interface BlockConfig {
  type: string;
  id: string;
  parameters: Record<string, string | number | boolean>;
}

/** Sent from frontend to WASM when executing a workload. */
export interface WorkloadConfig {
  operations: OperationConfig[];
  distribution: string;
  concurrency: number;
  totalOps: number;
}

export interface OperationConfig {
  type: string;
  weight: number;
  template: string;
}

/** Progress callback from WASM during execution. */
export interface WASMProgressReport {
  progress: number;      // 0–100
  phase: string;
  currentBlockId: string | null;
  message: string;
}

/** Final metrics bundle from WASM. */
export interface WASMMetricsResult {
  throughput: number;
  latency: {
    avg: number;
    p50: number;
    p95: number;
    p99: number;
  };
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
  blockMetrics: WASMBlockMetrics[];
}

export interface WASMBlockMetrics {
  blockId: string;
  blockType: string;
  blockName: string;
  executionTime: number;
  percentage: number;
  counters: Record<string, number>;
}
