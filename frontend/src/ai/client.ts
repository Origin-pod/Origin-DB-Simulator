// ---------------------------------------------------------------------------
// OpenAI-compatible LLM client with streaming support
// ---------------------------------------------------------------------------

import type { AIMessage, AISettings } from './types';

export interface StreamCallbacks {
  onToken: (token: string) => void;
  onDone: (fullText: string) => void;
  onError: (error: string) => void;
}

/**
 * Non-streaming call to an OpenAI-compatible endpoint.
 */
export async function callLLM(
  messages: AIMessage[],
  settings: AISettings,
): Promise<string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (settings.apiKey) {
    headers['Authorization'] = `Bearer ${settings.apiKey}`;
  }

  const res = await fetch(settings.endpoint, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      model: settings.model,
      messages,
      temperature: 0.7,
      max_tokens: 1024,
    }),
  });

  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(`LLM API error ${res.status}: ${text || res.statusText}`);
  }

  const data = await res.json();
  return data.choices?.[0]?.message?.content ?? '';
}

/**
 * Streaming call to an OpenAI-compatible endpoint.
 * Calls onToken for each chunk, onDone when complete.
 * Returns an AbortController so the caller can cancel.
 */
export function callLLMStreaming(
  messages: AIMessage[],
  settings: AISettings,
  callbacks: StreamCallbacks,
): AbortController {
  const controller = new AbortController();

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (settings.apiKey) {
    headers['Authorization'] = `Bearer ${settings.apiKey}`;
  }

  fetch(settings.endpoint, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      model: settings.model,
      messages,
      temperature: 0.7,
      max_tokens: 1024,
      stream: true,
    }),
    signal: controller.signal,
  })
    .then(async (res) => {
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        callbacks.onError(`LLM API error ${res.status}: ${text || res.statusText}`);
        return;
      }

      const reader = res.body?.getReader();
      if (!reader) {
        callbacks.onError('No response body');
        return;
      }

      const decoder = new TextDecoder();
      let buffer = '';
      let fullText = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() ?? '';

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || !trimmed.startsWith('data: ')) continue;
          const payload = trimmed.slice(6);
          if (payload === '[DONE]') continue;

          try {
            const parsed = JSON.parse(payload);
            const token = parsed.choices?.[0]?.delta?.content;
            if (token) {
              fullText += token;
              callbacks.onToken(token);
            }
          } catch {
            // skip malformed chunks
          }
        }
      }

      callbacks.onDone(fullText);
    })
    .catch((err) => {
      if (err.name === 'AbortError') return;
      callbacks.onError(err.message ?? 'Unknown error');
    });

  return controller;
}

/**
 * Quick connection test — sends a minimal prompt and checks for a response.
 */
export async function testConnection(settings: AISettings): Promise<boolean> {
  const response = await callLLM(
    [{ role: 'user', content: 'Say "ok" and nothing else.' }],
    settings,
  );
  return response.length > 0;
}
