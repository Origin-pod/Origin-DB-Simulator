import { useState, useCallback, useEffect, useRef } from 'react';
import { X, Settings, Trash2, Info, HelpCircle, RotateCcw, BookOpen } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { BlockEducationPanel } from '@/components/education/BlockEducationPanel';
import { useCanvasStore } from '@/stores/canvasStore';
import {
  CATEGORY_COLORS,
  DATA_TYPE_COLORS,
  type BlockNodeData,
  type PortDefinition,
  type ParameterDefinition,
  type ParameterConstraints,
} from '@/types';
import { getBlockDefinition } from '@/types/blocks';

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

interface ValidationResult {
  valid: boolean;
  error?: string;
}

function validateParam(
  value: string | number | boolean,
  param: ParameterDefinition,
): ValidationResult {
  if (param.type === 'boolean') return { valid: true };

  if (param.type === 'enum') {
    const opts = param.constraints?.options ?? [];
    if (opts.length > 0 && !opts.includes(String(value))) {
      return { valid: false, error: `Must be one of: ${opts.join(', ')}` };
    }
    return { valid: true };
  }

  if (param.type === 'number') {
    const n = Number(value);
    if (Number.isNaN(n)) return { valid: false, error: 'Must be a number' };
    const c = param.constraints;
    if (c?.min !== undefined && n < c.min)
      return { valid: false, error: `Minimum value is ${c.min}` };
    if (c?.max !== undefined && n > c.max)
      return { valid: false, error: `Maximum value is ${c.max}` };
    return { valid: true };
  }

  // string
  const s = String(value);
  const c = param.constraints;
  if (c?.pattern) {
    try {
      if (!new RegExp(c.pattern).test(s)) {
        return { valid: false, error: `Must match pattern: ${c.pattern}` };
      }
    } catch {
      // invalid regex in definition — skip
    }
  }
  return { valid: true };
}

// ---------------------------------------------------------------------------
// Tooltip component
// ---------------------------------------------------------------------------

