// ---------------------------------------------------------------------------
// Challenge store (Zustand) — manages challenge state and persistence
// ---------------------------------------------------------------------------

import { create } from 'zustand';
import type { Challenge, ChallengeStep } from '@/data/challenges';
import { getChallenges } from '@/data/challenges';
import { useCanvasStore } from './canvasStore';
import { useWorkloadStore } from './workloadStore';
import { useExecutionStore } from './executionStore';
import type { BlockNodeData } from '@/types';
import type { Node } from '@xyflow/react';

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

const STORAGE_KEY = 'db-sim-challenges';

interface PersistedData {
  completedChallenges: string[];
  completedSteps: Record<string, number[]>; // challengeId -> step indexes
}

function loadPersisted(): PersistedData {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { completedChallenges: [], completedSteps: {} };
}

function savePersisted(data: PersistedData) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

interface ChallengeState {
  // Browsing
  modalOpen: boolean;
  openModal: () => void;
  closeModal: () => void;

  // Active challenge
  activeChallenge: Challenge | null;
  currentStepIndex: number;
  stepCompleted: boolean;
  showPayoff: boolean;

  // Tracking
  completedChallenges: string[];
  completedSteps: Record<string, number[]>;

  // Actions
  startChallenge: (challengeId: string) => void;
  verifyStep: () => boolean;
  nextStep: () => void;
  exitChallenge: () => void;
  dismissPayoff: () => void;
}

export const useChallengeStore = create<ChallengeState>((set, get) => {
  const persisted = loadPersisted();

  return {
    modalOpen: false,
    activeChallenge: null,
    currentStepIndex: 0,
    stepCompleted: false,
    showPayoff: false,
    completedChallenges: persisted.completedChallenges,
    completedSteps: persisted.completedSteps,

    openModal: () => set({ modalOpen: true }),
    closeModal: () => set({ modalOpen: false }),

    startChallenge: (challengeId: string) => {
      const challenges = getChallenges();
      const challenge = challenges.find((c) => c.id === challengeId);
      if (!challenge) return;

      set({
        activeChallenge: challenge,
        currentStepIndex: 0,
        stepCompleted: false,
        showPayoff: false,
        modalOpen: false,
      });

      // Load the first step's scaffold if it exists
      const step = challenge.steps[0];
      if (step.scaffold) {
        loadScaffold(step);
      }
    },

    verifyStep: (): boolean => {
      const { activeChallenge, currentStepIndex } = get();
      if (!activeChallenge) return false;

      const step = activeChallenge.steps[currentStepIndex];
      if (!step) return false;

      const criteria = step.successCriteria;

      // No criteria = auto-pass (reflection steps)
      if (!criteria) {
        markStepComplete(set, get);
        return true;
      }

      switch (criteria.type) {
        case 'run_execution': {
          const { status, result } = useExecutionStore.getState();
          if (status === 'complete' && result?.success) {
            markStepComplete(set, get);
            return true;
          }
          return false;
        }
        case 'check_throughput': {
          const { result } = useExecutionStore.getState();
          if (result?.success && result.metrics.throughput >= (criteria.threshold ?? 0)) {
            markStepComplete(set, get);
            return true;
          }
          return false;
        }
        case 'has_block': {
          const { nodes } = useCanvasStore.getState();
          const hasIt = nodes.some(
            (n) => (n.data as BlockNodeData).blockType === criteria.blockType,
          );
          if (hasIt) {
            markStepComplete(set, get);
            return true;
          }
          return false;
        }
        default:
          markStepComplete(set, get);
          return true;
      }
    },

    nextStep: () => {
      const { activeChallenge, currentStepIndex } = get();
      if (!activeChallenge) return;

      const nextIdx = currentStepIndex + 1;
      if (nextIdx >= activeChallenge.steps.length) {
        // Challenge complete!
        const completedChallenges = [...new Set([...get().completedChallenges, activeChallenge.id])];
        const completedSteps = { ...get().completedSteps };

        set({
          completedChallenges,
          stepCompleted: false,
          showPayoff: false,
        });
        savePersisted({ completedChallenges, completedSteps });
        return;
      }

      set({
        currentStepIndex: nextIdx,
        stepCompleted: false,
        showPayoff: false,
      });

      // Load scaffold for next step
      const step = activeChallenge.steps[nextIdx];
      if (step.scaffold) {
        loadScaffold(step);
      }
    },

    exitChallenge: () => {
      set({
        activeChallenge: null,
        currentStepIndex: 0,
        stepCompleted: false,
        showPayoff: false,
      });
    },

    dismissPayoff: () => {
      set({ showPayoff: false });
    },
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function markStepComplete(
  set: (partial: Partial<ChallengeState>) => void,
  get: () => ChallengeState,
) {
  const { activeChallenge, currentStepIndex, completedSteps } = get();
  if (!activeChallenge) return;

  const updated = { ...completedSteps };
  const steps = updated[activeChallenge.id] ?? [];
  if (!steps.includes(currentStepIndex)) {
    updated[activeChallenge.id] = [...steps, currentStepIndex];
  }

  set({
    stepCompleted: true,
    showPayoff: true,
    completedSteps: updated,
  });
  savePersisted({
    completedChallenges: get().completedChallenges,
    completedSteps: updated,
  });
}

function loadScaffold(step: ChallengeStep) {
  if (!step.scaffold) return;

  const canvas = useCanvasStore.getState();
  canvas.loadDesign(
    'Challenge',
    step.scaffold.nodes as Node<BlockNodeData>[],
    step.scaffold.edges,
  );

  useWorkloadStore.setState({ workload: step.scaffold.workload });
  useExecutionStore.getState().clearResults();
}
