import { useCallback, useEffect, type DragEvent } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  useReactFlow,
  type NodeTypes,
  type Node,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useCanvasStore } from '@/stores/canvasStore';
import { useDesignStore } from '@/stores/designStore';
import { useWorkloadStore } from '@/stores/workloadStore';
import { useWikiStore } from '@/stores/wikiStore';
import { BlockNode } from '@/components/nodes/BlockNode';
import { CATEGORY_COLORS, type BlockCategory, type BlockNodeData } from '@/types';
import { getBlockDefinition } from '@/types/blocks';
import { validateImport } from '@/lib/persistence';
import { toast } from '@/stores/toastStore';

// Register custom node types
const nodeTypes: NodeTypes = {
  blockNode: BlockNode,
};

// Block definitions for creating nodes from palette drops
interface PaletteBlock {
  type: string;
  name: string;
  description: string;
  category: BlockCategory;
}

export function Canvas() {
  const {
    nodes, edges, onNodesChange, onEdgesChange, onConnect, addNode,
    setSelectedNode, removeNode, selectedNodeId, undo, redo, canUndo, canRedo,
  } = useCanvasStore();
  const { screenToFlowPosition } = useReactFlow();

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger when typing in an input/textarea
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      const isMeta = e.metaKey || e.ctrlKey;

      // Cmd/Ctrl+Z = Undo, Cmd/Ctrl+Shift+Z = Redo
      if (isMeta && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          if (canRedo) redo();
        } else {
          if (canUndo) undo();
        }
        return;
      }

      // Delete / Backspace = remove selected node
      if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNodeId) {
        e.preventDefault();
        removeNode(selectedNodeId);
        return;
      }

      // ? = open wiki for selected block
      if (e.key === '?' && selectedNodeId) {
        const node = useCanvasStore.getState().nodes.find((n) => n.id === selectedNodeId);
        if (node) {
          useWikiStore.getState().open((node.data as BlockNodeData).blockType);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedNodeId, removeNode, undo, redo, canUndo, canRedo]);

  const handleDragOver = useCallback((event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDrop = useCallback(
    (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();

      // Check if it's a file drop (import design)
      const files = event.dataTransfer.files;
      if (files.length > 0) {
        const file = files[0];
        if (file.name.endsWith('.json') || file.name.endsWith('.dbsim')) {
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
              const { saveCurrentCanvas, createDesign, setDesignResult } = useDesignStore.getState();
              saveCurrentCanvas();
              const newId = createDesign(d.name);
              useCanvasStore.getState().loadDesign(d.name, d.nodes, d.edges);
              if (d.workload) {
                useWorkloadStore.setState({ workload: d.workload });
              }
              if (d.lastResult) {
                setDesignResult(newId, d.lastResult);
              }
              toast.success(`Imported "${d.name}" from file.`);
            } catch {
              toast.error('Failed to parse dropped file.');
            }
          };
          reader.readAsText(file);
          return;
        }
      }

      // Otherwise it's a block palette drop
      const data = event.dataTransfer.getData('application/json');
      if (!data) return;

      const block: PaletteBlock = JSON.parse(data);
      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      // Get full block definition from registry
      const blockDef = getBlockDefinition(block.type);
      const inputs = blockDef?.inputs || [];
      const outputs = blockDef?.outputs || [];
      const icon = blockDef?.icon || 'Database';

      // Initialize parameters with default values
      const parameters: Record<string, string | number | boolean> = {};
      if (blockDef?.parameters) {
        for (const param of blockDef.parameters) {
          parameters[param.name] = param.default;
        }
      }

      const newNode: Node<BlockNodeData> = {
        id: `${block.type}-${Date.now()}`,
        type: 'blockNode',
        position,
        data: {
          blockType: block.type,
          label: block.name,
          category: block.category,
          icon: icon,
          color: CATEGORY_COLORS[block.category],
          inputs: inputs,
          outputs: outputs,
          parameters: parameters,
          state: 'idle' as const,
        },
      };

      addNode(newNode);
    },
    [screenToFlowPosition, addNode],
  );

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode]
  );

  const handleNodeDoubleClick = useCallback((_: React.MouseEvent, node: Node) => {
    useWikiStore.getState().open((node.data as BlockNodeData).blockType);
  }, []);

  const handlePaneClick = useCallback(() => {
    setSelectedNode(null);
  }, [setSelectedNode]);

  return (
    <div className="flex-1 h-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={handleNodeClick}
        onNodeDoubleClick={handleNodeDoubleClick}
        onPaneClick={handlePaneClick}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid
        snapGrid={[16, 16]}
        defaultEdgeOptions={{
          type: 'smoothstep',
          style: { strokeWidth: 2, stroke: '#94A3B8' },
        }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color="#D1D5DB"
        />
        <Controls
          position="bottom-right"
          className="!bg-white !border !border-gray-200 !rounded-lg !shadow-sm"
        />
        <MiniMap
          position="bottom-right"
          className="!bg-white !border !border-gray-200 !rounded-lg !shadow-sm"
          style={{ marginBottom: 60 }}
          nodeColor={(node) => {
            const category = (node.data as BlockNodeData)?.category;
            return category ? CATEGORY_COLORS[category] : '#94A3B8';
          }}
        />
      </ReactFlow>
    </div>
  );
}
