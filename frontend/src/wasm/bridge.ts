/**
 * WASM Bridge — high-level TypeScript API over the raw WASM module.
 *
 * Handles JSON serialization / deserialization and provides typed methods.
 * This is the single entry point that the WASMExecutionEngine uses.
 */

import { getWASMModule } from './loader';
import type {
  BlockConfig,
  PortRef,
  WorkloadConfig,
  WASMProgressReport,
  WASMMetricsResult,
  RustConnection,
} from './types';

// ---------------------------------------------------------------------------
// Validation result from Rust (matches RustValidationResult in types.ts)
// ---------------------------------------------------------------------------

interface BridgeValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ---------------------------------------------------------------------------
// Execution result from Rust (raw JSON)
// ---------------------------------------------------------------------------

interface BridgeExecutionResult {
  success: boolean;
  duration: number;
  metrics: WASMMetricsResult;
  errors?: string[];
}

// ---------------------------------------------------------------------------
// Bridge class
// ---------------------------------------------------------------------------

export class WASMBridge {
  // -------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------

  isReady(): boolean {
    return getWASMModule() !== null;
  }

  initRuntime(): void {
    const wasm = this.getModule();
    wasm.init_runtime();
  }

  destroyRuntime(): void {
    const wasm = this.getModule();
    wasm.destroy_runtime();
  }

  // -------------------------------------------------------------------
  // Block management
  // -------------------------------------------------------------------

  registerBlock(config: BlockConfig): string {
    const wasm = this.getModule();
    const resultJson = wasm.register_block(JSON.stringify(config));
    const result = JSON.parse(resultJson);
    if (result.error) {
      throw new Error(`Failed to register block "${config.id}": ${result.error}`);
    }
    return result.id ?? config.id;
  }

  createConnection(source: PortRef, target: PortRef): string {
    const wasm = this.getModule();
    const connection: Omit<RustConnection, 'id'> = {
      source_block_id: source.blockId,
      source_port_id: source.portName,
      target_block_id: target.blockId,
      target_port_id: target.portName,
      backpressure: false,
      buffer_size: null,
    };
    const resultJson = wasm.create_connection(JSON.stringify(connection));
    const result = JSON.parse(resultJson);
    if (result.error) {
      throw new Error(
        `Failed to connect ${source.blockId}:${source.portName} → ${target.blockId}:${target.portName}: ${result.error}`,
      );
    }
    return result.id ?? '';
  }

  // -------------------------------------------------------------------
  // Validation
  // -------------------------------------------------------------------

  validate(): BridgeValidationResult {
    const wasm = this.getModule();
    const json = wasm.validate();
    return JSON.parse(json) as BridgeValidationResult;
  }

  // -------------------------------------------------------------------
  // Execution
  // -------------------------------------------------------------------

  execute(
    workload: WorkloadConfig,
    onProgress: (report: WASMProgressReport) => void,
  ): BridgeExecutionResult {
    const wasm = this.getModule();

    // The WASM module calls progressCallback with JSON strings
    const progressCallback = (json: string) => {
      try {
        const report: WASMProgressReport = JSON.parse(json);
        onProgress(report);
      } catch {
        // Ignore malformed progress reports
      }
    };

    const resultJson = wasm.execute(JSON.stringify(workload), progressCallback);
    return JSON.parse(resultJson) as BridgeExecutionResult;
  }

  cancel(): void {
    const wasm = this.getModule();
    wasm.cancel_execution();
  }

  // -------------------------------------------------------------------
  // Metrics
  // -------------------------------------------------------------------

  getMetrics(): WASMMetricsResult {
    const wasm = this.getModule();
    const json = wasm.get_metrics();
    return JSON.parse(json) as WASMMetricsResult;
  }

  // -------------------------------------------------------------------
  // Discovery
  // -------------------------------------------------------------------

  getBlockTypes(): unknown[] {
    const wasm = this.getModule();
    const json = wasm.get_block_types();
    return JSON.parse(json);
  }

  // -------------------------------------------------------------------
  // Internal
  // -------------------------------------------------------------------

  private getModule() {
    const wasm = getWASMModule();
    if (!wasm) {
      throw new Error('WASM module is not loaded. Call loadWASM() first.');
    }
    return wasm;
  }
}

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

let bridgeInstance: WASMBridge | null = null;

export function getWASMBridge(): WASMBridge {
  if (!bridgeInstance) {
    bridgeInstance = new WASMBridge();
  }
  return bridgeInstance;
}
