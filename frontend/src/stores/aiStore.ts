// ---------------------------------------------------------------------------
// AI Teaching Assistant store (Zustand)
// ---------------------------------------------------------------------------

import { create } from 'zustand';
import type { AIMessage, AISettings } from '@/ai/types';
import { callLLMStreaming } from '@/ai/client';
import { buildSystemPrompt } from '@/ai/context';
import { useCanvasStore } from './canvasStore';
import { useExecutionStore } from './executionStore';
import type { BlockNodeData } from '@/types';
import type { Node } from '@xyflow/react';

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

const SETTINGS_KEY = 'db-sim-ai-settings';

function loadSettings(): AISettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { apiKey: '', endpoint: '', model: '' };
}

function saveSettings(s: AISettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

interface AIState {
  // Settings
  settings: AISettings;
  updateSettings: (s: Partial<AISettings>) => void;

  // Panel visibility
  panelOpen: boolean;
  settingsOpen: boolean;
  togglePanel: () => void;
  openSettings: () => void;
  closeSettings: () => void;

  // Conversation
  messages: AIMessage[];
  streaming: boolean;
  streamingText: string;
  error: string | null;

  // Actions
  sendMessage: (content: string) => void;
  explainResults: () => void;
  explainBottleneck: (blockName: string) => void;
  suggestNext: () => void;
  clearMessages: () => void;
  cancelStream: () => void;
}

let activeController: AbortController | null = null;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useAIStore = create<AIState>((set, get) => ({
  settings: loadSettings(),
  panelOpen: false,
  settingsOpen: false,
  messages: [],
  streaming: false,
  streamingText: '',
  error: null,

  updateSettings: (partial) => {
    const next = { ...get().settings, ...partial };
    set({ settings: next });
    saveSettings(next);
  },

  togglePanel: () => set((s) => ({ panelOpen: !s.panelOpen })),
  openSettings: () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),

  sendMessage: (content: string) => {
    const { settings, messages } = get();
    if (!settings.endpoint || !settings.model) {
      set({ error: 'Configure AI settings first (endpoint and model are required).' });
      return;
    }

    const userMsg: AIMessage = { role: 'user', content };
    const updatedMessages = [...messages, userMsg];
    set({ messages: updatedMessages, streaming: true, streamingText: '', error: null });

    // Build system prompt from current context
    const { nodes, edges } = useCanvasStore.getState();
    const { result, insights } = useExecutionStore.getState();
    const systemPrompt = buildSystemPrompt(
      nodes as Node<BlockNodeData>[],
      edges,
      result,
      insights,
    );

    const fullMessages: AIMessage[] = [
      { role: 'system', content: systemPrompt },
      ...updatedMessages,
    ];

    activeController = callLLMStreaming(fullMessages, settings, {
      onToken: (token) => {
        set((s) => ({ streamingText: s.streamingText + token }));
      },
      onDone: (fullText) => {
        const assistantMsg: AIMessage = { role: 'assistant', content: fullText };
        set((s) => ({
          messages: [...s.messages, assistantMsg],
          streaming: false,
          streamingText: '',
        }));
        activeController = null;
      },
      onError: (error) => {
        set({ streaming: false, streamingText: '', error });
        activeController = null;
      },
    });
  },

  explainResults: () => {
    const { result } = useExecutionStore.getState();
    if (!result) {
      set({ error: 'Run a design first to get results to explain.' });
      return;
    }
    get().sendMessage(
      'Explain what happened in my execution results. ' +
      'Why did I get these numbers? What are the bottlenecks and what do they tell me about database design?'
    );
  },

  explainBottleneck: (blockName: string) => {
    get().sendMessage(
      `Why is "${blockName}" slow? What is happening at the algorithm level, ` +
      `and what would a real database do differently?`
    );
  },

  suggestNext: () => {
    get().sendMessage(
      'Based on my current design and results, what experiment should I try next to learn something new about database internals?'
    );
  },

  clearMessages: () => {
    if (activeController) {
      activeController.abort();
      activeController = null;
    }
    set({ messages: [], streaming: false, streamingText: '', error: null });
  },

  cancelStream: () => {
    if (activeController) {
      activeController.abort();
      activeController = null;
    }
    // Keep whatever text was streamed so far as a message
    const { streamingText, messages } = get();
    if (streamingText) {
      set({
        messages: [...messages, { role: 'assistant', content: streamingText + '...' }],
        streaming: false,
        streamingText: '',
      });
    } else {
      set({ streaming: false, streamingText: '' });
    }
  },
}));
