/**
 * Abstract execution engine interface.
 *
 * Both the MockExecutionEngine and the future WASMExecutionEngine implement
 * this interface so the rest of the app can swap between them transparently.
 */

import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from '@/stores/workloadStore';
import type { ValidationResult, ExecutionResult, ProgressCallback } from './types';
import { isWASMReady } from '@/wasm/loader';
import { MockExecutionEngine } from './MockExecutionEngine';

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

export interface ExecutionEngine {
  /** Validate the design graph. */
  validate(nodes: Node<BlockNodeData>[], edges: Edge[]): ValidationResult;

  /** Execute a workload against the design. */
  execute(
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
    workload: Workload,
    onProgress: ProgressCallback,
  ): Promise<ExecutionResult>;

  /** Cancel a running execution. */
  cancel(): void;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create the best available execution engine.
 * Falls back to the mock engine when WASM is not loaded.
 */
export async function createExecutionEngine(): Promise<ExecutionEngine> {
  if (isWASMReady()) {
    // Lazy-import to keep the WASM code out of the critical bundle
    const { WASMExecutionEngine } = await import('./WASMExecutionEngine');
    return new WASMExecutionEngine();
  }

  return new MockExecutionEngine();
}

/**
 * Synchronous factory — always returns mock.
 * Use `createExecutionEngine()` (async) to get WASM when available.
 */
export function createMockEngine(): ExecutionEngine {
  return new MockExecutionEngine();
}
