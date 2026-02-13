// ---------------------------------------------------------------------------
// Architecture store (Zustand) — manages DB architecture browsing state
// ---------------------------------------------------------------------------

import { create } from 'zustand';
import type { DBArchitecture } from '@/data/architectures';
import { getArchitectureById } from '@/data/architectures';
import { useCanvasStore } from './canvasStore';
import { useWorkloadStore } from './workloadStore';
import { useExecutionStore } from './executionStore';
import { useDesignStore } from './designStore';
import type { BlockNodeData } from '@/types';
import type { Node } from '@xyflow/react';

interface ArchitectureState {
  modalOpen: boolean;
  activeArchitecture: DBArchitecture | null;

  openModal: () => void;
  closeModal: () => void;
  loadArchitecture: (id: string) => void;
  clearArchitecture: () => void;
}

export const useArchitectureStore = create<ArchitectureState>((set) => ({
  modalOpen: false,
  activeArchitecture: null,

  openModal: () => set({ modalOpen: true }),
  closeModal: () => set({ modalOpen: false }),

  loadArchitecture: (id: string) => {
    const arch = getArchitectureById(id);
    if (!arch) return;

    // Save current canvas first
    useDesignStore.getState().saveCurrentCanvas();

    // Create a new design for this architecture
    const newId = useDesignStore.getState().createDesign(`Study: ${arch.name}`);
    void newId;

    // Load nodes/edges/workload
    const canvas = useCanvasStore.getState();
    canvas.loadDesign(
      `Study: ${arch.name}`,
      arch.nodes as Node<BlockNodeData>[],
      arch.edges,
    );
    useWorkloadStore.setState({ workload: arch.workload });
    useExecutionStore.getState().clearResults();

    set({
      activeArchitecture: arch,
      modalOpen: false,
    });
  },

  clearArchitecture: () => {
    set({ activeArchitecture: null });
  },
}));
