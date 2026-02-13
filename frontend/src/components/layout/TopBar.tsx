import { useState, useCallback, useEffect } from 'react';
import {
  Database,
  Play,
  Square,
  GitCompare,
  Check,
  Pencil,
  ListChecks,
  LayoutTemplate,
  Undo2,
  Redo2,
  Download,
  Upload,
  Loader2,
  CheckCircle2,
  Cloud,
  Sparkles,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useCanvasStore } from '@/stores/canvasStore';
import { useWorkloadStore } from '@/stores/workloadStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useDesignStore } from '@/stores/designStore';
import type { BlockNodeData } from '@/types';
import { exportDesign, validateImport, downloadFile } from '@/lib/persistence';
import { toast } from '@/stores/toastStore';
import { useAIStore } from '@/stores/aiStore';

interface TopBarProps {
  onOpenComparison: () => void;
  onOpenTemplates: () => void;
}

export function TopBar({ onOpenComparison, onOpenTemplates }: TopBarProps) {
  const { designName, setDesignName, nodes, edges, undo, redo, canUndo, canRedo } = useCanvasStore();
  const { openEditor, workload } = useWorkloadStore();
  const { status, result, startExecution, cancelExecution, engineType } = useExecutionStore();
  const { activeDesignId, setDesignResult, saveCurrentCanvas, renameDesign, saveStatus, createDesign } =
    useDesignStore();
  const [isEditing, setIsEditing] = useState(false);
  const [editedName, setEditedName] = useState(designName);

  const isRunning = status === 'running' || status === 'validating';

  // Sync edited name when design changes
  useEffect(() => {
    setEditedName(designName);
  }, [designName]);

  // Save execution result to the design when execution completes
  useEffect(() => {
    if (status === 'complete' && result) {
      saveCurrentCanvas();
      setDesignResult(activeDesignId, result);
    }
  }, [status, result, activeDesignId, setDesignResult, saveCurrentCanvas]);

  const handleNameSubmit = () => {
    if (editedName.trim()) {
      setDesignName(editedName.trim());
      renameDesign(activeDesignId, editedName.trim());
    } else {
      setEditedName(designName);
    }
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleNameSubmit();
    } else if (e.key === 'Escape') {
      setEditedName(designName);
      setIsEditing(false);
    }
  };

  const handleRun = useCallback(() => {
    if (isRunning) {
      cancelExecution();
    } else {
      startExecution(
        nodes as import('@xyflow/react').Node<BlockNodeData>[],
        edges,
        workload,
      );
    }
  }, [isRunning, cancelExecution, startExecution, nodes, edges, workload]);

  const handleExport = useCallback(() => {
    const activeDesign = useDesignStore.getState().designs.find(
      (d) => d.id === activeDesignId,
    );
    const data = exportDesign(
      designName,
      nodes as import('@xyflow/react').Node<BlockNodeData>[],
      edges,
      workload,
      activeDesign?.lastResult ?? null,
    );
    const json = JSON.stringify(data, null, 2);
    const safeName = designName.replace(/[^a-zA-Z0-9_-]/g, '_').toLowerCase();
    downloadFile(`${safeName}.dbsim.json`, json);
    toast.success('Design exported.');
  }, [activeDesignId, designName, nodes, edges, workload]);

  const handleImport = useCallback(() => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,.dbsim,.dbsim.json';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        try {
          const parsed = JSON.parse(reader.result as string);
          const result = validateImport(parsed);
          if (!result.valid || !result.design) {
            toast.error(result.error ?? 'Invalid file.');
            return;
          }
          const d = result.design.design;
          // Create a new design from imported data
          saveCurrentCanvas();
          const newId = createDesign(d.name);
          // Load nodes/edges into canvas
          const canvasStore = useCanvasStore.getState();
          canvasStore.loadDesign(d.name, d.nodes, d.edges);
          // Load workload if present
          if (d.workload) {
            useWorkloadStore.setState({ workload: d.workload });
          }
          // Store result if present
          if (d.lastResult) {
            useDesignStore.getState().setDesignResult(newId, d.lastResult);
          }
          toast.success(`Imported "${d.name}".`);
        } catch {
          toast.error('Failed to parse file. Make sure it is valid JSON.');
        }
      };
      reader.readAsText(file);
    };
    input.click();
  }, [saveCurrentCanvas, createDesign]);

  return (
    <header className="h-14 bg-white border-b border-gray-200 flex items-center justify-between px-4">
      {/* Logo */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
            <Database className="w-5 h-5 text-white" />
          </div>
          <span className="font-semibold text-gray-900">DB Simulator</span>
        </div>
      </div>

      {/* Design Name + Save indicator */}
      <div className="flex items-center gap-2">
        {isEditing ? (
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={editedName}
              onChange={(e) => setEditedName(e.target.value)}
              onBlur={handleNameSubmit}
              onKeyDown={handleKeyDown}
              className="px-3 py-1.5 text-sm border border-primary-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              autoFocus
            />
            <Button variant="ghost" size="sm" onClick={handleNameSubmit}>
              <Check className="w-4 h-4" />
            </Button>
          </div>
        ) : (
          <button
            onClick={() => setIsEditing(true)}
            className="flex items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <span>{designName}</span>
            <Pencil className="w-3.5 h-3.5 text-gray-400" />
          </button>
        )}
        {/* Save status indicator */}
        <span className="flex items-center gap-1 text-xs text-gray-400">
          {saveStatus === 'saving' && (
            <>
              <Loader2 className="w-3 h-3 animate-spin" />
              Saving...
            </>
          )}
          {saveStatus === 'saved' && (
            <>
              <CheckCircle2 className="w-3 h-3 text-green-500" />
              Saved
            </>
          )}
          {saveStatus === 'unsaved' && (
            <>
              <Cloud className="w-3 h-3" />
              Unsaved
            </>
          )}
        </span>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-0.5 mr-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={undo}
            disabled={!canUndo || isRunning}
            title="Undo (Cmd+Z)"
          >
            <Undo2 className="w-4 h-4" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={redo}
            disabled={!canRedo || isRunning}
            title="Redo (Cmd+Shift+Z)"
          >
            <Redo2 className="w-4 h-4" />
          </Button>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={onOpenTemplates}
          disabled={isRunning}
        >
          <LayoutTemplate className="w-4 h-4" />
          Templates
        </Button>
        <Button
          variant="secondary"
          size="sm"
          onClick={openEditor}
          disabled={isRunning}
        >
          <ListChecks className="w-4 h-4" />
          Workload
          <span className="text-xs text-gray-400 ml-0.5">
            ({workload.operations.length})
          </span>
        </Button>
        <div className="flex items-center gap-1">
          <Button variant="primary" size="sm" onClick={handleRun}>
            {isRunning ? (
              <>
                <Square className="w-4 h-4" />
                Stop
              </>
            ) : (
              <>
                <Play className="w-4 h-4" />
                Run
              </>
            )}
          </Button>
          <span
            className={`text-[9px] font-semibold px-1.5 py-0.5 rounded ${
              engineType === 'wasm'
                ? 'bg-green-100 text-green-700'
                : 'bg-gray-100 text-gray-500'
            }`}
            title={engineType === 'wasm' ? 'Using WASM engine (Rust)' : 'Using mock engine (simulated)'}
          >
            {engineType === 'wasm' ? 'WASM' : 'Mock'}
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => useAIStore.getState().togglePanel()}
          title="AI Teaching Assistant"
        >
          <Sparkles className="w-4 h-4" />
          AI Assist
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={isRunning}
          onClick={onOpenComparison}
        >
          <GitCompare className="w-4 h-4" />
          Compare
        </Button>
        <Button variant="ghost" size="sm" onClick={handleExport} disabled={isRunning}>
          <Download className="w-4 h-4" />
          Export
        </Button>
        <Button variant="ghost" size="sm" onClick={handleImport} disabled={isRunning}>
          <Upload className="w-4 h-4" />
          Import
        </Button>
      </div>
    </header>
  );
}
