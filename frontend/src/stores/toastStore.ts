import { create } from 'zustand';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration: number; // ms, 0 = sticky
  action?: { label: string; onClick: () => void };
}

interface ToastState {
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => string;
  removeToast: (id: string) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let seq = 0;

const DEFAULT_DURATION: Record<ToastType, number> = {
  success: 3000,
  error: 5000,
  warning: 4000,
  info: 3000,
};

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],

  addToast: (partial) => {
    const id = `toast-${++seq}`;
    const duration = partial.duration ?? DEFAULT_DURATION[partial.type];
    const toast: Toast = { ...partial, id, duration };

    set({ toasts: [...get().toasts, toast] });

    if (duration > 0) {
      setTimeout(() => {
        get().removeToast(id);
      }, duration);
    }

    return id;
  },

  removeToast: (id) => {
    set({ toasts: get().toasts.filter((t) => t.id !== id) });
  },
}));

// ---------------------------------------------------------------------------
// Convenience helpers (callable from anywhere, no hook required)
// ---------------------------------------------------------------------------

export const toast = {
  success: (message: string, action?: Toast['action']) =>
    useToastStore.getState().addToast({ type: 'success', message, duration: 3000, action }),
  error: (message: string, action?: Toast['action']) =>
    useToastStore.getState().addToast({ type: 'error', message, duration: 5000, action }),
  warning: (message: string, action?: Toast['action']) =>
    useToastStore.getState().addToast({ type: 'warning', message, duration: 4000, action }),
  info: (message: string, action?: Toast['action']) =>
    useToastStore.getState().addToast({ type: 'info', message, duration: 3000, action }),
};
