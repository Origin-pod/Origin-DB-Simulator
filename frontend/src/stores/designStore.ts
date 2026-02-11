import { create } from 'zustand';
import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { ExecutionResult } from '@/engine/types';
import { useCanvasStore } from './canvasStore';
import { useWorkloadStore } from './workloadStore';
import {
  saveState,
  loadState,
  type PersistedDesign,
} from '@/lib/persistence';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Design {
  id: string;
  name: string;
  nodes: Node<BlockNodeData>[];
  edges: Edge[];
  lastResult: ExecutionResult | null;
  createdAt: number;
  updatedAt: number;
}

export type SaveStatus = 'saved' | 'saving' | 'unsaved';

interface DesignState {
  designs: Design[];
  activeDesignId: string;
  saveStatus: SaveStatus;

  // Actions
  createDesign: (name?: string) => string;
  duplicateDesign: (designId: string) => string;
  deleteDesign: (designId: string) => void;
  setActiveDesign: (designId: string) => void;
  renameDesign: (designId: string, name: string) => void;

  // Sync helpers
  saveCurrentCanvas: () => void;
  setDesignResult: (designId: string, result: ExecutionResult) => void;

  // Persistence
  persist: () => void;
  hydrate: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let nextId = 1;
function makeId(): string {
  return `design-${Date.now()}-${nextId++}`;
}

function defaultDesign(name: string): Design {
  return {
    id: makeId(),
    name,
    nodes: [],
    edges: [],
    lastResult: null,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const initialDesign = defaultDesign('Design A');

export const useDesignStore = create<DesignState>((set, get) => ({
  designs: [initialDesign],
  activeDesignId: initialDesign.id,
  saveStatus: 'saved' as SaveStatus,

  createDesign: (name) => {
    // Save current canvas first
    get().saveCurrentCanvas();

    const design = defaultDesign(name ?? `Design ${String.fromCharCode(65 + get().designs.length)}`);
    set({
      designs: [...get().designs, design],
      activeDesignId: design.id,
      saveStatus: 'unsaved',
    });

    // Load empty canvas
    useCanvasStore.getState().loadDesign(design.name, [], []);

    return design.id;
  },

  duplicateDesign: (designId) => {
    // Save current canvas first
    get().saveCurrentCanvas();

    const source = get().designs.find((d) => d.id === designId);
    if (!source) return designId;

    const dup: Design = {
      ...source,
      id: makeId(),
      name: `${source.name} (Copy)`,
      // Deep copy nodes and edges
      nodes: JSON.parse(JSON.stringify(source.nodes)),
      edges: JSON.parse(JSON.stringify(source.edges)),
      lastResult: null,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    set({
      designs: [...get().designs, dup],
      activeDesignId: dup.id,
      saveStatus: 'unsaved',
    });

    useCanvasStore.getState().loadDesign(dup.name, dup.nodes, dup.edges);

    return dup.id;
  },

  deleteDesign: (designId) => {
    const designs = get().designs;
    if (designs.length <= 1) return; // keep at least one

    const remaining = designs.filter((d) => d.id !== designId);
    const wasActive = get().activeDesignId === designId;
    const newActiveId = wasActive ? remaining[0].id : get().activeDesignId;

    set({ designs: remaining, activeDesignId: newActiveId, saveStatus: 'unsaved' });

    if (wasActive) {
      const active = remaining.find((d) => d.id === newActiveId)!;
      useCanvasStore.getState().loadDesign(active.name, active.nodes, active.edges);
    }
  },

  setActiveDesign: (designId) => {
    if (designId === get().activeDesignId) return;

    // Save current canvas state back to the current design
    get().saveCurrentCanvas();

    // Switch
    set({ activeDesignId: designId });

    const design = get().designs.find((d) => d.id === designId);
    if (design) {
      useCanvasStore.getState().loadDesign(design.name, design.nodes, design.edges);
    }
  },

  renameDesign: (designId, name) => {
    set({
      designs: get().designs.map((d) =>
        d.id === designId ? { ...d, name, updatedAt: Date.now() } : d,
      ),
      saveStatus: 'unsaved',
    });
    // If renaming the active design, also update canvasStore
    if (designId === get().activeDesignId) {
      useCanvasStore.getState().setDesignName(name);
    }
  },

  saveCurrentCanvas: () => {
    const canvas = useCanvasStore.getState();
    const activeId = get().activeDesignId;
    set({
      designs: get().designs.map((d) =>
        d.id === activeId
          ? {
              ...d,
              name: canvas.designName,
              nodes: canvas.nodes as Node<BlockNodeData>[],
              edges: canvas.edges,
              updatedAt: Date.now(),
            }
          : d,
      ),
      saveStatus: 'unsaved',
    });
  },

  setDesignResult: (designId, result) => {
    set({
      designs: get().designs.map((d) =>
        d.id === designId ? { ...d, lastResult: result, updatedAt: Date.now() } : d,
      ),
      saveStatus: 'unsaved',
    });
  },

  // -------------------------------------------------------------------
  // Persistence
  // -------------------------------------------------------------------

  persist: () => {
    set({ saveStatus: 'saving' });

    // Sync canvas → active design first
    const canvas = useCanvasStore.getState();
    const activeId = get().activeDesignId;
    const designs = get().designs.map((d) =>
      d.id === activeId
        ? {
            ...d,
            name: canvas.designName,
            nodes: canvas.nodes as Node<BlockNodeData>[],
            edges: canvas.edges,
            updatedAt: Date.now(),
          }
        : d,
    );

    const persisted: PersistedDesign[] = designs.map((d) => ({
      id: d.id,
      name: d.name,
      version: 1,
      nodes: d.nodes,
      edges: d.edges,
      lastResult: d.lastResult,
      createdAt: new Date(d.createdAt).toISOString(),
      updatedAt: new Date(d.updatedAt).toISOString(),
    }));

    const workload = useWorkloadStore.getState().workload;

    saveState({
      version: 1,
      designs: persisted,
      activeDesignId: activeId,
      workload,
    });

    set({ designs, saveStatus: 'saved' });
  },

  hydrate: () => {
    const loaded = loadState();
    if (!loaded) return;

    const designs: Design[] = loaded.designs.map((pd) => ({
      id: pd.id,
      name: pd.name,
      nodes: pd.nodes,
      edges: pd.edges,
      lastResult: pd.lastResult,
      createdAt: new Date(pd.createdAt).getTime(),
      updatedAt: new Date(pd.updatedAt).getTime(),
    }));

    if (designs.length === 0) return;

    const activeId =
      designs.find((d) => d.id === loaded.activeDesignId)?.id ?? designs[0].id;
    const activeDesign = designs.find((d) => d.id === activeId)!;

    set({ designs, activeDesignId: activeId, saveStatus: 'saved' });

    // Load active design into canvas
    useCanvasStore.getState().loadDesign(
      activeDesign.name,
      activeDesign.nodes,
      activeDesign.edges,
    );

    // Load workload
    if (loaded.workload) {
      useWorkloadStore.setState({ workload: loaded.workload });
    }
  },
}));

// ---------------------------------------------------------------------------
// Auto-save: subscribe to store changes and debounce
// ---------------------------------------------------------------------------

let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleAutoSave() {
  if (autoSaveTimer) clearTimeout(autoSaveTimer);
  autoSaveTimer = setTimeout(() => {
    useDesignStore.getState().persist();
  }, 500);
}

// Subscribe to designStore changes
useDesignStore.subscribe((state, prevState) => {
  // Only auto-save when designs or active design changed
  if (state.designs !== prevState.designs || state.activeDesignId !== prevState.activeDesignId) {
    scheduleAutoSave();
  }
});

// Also subscribe to canvasStore changes (nodes/edges mutations)
useCanvasStore.subscribe((state, prevState) => {
  if (state.nodes !== prevState.nodes || state.edges !== prevState.edges) {
    // Mark as unsaved immediately
    useDesignStore.setState({ saveStatus: 'unsaved' });
    scheduleAutoSave();
  }
});

// Also subscribe to workload changes
useWorkloadStore.subscribe((state, prevState) => {
  if (state.workload !== prevState.workload) {
    useDesignStore.setState({ saveStatus: 'unsaved' });
    scheduleAutoSave();
  }
});
