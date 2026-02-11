import { create } from 'zustand';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type OperationType = 'INSERT' | 'SELECT' | 'UPDATE' | 'DELETE' | 'SCAN';
export type Distribution = 'uniform' | 'zipfian' | 'latest';

export interface Operation {
  id: string;
  type: OperationType;
  weight: number; // 0–100, all weights must sum to 100
  template: string;
}

export interface Workload {
  name: string;
  operations: Operation[];
  distribution: Distribution;
  concurrency: number;
  totalOperations: number;
}

// ---------------------------------------------------------------------------
// Default templates per operation type
// ---------------------------------------------------------------------------

const DEFAULT_TEMPLATES: Record<OperationType, string> = {
  INSERT: 'INSERT INTO {table} VALUES (?)',
  SELECT: 'SELECT * FROM {table} WHERE id = ?',
  UPDATE: 'UPDATE {table} SET col = ? WHERE id = ?',
  DELETE: 'DELETE FROM {table} WHERE id = ?',
  SCAN: 'SELECT * FROM {table} WHERE id BETWEEN ? AND ?',
};

// ---------------------------------------------------------------------------
// Preset workloads
// ---------------------------------------------------------------------------

export interface WorkloadPreset {
  id: string;
  name: string;
  description: string;
  workload: Omit<Workload, 'name'> & { name: string };
}

