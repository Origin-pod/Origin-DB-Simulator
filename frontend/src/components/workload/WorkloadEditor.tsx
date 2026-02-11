import { useCallback } from 'react';
import {
  X,
  Plus,
  Trash2,
  Zap,
  Shuffle,
  Clock,
  Activity,
  FileText,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import {
  useWorkloadStore,
  WORKLOAD_PRESETS,
  type OperationType,
  type Distribution,
  type Operation,
} from '@/stores/workloadStore';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const OPERATION_TYPES: { value: OperationType; label: string; color: string }[] = [
  { value: 'SELECT', label: 'SELECT', color: '#3B82F6' },
  { value: 'INSERT', label: 'INSERT', color: '#10B981' },
  { value: 'UPDATE', label: 'UPDATE', color: '#F59E0B' },
  { value: 'DELETE', label: 'DELETE', color: '#EF4444' },
  { value: 'SCAN', label: 'SCAN', color: '#8B5CF6' },
];

const DISTRIBUTIONS: { value: Distribution; label: string; description: string }[] = [
  { value: 'zipfian', label: 'Zipfian', description: 'Hot keys — realistic OLTP' },
  { value: 'uniform', label: 'Uniform', description: 'All keys equally likely' },
  { value: 'latest', label: 'Latest', description: 'Recent records more likely' },
];

function getOpColor(type: OperationType): string {
  return OPERATION_TYPES.find((o) => o.value === type)?.color ?? '#6B7280';
}

// ---------------------------------------------------------------------------
// Operation row
// ---------------------------------------------------------------------------

function OperationRow({
  operation,
  canDelete,
}: {
  operation: Operation;
  canDelete: boolean;
}) {
  const {
    removeOperation,
    updateOperationType,
    updateOperationWeight,
    updateOperationTemplate,
  } = useWorkloadStore();

  const color = getOpColor(operation.type);

  return (
    <div className="border border-gray-200 rounded-lg p-3 space-y-2.5">
      {/* Top: type selector + delete */}
      <div className="flex items-center gap-2">
        <span
          className="w-2 h-8 rounded-full flex-shrink-0"
          style={{ backgroundColor: color }}
        />
        <select
          value={operation.type}
          onChange={(e) => updateOperationType(operation.id, e.target.value as OperationType)}
          className="flex-1 px-2 py-1.5 text-sm font-medium border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500"
        >
          {OPERATION_TYPES.map((ot) => (
            <option key={ot.value} value={ot.value}>
              {ot.label}
            </option>
          ))}
        </select>
        {canDelete && (
          <button
            onClick={() => removeOperation(operation.id)}
            className="p-1 text-gray-400 hover:text-red-500 transition-colors"
            title="Remove operation"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Weight slider */}
      <div className="space-y-1">
        <div className="flex items-center justify-between">
          <span className="text-xs text-gray-500">Weight</span>
          <span className="text-xs font-mono font-medium text-gray-700">
            {operation.weight}%
          </span>
        </div>
        <input
          type="range"
          value={operation.weight}
          min={0}
          max={100}
          step={1}
          onChange={(e) => updateOperationWeight(operation.id, Number(e.target.value))}
          className="w-full h-2 rounded-lg appearance-none cursor-pointer accent-primary-500"
          style={{
            background: `linear-gradient(to right, ${color} ${operation.weight}%, #E5E7EB ${operation.weight}%)`,
          }}
        />
      </div>

      {/* Template */}
      <div className="space-y-1">
        <span className="text-xs text-gray-500">Template</span>
        <input
          type="text"
          value={operation.template}
          onChange={(e) => updateOperationTemplate(operation.id, e.target.value)}
          className="w-full px-2 py-1 text-xs font-mono border border-gray-200 rounded bg-gray-50 text-gray-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:bg-white"
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Weight summary bar
// ---------------------------------------------------------------------------

function WeightBar({ operations }: { operations: Operation[] }) {
  const total = operations.reduce((s, op) => s + op.weight, 0);

  return (
    <div className="space-y-1.5">
      {/* Visual bar */}
      <div className="flex h-3 rounded-full overflow-hidden bg-gray-100">
        {operations.map((op) => (
          <div
            key={op.id}
            style={{
              width: `${op.weight}%`,
              backgroundColor: getOpColor(op.type),
            }}
            className="transition-all duration-200"
            title={`${op.type}: ${op.weight}%`}
          />
        ))}
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-3">
        {operations.map((op) => (
          <div key={op.id} className="flex items-center gap-1">
            <span
              className="w-2 h-2 rounded-full"
              style={{ backgroundColor: getOpColor(op.type) }}
            />
            <span className="text-xs text-gray-600">
              {op.type} {op.weight}%
            </span>
          </div>
        ))}
      </div>

      {/* Warning if total != 100 */}
      {total !== 100 && (
        <p className="text-xs text-amber-600">
          Weights sum to {total}% (should be 100%)
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main modal
// ---------------------------------------------------------------------------

export function WorkloadEditor() {
  const {
    workload,
    isEditorOpen,
    closeEditor,
    setWorkloadName,
    setDistribution,
    setConcurrency,
    setTotalOperations,
    addOperation,
    loadPreset,
  } = useWorkloadStore();

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.target === e.currentTarget) closeEditor();
    },
    [closeEditor],
  );

  if (!isEditorOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onClick={handleBackdropClick}
    >
      <div className="bg-white rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <div className="flex items-center gap-2">
            <Activity className="w-5 h-5 text-primary-500" />
            <h2 className="text-lg font-semibold text-gray-900">Define Workload</h2>
          </div>
          <button
            onClick={closeEditor}
            className="p-1 text-gray-400 hover:text-gray-600 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body — scrollable */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
          {/* Workload name */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-gray-700 flex items-center gap-1.5">
              <FileText className="w-4 h-4 text-gray-400" />
              Workload Name
            </label>
            <input
              type="text"
              value={workload.name}
              onChange={(e) => setWorkloadName(e.target.value)}
              className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="My workload"
            />
          </div>

          {/* Presets */}
          <div className="space-y-2">
            <span className="text-sm font-medium text-gray-700">Presets</span>
            <div className="flex flex-wrap gap-2">
              {WORKLOAD_PRESETS.map((preset) => (
                <button
                  key={preset.id}
                  onClick={() => loadPreset(preset.id)}
                  className="px-3 py-1.5 text-xs font-medium border border-gray-200 rounded-lg hover:bg-gray-50 hover:border-primary-300 transition-colors"
                  title={preset.description}
                >
                  {preset.name}
                </button>
              ))}
            </div>
          </div>

          {/* Operations */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-700">Operations</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => addOperation()}
                className="text-primary-600"
              >
                <Plus className="w-4 h-4" />
                Add
              </Button>
            </div>

            {/* Weight overview */}
            <WeightBar operations={workload.operations} />

            {/* Operation list */}
            <div className="space-y-2">
              {workload.operations.map((op) => (
                <OperationRow
                  key={op.id}
                  operation={op}
                  canDelete={workload.operations.length > 1}
                />
              ))}
            </div>
          </div>

          {/* Distribution */}
          <div className="space-y-2">
            <span className="text-sm font-medium text-gray-700 flex items-center gap-1.5">
              <Shuffle className="w-4 h-4 text-gray-400" />
              Key Distribution
            </span>
            <div className="grid grid-cols-3 gap-2">
              {DISTRIBUTIONS.map((d) => (
                <button
                  key={d.value}
                  onClick={() => setDistribution(d.value)}
                  className={`p-3 rounded-lg border-2 text-left transition-colors ${
                    workload.distribution === d.value
                      ? 'border-primary-500 bg-primary-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <span className="text-sm font-medium text-gray-900 block">
                    {d.label}
                  </span>
                  <span className="text-xs text-gray-500">{d.description}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Concurrency + Total Ops */}
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-gray-700 flex items-center gap-1.5">
                <Zap className="w-4 h-4 text-gray-400" />
                Concurrency
              </label>
              <input
                type="number"
                value={workload.concurrency}
                min={1}
                max={1000}
                onChange={(e) => setConcurrency(Number(e.target.value))}
                className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
              <input
                type="range"
                value={workload.concurrency}
                min={1}
                max={1000}
                step={1}
                onChange={(e) => setConcurrency(Number(e.target.value))}
                className="w-full h-2 rounded-lg appearance-none cursor-pointer accent-primary-500"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium text-gray-700 flex items-center gap-1.5">
                <Clock className="w-4 h-4 text-gray-400" />
                Total Operations
              </label>
              <input
                type="number"
                value={workload.totalOperations}
                min={1}
                max={1000000}
                step={1000}
                onChange={(e) => setTotalOperations(Number(e.target.value))}
                className="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
              <div className="flex justify-between text-xs text-gray-400">
                <span>1</span>
                <span>{workload.totalOperations.toLocaleString()}</span>
                <span>1M</span>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-200 bg-gray-50 rounded-b-xl">
          <Button variant="secondary" size="md" onClick={closeEditor}>
            Cancel
          </Button>
          <Button variant="primary" size="md" onClick={closeEditor}>
            Save & Close
          </Button>
        </div>
      </div>
    </div>
  );
}
