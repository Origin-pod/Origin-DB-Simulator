// ---------------------------------------------------------------------------
// AI Teaching Assistant types
// ---------------------------------------------------------------------------

export interface AIMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface AISettings {
  apiKey: string;
  endpoint: string;
  model: string;
}

export interface AIPreset {
  name: string;
  endpoint: string;
  model: string;
  requiresKey: boolean;
}

export const AI_PRESETS: AIPreset[] = [
  {
    name: 'OpenAI',
    endpoint: 'https://api.openai.com/v1/chat/completions',
    model: 'gpt-4o-mini',
    requiresKey: true,
  },
  {
    name: 'Groq',
    endpoint: 'https://api.groq.com/openai/v1/chat/completions',
    model: 'llama-3.3-70b-versatile',
    requiresKey: true,
  },
  {
    name: 'Ollama (local)',
    endpoint: 'http://localhost:11434/v1/chat/completions',
    model: 'llama3.2',
    requiresKey: false,
  },
];
