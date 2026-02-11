import { create } from 'zustand';
import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from './workloadStore';
import type {
  ValidationResult,
  ExecutionResult,
  ProgressReport,
} from '@/engine/types';
import {
  createExecutionEngine,
  type ExecutionEngine,
} from '@/engine/ExecutionEngine';
import { isWASMReady } from '@/wasm/loader';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export type ExecutionStatus =
  | 'idle'
  | 'validating'
  | 'running'
  | 'complete'
  | 'error';

export type EngineType = 'mock' | 'wasm';

interface ExecutionState {
  status: ExecutionStatus;
  progress: number; // 0–100
  currentBlock: string | null;
  progressMessage: string;
  validation: ValidationResult | null;
  result: ExecutionResult | null;
  error: string | null;
  engineType: EngineType;

  // Actions
  startExecution: (
    nodes: Node<BlockNodeData>[],
    edges: Edge[],
    workload: Workload,
  ) => Promise<void>;
  cancelExecution: () => void;
  clearResults: () => void;
  dismissValidation: () => void;
  refreshEngineType: () => void;
}

// ---------------------------------------------------------------------------
// Engine instance
// ---------------------------------------------------------------------------

let engine: ExecutionEngine | null = null;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useExecutionStore = create<ExecutionState>((set) => ({
  status: 'idle',
  progress: 0,
  currentBlock: null,
  progressMessage: '',
  validation: null,
  result: null,
  error: null,
  engineType: isWASMReady() ? 'wasm' : 'mock',

  startExecution: async (nodes, edges, workload) => {
    // Create engine (async — picks WASM if available, else mock)
    engine = await createExecutionEngine();
    const engineType: EngineType = isWASMReady() ? 'wasm' : 'mock';

    // Validation phase
    set({
      status: 'validating',
      progress: 0,
      currentBlock: null,
      progressMessage: 'Validating design...',
      validation: null,
      result: null,
      error: null,
      engineType,
    });

    const validation = engine.validate(nodes, edges);
    set({ validation });

    if (!validation.valid) {
      set({
        status: 'error',
        error: `Design has ${validation.errors.length} error(s).`,
        progressMessage: '',
      });
      return;
    }

    // Execution phase
    set({ status: 'running', progress: 0 });

    const onProgress = (report: ProgressReport) => {
      set({
        progress: report.progress,
        currentBlock: report.currentBlock,
        progressMessage: report.message,
      });
    };

    try {
      const result = await engine.execute(nodes, edges, workload, onProgress);

      if (result.success) {
        set({
          status: 'complete',
          progress: 100,
          currentBlock: null,
          progressMessage: 'Done!',
          result,
        });
      } else {
        set({
          status: 'error',
          progress: 0,
          currentBlock: null,
          progressMessage: '',
          error: result.errors?.join('; ') ?? 'Execution failed.',
        });
      }
    } catch (err) {
      set({
        status: 'error',
        progress: 0,
        currentBlock: null,
        progressMessage: '',
        error: err instanceof Error ? err.message : 'Unknown error.',
      });
    }
  },

  cancelExecution: () => {
    if (engine) engine.cancel();
    set({
      status: 'idle',
      progress: 0,
      currentBlock: null,
      progressMessage: '',
    });
  },

  clearResults: () => {
    set({
      status: 'idle',
      progress: 0,
      currentBlock: null,
      progressMessage: '',
      validation: null,
      result: null,
      error: null,
    });
  },

  dismissValidation: () => {
    set({ validation: null, status: 'idle', error: null });
  },

  refreshEngineType: () => {
    set({ engineType: isWASMReady() ? 'wasm' : 'mock' });
  },
}));
