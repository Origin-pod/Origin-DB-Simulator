import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { Workload } from '@/stores/workloadStore';
import type { ExecutionResult } from '@/engine/types';

// ---------------------------------------------------------------------------
// Schema version (bump when persisted shape changes)
// ---------------------------------------------------------------------------

const SCHEMA_VERSION = 1;

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

const KEYS = {
  designs: 'db-sim-designs',
  workload: 'db-sim-workload',
  activeDesignId: 'db-sim-active-design',
  schemaVersion: 'db-sim-schema-version',
} as const;

// ---------------------------------------------------------------------------
// Persisted types
// ---------------------------------------------------------------------------

export interface PersistedDesign {
  id: string;
  name: string;
  version: number;
  nodes: Node<BlockNodeData>[];
  edges: Edge[];
  workload?: Workload;
  lastResult: ExecutionResult | null;
  createdAt: string; // ISO
  updatedAt: string; // ISO
}

interface PersistedState {
  version: number;
  designs: PersistedDesign[];
  activeDesignId: string;
  workload: Workload;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function safeGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeSet(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // storage full or unavailable — silently ignore
  }
}

// ---------------------------------------------------------------------------
// Save
// ---------------------------------------------------------------------------

export function saveState(state: PersistedState): void {
  safeSet(KEYS.schemaVersion, String(SCHEMA_VERSION));
  safeSet(KEYS.designs, JSON.stringify(state.designs));
  safeSet(KEYS.activeDesignId, state.activeDesignId);
  safeSet(KEYS.workload, JSON.stringify(state.workload));
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

export function loadState(): PersistedState | null {
  const raw = safeGet(KEYS.designs);
  if (!raw) return null;

  try {
    const storedVersion = Number(safeGet(KEYS.schemaVersion) ?? '0');
    if (storedVersion > SCHEMA_VERSION) {
      // Future version — don't try to parse
      return null;
    }

    const designs: PersistedDesign[] = JSON.parse(raw);
    if (!Array.isArray(designs) || designs.length === 0) return null;

    const activeDesignId = safeGet(KEYS.activeDesignId) ?? designs[0].id;

    let workload: Workload | null = null;
    const wlRaw = safeGet(KEYS.workload);
    if (wlRaw) {
      workload = JSON.parse(wlRaw);
    }

    return {
      version: storedVersion,
      designs,
      activeDesignId,
      workload: workload ?? {
        name: 'Custom Workload',
        operations: [
          { id: 'op-1', type: 'SELECT', weight: 50, template: 'SELECT * FROM {table} WHERE id = ?' },
          { id: 'op-2', type: 'INSERT', weight: 50, template: 'INSERT INTO {table} VALUES (?)' },
        ],
        distribution: 'zipfian',
        concurrency: 100,
        totalOperations: 10000,
      },
    };
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Clear
// ---------------------------------------------------------------------------

export function clearPersistedState(): void {
  try {
    localStorage.removeItem(KEYS.designs);
    localStorage.removeItem(KEYS.workload);
    localStorage.removeItem(KEYS.activeDesignId);
    localStorage.removeItem(KEYS.schemaVersion);
  } catch {
    // ignore
  }
}

// ---------------------------------------------------------------------------
// Design export/import helpers
// ---------------------------------------------------------------------------

export interface ExportedDesign {
  version: string;
  exportedAt: string;
  design: {
    name: string;
    nodes: Node<BlockNodeData>[];
    edges: Edge[];
    workload: Workload;
    lastResult: ExecutionResult | null;
  };
}

export function exportDesign(
  name: string,
  nodes: Node<BlockNodeData>[],
  edges: Edge[],
  workload: Workload,
  lastResult: ExecutionResult | null,
): ExportedDesign {
  return {
    version: '1.0',
    exportedAt: new Date().toISOString(),
    design: {
      name,
      nodes: JSON.parse(JSON.stringify(nodes)),
      edges: JSON.parse(JSON.stringify(edges)),
      workload: JSON.parse(JSON.stringify(workload)),
      lastResult: lastResult ? JSON.parse(JSON.stringify(lastResult)) : null,
    },
  };
}

export function validateImport(data: unknown): { valid: boolean; error?: string; design?: ExportedDesign } {
  if (!data || typeof data !== 'object') {
    return { valid: false, error: 'Invalid file: not a JSON object.' };
  }

  const obj = data as Record<string, unknown>;

  if (typeof obj.version !== 'string') {
    return { valid: false, error: 'Invalid file: missing version field.' };
  }

  // Basic version check
  const major = parseInt(obj.version as string, 10);
  if (isNaN(major) || major > 1) {
    return { valid: false, error: `Incompatible version: ${obj.version}. This app supports version 1.x.` };
  }

  if (!obj.design || typeof obj.design !== 'object') {
    return { valid: false, error: 'Invalid file: missing design data.' };
  }

  const design = obj.design as Record<string, unknown>;

  if (!Array.isArray(design.nodes)) {
    return { valid: false, error: 'Invalid file: design.nodes must be an array.' };
  }

  if (!Array.isArray(design.edges)) {
    return { valid: false, error: 'Invalid file: design.edges must be an array.' };
  }

  return { valid: true, design: data as ExportedDesign };
}

// ---------------------------------------------------------------------------
// File download helper
// ---------------------------------------------------------------------------

export function downloadFile(filename: string, content: string, mimeType = 'application/json'): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
