import { useState } from 'react';
import { X, Check, Loader2, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useAIStore } from '@/stores/aiStore';
import { AI_PRESETS } from '@/ai/types';
import { testConnection } from '@/ai/client';

export function AISettingsModal() {
  const { settings, updateSettings, settingsOpen, closeSettings } = useAIStore();
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testError, setTestError] = useState('');

  if (!settingsOpen) return null;

  const handlePreset = (presetName: string) => {
    const preset = AI_PRESETS.find((p) => p.name === presetName);
    if (preset) {
      updateSettings({
        endpoint: preset.endpoint,
        model: preset.model,
      });
      setTestResult(null);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    setTestError('');
    try {
      const ok = await testConnection(settings);
      setTestResult(ok ? 'success' : 'error');
      if (!ok) setTestError('No response from API');
    } catch (err) {
      setTestResult('error');
      setTestError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      setTesting(false);
    }
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/30 z-50"
        onClick={closeSettings}
      />

      {/* Modal */}
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="bg-white rounded-xl shadow-xl w-full max-w-md"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200">
            <h2 className="text-base font-semibold text-gray-900">
              AI Teaching Assistant Settings
            </h2>
            <button
              onClick={closeSettings}
              className="text-gray-400 hover:text-gray-600"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Body */}
          <div className="px-5 py-4 space-y-4">
            {/* Presets */}
            <div>
              <label className="text-xs font-medium text-gray-700 block mb-1.5">
                Quick Setup
              </label>
              <div className="flex gap-2">
                {AI_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    onClick={() => handlePreset(preset.name)}
                    className={`px-3 py-1.5 text-xs rounded-lg border transition-colors ${
                      settings.endpoint === preset.endpoint
                        ? 'border-blue-500 bg-blue-50 text-blue-700'
                        : 'border-gray-200 text-gray-600 hover:bg-gray-50'
                    }`}
                  >
                    {preset.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Endpoint */}
            <div>
              <label className="text-xs font-medium text-gray-700 block mb-1.5">
                API Endpoint
              </label>
              <input
                type="url"
                value={settings.endpoint}
                onChange={(e) => {
                  updateSettings({ endpoint: e.target.value });
                  setTestResult(null);
                }}
                placeholder="https://api.openai.com/v1/chat/completions"
                className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>

            {/* Model */}
            <div>
              <label className="text-xs font-medium text-gray-700 block mb-1.5">
                Model
              </label>
              <input
                type="text"
                value={settings.model}
                onChange={(e) => {
                  updateSettings({ model: e.target.value });
                  setTestResult(null);
                }}
                placeholder="gpt-4o-mini"
                className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>

            {/* API Key */}
            <div>
              <label className="text-xs font-medium text-gray-700 block mb-1.5">
                API Key
              </label>
              <input
                type="password"
                value={settings.apiKey}
                onChange={(e) => {
                  updateSettings({ apiKey: e.target.value });
                  setTestResult(null);
                }}
                placeholder="sk-... (leave empty for Ollama)"
                className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <p className="text-[10px] text-gray-400 mt-1">
                Stored in your browser's localStorage. Never sent to our servers.
              </p>
            </div>

            {/* Test Connection */}
            <div className="flex items-center gap-3">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleTest}
                disabled={testing || !settings.endpoint || !settings.model}
              >
                {testing ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    Testing...
                  </>
                ) : (
                  'Test Connection'
                )}
              </Button>

              {testResult === 'success' && (
                <span className="flex items-center gap-1 text-xs text-green-600">
                  <Check className="w-3.5 h-3.5" />
                  Connected
                </span>
              )}
              {testResult === 'error' && (
                <span className="flex items-center gap-1 text-xs text-red-600">
                  <AlertTriangle className="w-3.5 h-3.5" />
                  {testError || 'Failed'}
                </span>
              )}
            </div>
          </div>

          {/* Footer */}
          <div className="px-5 py-3 border-t border-gray-200 bg-gray-50 rounded-b-xl">
            <Button variant="primary" size="sm" onClick={closeSettings}>
              Done
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}
