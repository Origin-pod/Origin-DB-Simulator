import {
  X,
  AlertTriangle,
  AlertCircle,
  Loader2,
  XCircle,
  MousePointerClick,
  Trash2,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useExecutionStore } from '@/stores/executionStore';
import { useCanvasStore } from '@/stores/canvasStore';
import { useReactFlow } from '@xyflow/react';
import type { ValidationError, ValidationWarning } from '@/engine/types';

// ---------------------------------------------------------------------------
// Validation error panel (floating) — with suggestions & "Show on Canvas"
// ---------------------------------------------------------------------------

function ValidationPanel() {
  const { validation, dismissValidation } = useExecutionStore();
  const { setSelectedNode, removeNode } = useCanvasStore();
  const reactFlow = useReactFlow();

  if (!validation || validation.valid) return null;

  const handleShowOnCanvas = (nodeId: string) => {
    setSelectedNode(nodeId);
    // Zoom to the node
    const node = useCanvasStore.getState().nodes.find((n) => n.id === nodeId);
    if (node) {
      reactFlow.setCenter(
        node.position.x + 100,
        node.position.y + 40,
        { zoom: 1.2, duration: 400 },
      );
    }
    dismissValidation();
  };

  const handleDeleteBlock = (nodeId: string) => {
    removeNode(nodeId);
  };

  const renderItem = (
    item: ValidationError | ValidationWarning,
    index: number,
    type: 'error' | 'warning',
  ) => {
    const isError = type === 'error';
    return (
      <div key={`${type}-${index}`} className="space-y-1">
        <div className="flex items-start gap-2">
          {isError ? (
            <XCircle className="w-4 h-4 text-red-500 mt-0.5 flex-shrink-0" />
          ) : (
            <AlertTriangle className="w-4 h-4 text-amber-500 mt-0.5 flex-shrink-0" />
          )}
          <span className={`text-sm ${isError ? 'text-gray-700' : 'text-gray-600'}`}>
            {item.message}
          </span>
        </div>
        {item.suggestion && (
          <p className="ml-6 text-xs text-gray-400 italic">
            → {item.suggestion}
          </p>
        )}
        {item.nodeId && (
          <div className="ml-6 flex items-center gap-2">
            <button
              onClick={() => handleShowOnCanvas(item.nodeId!)}
              className="flex items-center gap-1 text-xs text-primary-500 hover:text-primary-700 font-medium"
            >
              <MousePointerClick className="w-3 h-3" />
              Show on Canvas
            </button>
            <button
              onClick={() => handleDeleteBlock(item.nodeId!)}
              className="flex items-center gap-1 text-xs text-gray-400 hover:text-red-500"
            >
              <Trash2 className="w-3 h-3" />
              Delete Block
            </button>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-40 w-full max-w-lg">
      <div className="bg-white rounded-xl shadow-2xl border border-red-200 overflow-hidden">
        <div className="flex items-center justify-between px-4 py-3 bg-red-50 border-b border-red-200">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-5 h-5 text-red-500" />
            <span className="text-sm font-semibold text-red-800">
              Design Has Issues
            </span>
            <span className="text-xs text-red-500">
              ({validation.errors.length} error{validation.errors.length !== 1 ? 's' : ''}
              {validation.warnings.length > 0 &&
                `, ${validation.warnings.length} warning${validation.warnings.length !== 1 ? 's' : ''}`})
            </span>
          </div>
          <button
            onClick={dismissValidation}
            className="p-1 text-red-400 hover:text-red-600"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="px-4 py-3 space-y-3 max-h-64 overflow-y-auto">
          {validation.errors.map((err, i) => renderItem(err, i, 'error'))}
          {validation.warnings.map((warn, i) => renderItem(warn, i, 'warning'))}
        </div>
        <div className="flex items-center justify-end px-4 py-3 border-t border-gray-100 bg-gray-50">
          <Button variant="secondary" size="sm" onClick={dismissValidation}>
            Dismiss
          </Button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Progress bar (floating, during execution)
// ---------------------------------------------------------------------------

function ProgressBar() {
  const { status, progress, progressMessage, cancelExecution } =
    useExecutionStore();

  if (status !== 'validating' && status !== 'running') return null;

  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-40 w-full max-w-md">
      <div className="bg-white rounded-xl shadow-2xl border border-gray-200 px-5 py-4">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <Loader2 className="w-4 h-4 text-primary-500 animate-spin" />
            <span className="text-sm font-medium text-gray-900">
              {status === 'validating' ? 'Validating...' : 'Running workload...'}
            </span>
          </div>
          <button
            onClick={cancelExecution}
            className="text-xs text-gray-400 hover:text-gray-600"
          >
            Cancel
          </button>
        </div>
        <div className="w-full h-2 bg-gray-100 rounded-full overflow-hidden">
          <div
            className="h-full bg-primary-500 rounded-full transition-all duration-300 ease-out"
            style={{ width: `${progress}%` }}
          />
        </div>
        <p className="text-xs text-gray-500 mt-1.5">{progressMessage}</p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Error panel (floating, for runtime errors — not validation)
// ---------------------------------------------------------------------------

function ErrorPanel() {
  const { status, error, validation, clearResults } = useExecutionStore();

  if (status !== 'error' || !error || (validation && !validation.valid)) return null;

  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-40 w-full max-w-md">
      <div className="bg-white rounded-xl shadow-2xl border border-red-200 px-5 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <XCircle className="w-5 h-5 text-red-500" />
            <span className="text-sm font-medium text-gray-900">{error}</span>
          </div>
          <button onClick={clearResults} className="p-1 text-gray-400 hover:text-gray-600">
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main overlay (floating panels only — dashboard is docked separately)
// ---------------------------------------------------------------------------

export function ExecutionOverlay() {
  return (
    <>
      <ValidationPanel />
      <ProgressBar />
      <ErrorPanel />
    </>
  );
}
