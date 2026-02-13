// ---------------------------------------------------------------------------
// Wiki store (Zustand) — manages Node Wiki panel state + navigation
// ---------------------------------------------------------------------------

import { create } from 'zustand';

interface WikiState {
  isOpen: boolean;
  blockType: string | null;
  history: string[]; // breadcrumb trail for back navigation

  open: (blockType: string) => void;
  close: () => void;
  navigateTo: (blockType: string) => void; // push current to history, switch
  back: () => void; // pop history
}

export const useWikiStore = create<WikiState>((set, get) => ({
  isOpen: false,
  blockType: null,
  history: [],

  open: (blockType: string) => {
    set({ isOpen: true, blockType, history: [] });
  },

  close: () => {
    set({ isOpen: false, blockType: null, history: [] });
  },

  navigateTo: (blockType: string) => {
    const { blockType: current, history } = get();
    if (current) {
      set({ blockType, history: [...history, current] });
    } else {
      set({ blockType, isOpen: true, history: [] });
    }
  },

  back: () => {
    const { history } = get();
    if (history.length === 0) {
      set({ isOpen: false, blockType: null });
      return;
    }
    const newHistory = [...history];
    const previous = newHistory.pop()!;
    set({ blockType: previous, history: newHistory });
  },
}));