function Tooltip({
  param,
  show,
  onToggle,
}: {
  param: ParameterDefinition;
  show: boolean;
  onToggle: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!show) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onToggle();
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [show, onToggle]);

  return (
    <div className="relative" ref={ref}>
      <button
        className="text-gray-400 hover:text-gray-600"
        onClick={onToggle}
        title={param.description}
      >
        <HelpCircle className="w-3 h-3" />
      </button>
      {show && (
        <div className="absolute right-0 top-5 z-50 w-56 p-3 bg-white rounded-lg shadow-lg border border-gray-200 text-xs">
          <p className="text-gray-700 mb-1.5">{param.description}</p>
          <p className="text-gray-500">
            Default: <span className="font-mono text-gray-700">{String(param.default)}</span>
          </p>
          {param.constraints?.min !== undefined && (
            <p className="text-gray-500 mt-0.5">
              Range: {param.constraints.min} – {param.constraints.max}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Individual parameter input components
// ---------------------------------------------------------------------------

function BooleanInput({
  value,
  onChange,
}: {
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <input
        type="checkbox"
        checked={value}
        onChange={(e) => onChange(e.target.checked)}
        className="w-4 h-4 rounded border-gray-300 text-primary-500 focus:ring-primary-500 cursor-pointer"
      />
      <span className="text-xs text-gray-500">
        {value ? 'Enabled' : 'Disabled'}
      </span>
    </div>
  );
}

function EnumInput({
  value,
  options,
  onChange,
}: {
  value: string;
  options: string[];
  onChange: (v: string) => void;
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent cursor-pointer"
    >
      {options.map((opt) => (
        <option key={opt} value={opt}>
          {opt}
        </option>
      ))}
    </select>
  );
}

function SliderInput({
  value,
  constraints,
  onChange,
}: {
  value: number;
  constraints: ParameterConstraints;
  onChange: (v: number) => void;
}) {
  return (
    <div className="space-y-1">
      <input
        type="range"
        value={value}
        min={constraints.min}
        max={constraints.max}
        step={constraints.step}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-primary-500"
      />
      <div className="flex justify-between text-xs text-gray-500">
        <span>{constraints.min}</span>
        <span className="font-medium text-gray-700">{value}</span>
        <span>{constraints.max}</span>
      </div>
    </div>
  );
}

function NumberInput({
  value,
  constraints,
  error,
  onChange,
}: {
  value: number;
  constraints?: ParameterConstraints;
  error?: string;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <input
        type="number"
        value={value}
        min={constraints?.min}
        max={constraints?.max}
        step={constraints?.step}
        onChange={(e) => onChange(Number(e.target.value))}
        className={`w-full px-2 py-1.5 text-sm border rounded-lg bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent ${
          error ? 'border-red-400' : 'border-gray-200'
        }`}
      />
      {error && <p className="text-xs text-red-500 mt-0.5">{error}</p>}
    </div>
  );
}

function TextInput({
  value,
  error,
  onChange,
}: {
  value: string;
  error?: string;
  onChange: (v: string) => void;
}) {
  return (
    <div>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`w-full px-2 py-1.5 text-sm border rounded-lg bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent ${
          error ? 'border-red-400' : 'border-gray-200'
        }`}
      />
      {error && <p className="text-xs text-red-500 mt-0.5">{error}</p>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Parameter row (label + tooltip + input)
// ---------------------------------------------------------------------------

function ParameterRow({
  param,
  value,
  onUpdate,
}: {
  param: ParameterDefinition;
  value: string | number | boolean;
  onUpdate: (name: string, value: string | number | boolean) => void;
}) {
  const [showTooltip, setShowTooltip] = useState(false);
  const [localValue, setLocalValue] = useState(value);
  const [error, setError] = useState<string | undefined>();
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  // Sync local state when value changes from outside (e.g. reset)
  useEffect(() => {
    setLocalValue(value);
    setError(undefined);
  }, [value]);

  const commitValue = useCallback(
    (v: string | number | boolean) => {
      const result = validateParam(v, param);
      if (result.valid) {
        setError(undefined);
        onUpdate(param.name, v);
      } else {
        setError(result.error);
      }
    },
    [param, onUpdate],
  );

  // Debounced commit for text and number inputs
  const debouncedCommit = useCallback(
    (v: string | number | boolean) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => commitValue(v), 300);
    },
    [commitValue],
  );

  const handleChange = useCallback(
    (v: string | number | boolean) => {
      setLocalValue(v);
      // Booleans and enums commit immediately; text/number debounce
      if (param.type === 'boolean' || param.type === 'enum') {
        commitValue(v);
      } else if (param.uiHint === 'slider') {
        // Sliders commit immediately (constrained range)
        commitValue(v);
      } else {
        debouncedCommit(v);
      }
    },
    [param, commitValue, debouncedCommit],
  );

  const toggleTooltip = useCallback(() => setShowTooltip((s) => !s), []);

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <label className="text-xs font-medium text-gray-700">
          {param.name}
        </label>
        <Tooltip param={param} show={showTooltip} onToggle={toggleTooltip} />
      </div>

      {param.type === 'boolean' ? (
        <BooleanInput
          value={Boolean(localValue)}
          onChange={(v) => handleChange(v)}
        />
      ) : param.type === 'enum' && param.constraints?.options ? (
        <EnumInput
          value={String(localValue)}
          options={param.constraints.options}
          onChange={(v) => handleChange(v)}
        />
      ) : param.uiHint === 'slider' && param.constraints ? (
        <SliderInput
          value={Number(localValue)}
          constraints={param.constraints}
          onChange={(v) => handleChange(v)}
        />
      ) : param.type === 'number' ? (
        <NumberInput
          value={Number(localValue)}
          constraints={param.constraints}
          error={error}
          onChange={(v) => handleChange(v)}
        />
      ) : (
        <TextInput
          value={String(localValue)}
          error={error}
          onChange={(v) => handleChange(v)}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main ParameterPanel component
// ---------------------------------------------------------------------------

export function ParameterPanel() {
  const { nodes, selectedNodeId, setSelectedNode, removeNode, updateNodeData } =
    useCanvasStore();

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const data = selectedNode?.data as BlockNodeData | undefined;
  const blockDef = data ? getBlockDefinition(data.blockType) : undefined;

  const handleParamUpdate = useCallback(
    (name: string, value: string | number | boolean) => {
      if (!selectedNode || !data) return;
      updateNodeData(selectedNode.id, {
        parameters: { ...data.parameters, [name]: value },
      });
    },
    [selectedNode?.id, data?.parameters, updateNodeData],
  );

  const handleResetDefaults = useCallback(() => {
    if (!blockDef || !selectedNode) return;
    const defaults: Record<string, string | number | boolean> = {};
    for (const p of blockDef.parameters) {
      defaults[p.name] = p.default;
    }
    updateNodeData(selectedNode.id, { parameters: defaults });
  }, [blockDef, selectedNode?.id, updateNodeData]);

  const [activeTab, setActiveTab] = useState<'configure' | 'learn'>('configure');
  const hasEducation = !!(blockDef?.documentation?.overview || blockDef?.documentation?.algorithm || blockDef?.documentation?.details);

  if (!selectedNode || !data) {
    return (
      <aside className="w-72 bg-white border-l border-gray-200 flex flex-col">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center px-6">
            <Settings className="w-12 h-12 text-gray-300 mx-auto mb-3" />
            <p className="text-sm text-gray-500">
              Select a block on the canvas to view its configuration
            </p>
          </div>
        </div>
      </aside>
    );
  }

  const categoryColor = CATEGORY_COLORS[data.category];

  const handleDelete = () => {
    removeNode(selectedNode.id);
  };

  return (
    <aside className="w-72 bg-white border-l border-gray-200 flex flex-col">
      {/* Header */}
      <div
        className="p-4 border-b"
        style={{ borderBottomColor: categoryColor, borderBottomWidth: 3 }}
      >
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold text-gray-900">{data.label}</h3>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDelete}
              className="text-gray-400 hover:text-error"
              title="Delete block"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSelectedNode(null)}
              className="text-gray-400 hover:text-gray-600"
              title="Close panel"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span
            className="text-xs font-medium px-2 py-0.5 rounded"
            style={{ backgroundColor: `${categoryColor}20`, color: categoryColor }}
          >
            {data.category.charAt(0).toUpperCase() + data.category.slice(1)}
          </span>
        </div>
        {blockDef?.description && (
          <p className="text-xs text-gray-500 mt-2">{blockDef.description}</p>
        )}
      </div>

      {/* Tabs */}
      {hasEducation && (
        <div className="flex border-b border-gray-200">
          <button
            onClick={() => setActiveTab('configure')}
            className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors ${
              activeTab === 'configure'
                ? 'text-gray-900 border-b-2 border-gray-900'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            Configure
          </button>
          <button
            onClick={() => setActiveTab('learn')}
            className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors ${
              activeTab === 'learn'
                ? 'text-blue-600 border-b-2 border-blue-600'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            <BookOpen className="w-3.5 h-3.5" />
            Learn
          </button>
        </div>
      )}

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'configure' ? (
          <>
            {/* Parameters Section */}
            {blockDef && blockDef.parameters.length > 0 && (
              <div className="p-4 border-b border-gray-100">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="text-sm font-medium text-gray-900 flex items-center gap-2">
                    <Settings className="w-4 h-4 text-gray-400" />
                    Configuration
                  </h4>
                  <button
                    onClick={handleResetDefaults}
                    className="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
                    title="Reset all parameters to defaults"
                  >
                    <RotateCcw className="w-3 h-3" />
                    Reset
                  </button>
                </div>
                <div className="space-y-3">
                  {blockDef.parameters.map((param) => {
                    const value = data.parameters[param.name] ?? param.default;
                    return (
                      <ParameterRow
                        key={param.name}
                        param={param}
                        value={value}
                        onUpdate={handleParamUpdate}
                      />
                    );
                  })}
                </div>
              </div>
            )}

            {/* Ports Section */}
            <div className="p-4">
              <h4 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
                <Info className="w-4 h-4 text-gray-400" />
                Ports
              </h4>

              {data.inputs.length > 0 && (
                <div className="mb-4">
                  <p className="text-xs font-medium text-gray-500 mb-2">Inputs</p>
                  <div className="space-y-2">
                    {data.inputs.map((port: PortDefinition) => (
                      <div
                        key={port.name}
                        className="flex items-center gap-2 p-2 bg-gray-50 rounded-lg"
                      >
                        <span
                          className="w-3 h-3 rounded-full flex-shrink-0"
                          style={{ backgroundColor: DATA_TYPE_COLORS[port.dataType] }}
                        />
                        <div className="flex-1 min-w-0">
                          <span className="text-sm text-gray-700 block truncate">
                            {port.name}
                          </span>
                          <span className="text-xs text-gray-400">
                            {port.dataType}
                            {port.required && (
                              <span className="text-red-500 ml-1">*</span>
                            )}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {data.outputs.length > 0 && (
                <div>
                  <p className="text-xs font-medium text-gray-500 mb-2">Outputs</p>
                  <div className="space-y-2">
                    {data.outputs.map((port: PortDefinition) => (
                      <div
                        key={port.name}
                        className="flex items-center gap-2 p-2 bg-gray-50 rounded-lg"
                      >
                        <span
                          className="w-3 h-3 rounded-full flex-shrink-0"
                          style={{ backgroundColor: DATA_TYPE_COLORS[port.dataType] }}
                        />
                        <div className="flex-1 min-w-0">
                          <span className="text-sm text-gray-700 block truncate">
                            {port.name}
                          </span>
                          <span className="text-xs text-gray-400">{port.dataType}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {data.inputs.length === 0 && data.outputs.length === 0 && (
                <p className="text-sm text-gray-500 text-center py-4">
                  This block has no ports
                </p>
              )}
            </div>
          </>
        ) : (
          /* Learn tab — education content expanded */
          <div className="p-4">
            {blockDef ? (
              <BlockEducationPanel block={blockDef} compact={false} />
            ) : (
              <p className="text-sm text-gray-500 text-center py-8">
                No documentation available for this block.
              </p>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-gray-200 bg-gray-50">
        <p className="text-xs text-gray-400 font-mono">
          ID: {selectedNode.id}
        </p>
      </div>
    </aside>
  );
}
