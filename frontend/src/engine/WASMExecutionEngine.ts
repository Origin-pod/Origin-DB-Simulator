/**
 * WASMExecutionEngine — executes designs using the Rust block-system
 * compiled to WebAssembly via wasm-bindgen.
 *
 * Implements the same ExecutionEngine interface as MockExecutionEngine,
 * so the rest of the app is agnostic to which engine is active.
 */

import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from '@/stores/workloadStore';
import type {
  ValidationResult,
  ExecutionResult,
  ProgressCallback,
  BlockMetrics,
} from './types';
import type { ExecutionEngine } from './ExecutionEngine';
import { getWASMBridge } from '@/wasm/bridge';
import type { BlockConfig, WorkloadConfig } from '@/wasm/types';

// ---------------------------------------------------------------------------
// Frontend-only block types with no Rust/WASM implementation.
// These are silently skipped during WASM registration and execution.
// ---------------------------------------------------------------------------

const FRONTEND_ONLY_BLOCKS = new Set([
  'schema_definition',
  'clock_buffer',
]);

// ---------------------------------------------------------------------------
// WASMExecutionEngine
// ---------------------------------------------------------------------------

export class WASMExecutionEngine implements ExecutionEngine {
  private cancelled = false;

  // -------------------------------------------------------------------
  // Validate
  // -------------------------------------------------------------------

  validate(
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
  ): ValidationResult {
    const bridge = getWASMBridge();

    // Initialize a fresh runtime
    bridge.initRuntime();

    // Register all blocks (skip frontend-only types)
    const skippedNodeIds = new Set<string>();
    for (const node of nodes) {
      const data = node.data as BlockNodeData;
      if (FRONTEND_ONLY_BLOCKS.has(data.blockType)) {
        skippedNodeIds.add(node.id);
        continue;
      }
      const config: BlockConfig = {
        type: data.blockType,
        id: node.id,
        parameters: { ...data.parameters },
      };
      bridge.registerBlock(config);
    }

    // Create connections (skip edges involving frontend-only blocks)
    for (const edge of edges) {
      if (edge.sourceHandle && edge.targetHandle
          && !skippedNodeIds.has(edge.source)
          && !skippedNodeIds.has(edge.target)) {
        bridge.createConnection(
          { blockId: edge.source, portName: edge.sourceHandle },
          { blockId: edge.target, portName: edge.targetHandle },
        );
      }
    }

    // Run validation
    const result = bridge.validate();

    // Map Rust validation result to our frontend type
    return {
      valid: result.valid,
      errors: result.errors.map((msg) => ({ message: msg })),
      warnings: result.warnings.map((msg) => ({ message: msg })),
    };
  }

  // -------------------------------------------------------------------
  // Execute
  // -------------------------------------------------------------------

  async execute(
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
    workload: Workload,
    onProgress: ProgressCallback,
  ): Promise<ExecutionResult> {
    this.cancelled = false;

    // Validate first (also sets up the runtime)
    const validation = this.validate(nodes, edges);
    if (!validation.valid) {
      return {
        success: false,
        duration: 0,
        metrics: {
          throughput: 0,
          latency: { avg: 0, p50: 0, p95: 0, p99: 0 },
          totalOperations: workload.totalOperations,
          successfulOperations: 0,
          failedOperations: 0,
        },
        blockMetrics: [],
        errors: validation.errors.map((e) => e.message),
      };
    }

    const bridge = getWASMBridge();

    // Map workload to WASM config
    const wasmWorkload: WorkloadConfig = {
      operations: workload.operations.map((op) => ({
        type: op.type,
        weight: op.weight,
        template: op.template,
      })),
      distribution: workload.distribution,
      concurrency: workload.concurrency,
      totalOps: workload.totalOperations,
    };

    // Execute with progress reporting
    const rawResult = bridge.execute(wasmWorkload, (report) => {
      if (this.cancelled) return;
      onProgress({
        phase: report.phase === 'aggregating' ? 'aggregating' : 'executing',
        progress: report.progress,
        currentBlock: report.currentBlockId,
        message: report.message,
      });
    });

    if (!rawResult.success) {
      return {
        success: false,
        duration: rawResult.duration,
        metrics: {
          throughput: 0,
          latency: { avg: 0, p50: 0, p95: 0, p99: 0 },
          totalOperations: workload.totalOperations,
          successfulOperations: 0,
          failedOperations: 0,
        },
        blockMetrics: [],
        errors: rawResult.errors ?? ['WASM execution failed.'],
      };
    }

    // Map WASM metrics to frontend types
    const wm = rawResult.metrics;
    const blockMetrics: BlockMetrics[] = wm.blockMetrics.map((bm) => ({
      blockId: bm.blockId,
      blockType: bm.blockType,
      blockName: bm.blockName,
      executionTime: bm.executionTime,
      percentage: bm.percentage,
      counters: bm.counters,
    }));

    return {
      success: true,
      duration: rawResult.duration,
      metrics: {
        throughput: wm.throughput,
        latency: wm.latency,
        totalOperations: wm.totalOperations,
        successfulOperations: wm.successfulOperations,
        failedOperations: wm.failedOperations,
      },
      blockMetrics,
    };
  }

  // -------------------------------------------------------------------
  // Cancel
  // -------------------------------------------------------------------

  cancel(): void {
    this.cancelled = true;
    try {
      getWASMBridge().cancel();
    } catch {
      // Bridge may not be initialized
    }
  }
}