export const WORKLOAD_PRESETS: WorkloadPreset[] = [
  {
    id: 'ycsb-a',
    name: 'YCSB-A',
    description: '50% read, 50% update — Update-heavy',
    workload: {
      name: 'YCSB-A (Update Heavy)',
      operations: [
        { id: 'op-1', type: 'SELECT', weight: 50, template: DEFAULT_TEMPLATES.SELECT },
        { id: 'op-2', type: 'UPDATE', weight: 50, template: DEFAULT_TEMPLATES.UPDATE },
      ],
      distribution: 'zipfian',
      concurrency: 100,
      totalOperations: 10000,
    },
  },
  {
    id: 'ycsb-b',
    name: 'YCSB-B',
    description: '95% read, 5% update — Read-mostly',
    workload: {
      name: 'YCSB-B (Read Mostly)',
      operations: [
        { id: 'op-1', type: 'SELECT', weight: 95, template: DEFAULT_TEMPLATES.SELECT },
        { id: 'op-2', type: 'UPDATE', weight: 5, template: DEFAULT_TEMPLATES.UPDATE },
      ],
      distribution: 'zipfian',
      concurrency: 100,
      totalOperations: 10000,
    },
  },
  {
    id: 'ycsb-c',
    name: 'YCSB-C',
    description: '100% read — Read-only',
    workload: {
      name: 'YCSB-C (Read Only)',
      operations: [
        { id: 'op-1', type: 'SELECT', weight: 100, template: DEFAULT_TEMPLATES.SELECT },
      ],
      distribution: 'zipfian',
      concurrency: 100,
      totalOperations: 10000,
    },
  },
  {
    id: 'tpc-c',
    name: 'TPC-C',
    description: '45% insert, 43% update, 4% select, 4% scan, 4% delete',
    workload: {
      name: 'TPC-C (OLTP)',
      operations: [
        { id: 'op-1', type: 'INSERT', weight: 45, template: DEFAULT_TEMPLATES.INSERT },
        { id: 'op-2', type: 'UPDATE', weight: 43, template: DEFAULT_TEMPLATES.UPDATE },
        { id: 'op-3', type: 'SELECT', weight: 4, template: DEFAULT_TEMPLATES.SELECT },
        { id: 'op-4', type: 'SCAN', weight: 4, template: DEFAULT_TEMPLATES.SCAN },
        { id: 'op-5', type: 'DELETE', weight: 4, template: DEFAULT_TEMPLATES.DELETE },
      ],
      distribution: 'zipfian',
      concurrency: 50,
      totalOperations: 10000,
    },
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let nextId = 1;
function makeId(): string {
  return `op-${Date.now()}-${nextId++}`;
}

/** Default starting workload */
function defaultWorkload(): Workload {
  return {
    name: 'Custom Workload',
    operations: [
      { id: makeId(), type: 'SELECT', weight: 50, template: DEFAULT_TEMPLATES.SELECT },
      { id: makeId(), type: 'INSERT', weight: 50, template: DEFAULT_TEMPLATES.INSERT },
    ],
    distribution: 'zipfian',
    concurrency: 100,
    totalOperations: 10000,
  };
}

/**
 * Redistribute weights so they sum to 100.
 * `changedIdx` is the operation whose weight the user explicitly set.
 * The remaining operations share what's left proportionally.
 */
function redistributeWeights(
  operations: Operation[],
  changedIdx: number,
): Operation[] {
  if (operations.length <= 1) return operations;

  const changedWeight = Math.min(100, Math.max(0, operations[changedIdx].weight));
  const remaining = 100 - changedWeight;
  const othersCount = operations.length - 1;

  // Sum of other weights before redistribution
  const othersSum = operations.reduce(
    (sum, op, i) => (i === changedIdx ? sum : sum + op.weight),
    0,
  );

  return operations.map((op, i) => {
    if (i === changedIdx) return { ...op, weight: changedWeight };
    if (othersSum === 0) {
      // Equal split among others
      return { ...op, weight: Math.round(remaining / othersCount) };
    }
    return { ...op, weight: Math.round((op.weight / othersSum) * remaining) };
  });
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface WorkloadState {
  workload: Workload;
  isEditorOpen: boolean;

  // Modal
  openEditor: () => void;
  closeEditor: () => void;

  // Workload mutations
  setWorkloadName: (name: string) => void;
  setDistribution: (d: Distribution) => void;
  setConcurrency: (n: number) => void;
  setTotalOperations: (n: number) => void;

  // Operation mutations
  addOperation: (type?: OperationType) => void;
  removeOperation: (id: string) => void;
  updateOperationType: (id: string, type: OperationType) => void;
  updateOperationWeight: (id: string, weight: number) => void;
  updateOperationTemplate: (id: string, template: string) => void;

  // Presets
  loadPreset: (presetId: string) => void;
}

export const useWorkloadStore = create<WorkloadState>((set, get) => ({
  workload: defaultWorkload(),
  isEditorOpen: false,

  openEditor: () => set({ isEditorOpen: true }),
  closeEditor: () => set({ isEditorOpen: false }),

  setWorkloadName: (name) =>
    set({ workload: { ...get().workload, name } }),

  setDistribution: (distribution) =>
    set({ workload: { ...get().workload, distribution } }),

  setConcurrency: (concurrency) =>
    set({ workload: { ...get().workload, concurrency: Math.max(1, concurrency) } }),

  setTotalOperations: (totalOperations) =>
    set({ workload: { ...get().workload, totalOperations: Math.max(1, totalOperations) } }),

  addOperation: (type = 'SELECT') => {
    const ops = get().workload.operations;
    const newOp: Operation = {
      id: makeId(),
      type,
      weight: 0,
      template: DEFAULT_TEMPLATES[type],
    };
    // Add with weight 0, then redistribute evenly
    const updated = [...ops, newOp];
    const evenWeight = Math.round(100 / updated.length);
    const balanced = updated.map((op, i) => ({
      ...op,
      weight: i === updated.length - 1 ? 100 - evenWeight * (updated.length - 1) : evenWeight,
    }));
    set({ workload: { ...get().workload, operations: balanced } });
  },

  removeOperation: (id) => {
    const ops = get().workload.operations.filter((op) => op.id !== id);
    if (ops.length === 0) return; // keep at least one
    // Redistribute remaining weights to sum to 100
    const total = ops.reduce((s, op) => s + op.weight, 0);
    const balanced =
      total === 0
        ? ops.map((op, i) => ({
            ...op,
            weight: i === ops.length - 1 ? 100 - Math.round(100 / ops.length) * (ops.length - 1) : Math.round(100 / ops.length),
          }))
        : ops.map((op) => ({ ...op, weight: Math.round((op.weight / total) * 100) }));
    set({ workload: { ...get().workload, operations: balanced } });
  },

  updateOperationType: (id, type) => {
    const ops = get().workload.operations.map((op) =>
      op.id === id ? { ...op, type, template: DEFAULT_TEMPLATES[type] } : op,
    );
    set({ workload: { ...get().workload, operations: ops } });
  },

  updateOperationWeight: (id, weight) => {
    const ops = get().workload.operations;
    const idx = ops.findIndex((op) => op.id === id);
    if (idx === -1) return;
    const updated = ops.map((op, i) => (i === idx ? { ...op, weight } : op));
    set({
      workload: {
        ...get().workload,
        operations: redistributeWeights(updated, idx),
      },
    });
  },

  updateOperationTemplate: (id, template) => {
    const ops = get().workload.operations.map((op) =>
      op.id === id ? { ...op, template } : op,
    );
    set({ workload: { ...get().workload, operations: ops } });
  },

  loadPreset: (presetId) => {
    const preset = WORKLOAD_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    // Deep-copy operations to avoid shared references
    const workload: Workload = {
      ...preset.workload,
      operations: preset.workload.operations.map((op) => ({
        ...op,
        id: makeId(),
      })),
    };
    set({ workload });
  },
}));
