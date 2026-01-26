import { useCallback, type DragEvent } from 'react';
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
import { BlockNode } from '@/components/nodes/BlockNode';
import { CATEGORY_COLORS, type BlockCategory, type BlockNodeData } from '@/types';
import { getBlockDefinition } from '@/types/blocks';

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
  const { nodes, edges, onNodesChange, onEdgesChange, onConnect, addNode, setSelectedNode } =
    useCanvasStore();
  const { screenToFlowPosition } = useReactFlow();

  const handleDragOver = useCallback((event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDrop = useCallback(
    (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();

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
    [screenToFlowPosition, addNode]
  );

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode]
  );

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
