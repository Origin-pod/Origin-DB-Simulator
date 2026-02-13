import { useRef, useEffect } from 'react';
import {
  X,
  Settings,
  Sparkles,
  Zap,
  Compass,
  Loader2,
  Trash2,
  StopCircle,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useAIStore } from '@/stores/aiStore';
import { useExecutionStore } from '@/stores/executionStore';

export function AITeachingPanel() {
  const {
    panelOpen,
    togglePanel,
    openSettings,
    messages,
    streaming,
    streamingText,
    error,
    sendMessage,
    explainResults,
    suggestNext,
    clearMessages,
    cancelStream,
    settings,
  } = useAIStore();

  const { result } = useExecutionStore();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingText]);

  if (!panelOpen) return null;

  const isConfigured = settings.endpoint && settings.model;

  // Find bottleneck block for contextual button
  const bottleneck = result?.blockMetrics
    ? [...result.blockMetrics].sort((a, b) => b.percentage - a.percentage)[0]
    : null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const val = inputRef.current?.value.trim();
    if (!val || streaming) return;
    sendMessage(val);
    if (inputRef.current) inputRef.current.value = '';
  };

  return (
    <div className="fixed right-0 top-0 bottom-0 w-96 bg-white border-l border-gray-200 shadow-xl z-40 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 bg-gradient-to-r from-blue-50 to-purple-50">
        <div className="flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-blue-600" />
          <span className="text-sm font-semibold text-gray-900">
            AI Teaching Assistant
          </span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={openSettings}
            className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-white/60"
            title="Settings"
          >
            <Settings className="w-4 h-4" />
          </button>
          <button
            onClick={clearMessages}
            className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-white/60"
            title="Clear conversation"
            disabled={messages.length === 0}
          >
            <Trash2 className="w-4 h-4" />
          </button>
          <button
            onClick={togglePanel}
            className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-white/60"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Not configured notice */}
      {!isConfigured && (
        <div className="px-4 py-3 bg-amber-50 border-b border-amber-100">
          <p className="text-xs text-amber-800">
            Configure an LLM endpoint to use the AI assistant.
          </p>
          <button
            onClick={openSettings}
            className="text-xs text-blue-600 hover:underline mt-1"
          >
            Open settings
          </button>
        </div>
      )}

      {/* Action buttons */}
      {isConfigured && (
        <div className="px-4 py-3 border-b border-gray-100 space-y-2">
          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={explainResults}
              disabled={streaming || !result}
              className="flex-1 text-xs"
            >
              <Zap className="w-3.5 h-3.5" />
              Explain Results
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={suggestNext}
              disabled={streaming}
              className="flex-1 text-xs"
            >
              <Compass className="w-3.5 h-3.5" />
              What Next?
            </Button>
          </div>
          {bottleneck && bottleneck.percentage > 30 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                useAIStore.getState().explainBottleneck(bottleneck.blockName)
              }
              disabled={streaming}
              className="w-full text-xs text-amber-700 bg-amber-50 hover:bg-amber-100"
            >
              <Zap className="w-3.5 h-3.5 text-amber-500" />
              Why is "{bottleneck.blockName}" slow? ({bottleneck.percentage.toFixed(0)}%)
            </Button>
          )}
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
        {messages.length === 0 && !streaming && (
          <div className="text-center py-12">
            <Sparkles className="w-8 h-8 text-gray-300 mx-auto mb-3" />
            <p className="text-sm text-gray-500">
              Ask questions about your database design
            </p>
            <p className="text-xs text-gray-400 mt-1">
              Run a design first, then click "Explain Results"
            </p>
          </div>
        )}

        {messages.map((msg, i) => (
          <MessageBubble key={i} role={msg.role} content={msg.content} />
        ))}

        {/* Streaming response */}
        {streaming && streamingText && (
          <MessageBubble role="assistant" content={streamingText} streaming />
        )}

        {/* Loading indicator */}
        {streaming && !streamingText && (
          <div className="flex items-center gap-2 text-xs text-gray-400">
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
            Thinking...
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="px-3 py-2 rounded-lg bg-red-50 border border-red-200 text-xs text-red-700">
            {error}
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      {isConfigured && (
        <div className="px-4 py-3 border-t border-gray-200 bg-gray-50">
          {streaming ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={cancelStream}
              className="w-full text-xs"
            >
              <StopCircle className="w-3.5 h-3.5" />
              Stop generating
            </Button>
          ) : (
            <form onSubmit={handleSubmit} className="flex gap-2">
              <input
                ref={inputRef}
                type="text"
                placeholder="Ask about your design..."
                className="flex-1 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <Button variant="primary" size="sm" type="submit">
                Send
              </Button>
            </form>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Message bubble
// ---------------------------------------------------------------------------

function MessageBubble({
  role,
  content,
  streaming,
}: {
  role: string;
  content: string;
  streaming?: boolean;
}) {
  if (role === 'system') return null;

  const isUser = role === 'user';

  return (
    <div
      className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}
    >
      <div
        className={`max-w-[85%] px-3 py-2 rounded-lg text-sm leading-relaxed ${
          isUser
            ? 'bg-blue-600 text-white rounded-br-sm'
            : 'bg-gray-100 text-gray-800 rounded-bl-sm'
        }`}
      >
        {isUser ? (
          <p>{content}</p>
        ) : (
          <div className="prose-sm">
            <MarkdownLite text={content} />
            {streaming && (
              <span className="inline-block w-1.5 h-4 bg-gray-400 animate-pulse ml-0.5 align-text-bottom" />
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Minimal markdown renderer (bold, code, paragraphs)
// ---------------------------------------------------------------------------

function MarkdownLite({ text }: { text: string }) {
  const paragraphs = text.split(/\n\n+/);

  return (
    <>
      {paragraphs.map((para, i) => {
        // Code block
        if (para.startsWith('```')) {
          const code = para.replace(/^```\w*\n?/, '').replace(/```$/, '');
          return (
            <pre
              key={i}
              className="bg-gray-200 text-gray-800 rounded px-2 py-1.5 text-xs font-mono overflow-x-auto my-1.5"
            >
              {code}
            </pre>
          );
        }

        // Regular paragraph with inline formatting
        return (
          <p key={i} className={i > 0 ? 'mt-2' : ''}>
            {formatInline(para)}
          </p>
        );
      })}
    </>
  );
}

function formatInline(text: string): React.ReactNode[] {
  // Split on bold (**text**) and inline code (`text`)
  const parts: React.ReactNode[] = [];
  const regex = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let lastIdx = 0;

  text.replace(regex, (match, _p1, offset) => {
    // Text before match
    if (offset > lastIdx) {
      parts.push(text.slice(lastIdx, offset));
    }

    if (match.startsWith('**')) {
      parts.push(
        <strong key={offset} className="font-semibold">
          {match.slice(2, -2)}
        </strong>,
      );
    } else if (match.startsWith('`')) {
      parts.push(
        <code
          key={offset}
          className="bg-gray-200 text-gray-800 px-1 py-0.5 rounded text-[11px] font-mono"
        >
          {match.slice(1, -1)}
        </code>,
      );
    }

    lastIdx = offset + match.length;
    return match;
  });

  // Remaining text
  if (lastIdx < text.length) {
    parts.push(text.slice(lastIdx));
  }

  return parts;
}
