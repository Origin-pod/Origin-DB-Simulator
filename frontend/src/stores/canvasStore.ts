import { create } from 'zustand';
import {
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
  type Node,
  type Edge,
  type NodeChange,
  type EdgeChange,
  type Connection,
} from '@xyflow/react';
import type { BlockNodeData } from '@/types';

type BlockNode = Node<BlockNodeData>;
type SchemaEdge = Edge;

interface CanvasState {
  // State
  nodes: BlockNode[];
  edges: SchemaEdge[];
  selectedNodeId: string | null;
  designName: string;

  // Node actions
  onNodesChange: (changes: NodeChange[]) => void;
  addNode: (node: BlockNode) => void;
  removeNode: (nodeId: string) => void;
  updateNodeData: (nodeId: string, data: Partial<BlockNodeData>) => void;

  // Edge actions
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  removeEdge: (edgeId: string) => void;

  // Selection
  setSelectedNode: (nodeId: string | null) => void;

  // Design
  setDesignName: (name: string) => void;
  clearCanvas: () => void;
}

export const useCanvasStore = create<CanvasState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  designName: 'Untitled Design',

  onNodesChange: (changes) => {
    set({
      nodes: applyNodeChanges(changes, get().nodes) as BlockNode[],
    });
  },

  addNode: (node) => {
    set({
      nodes: [...get().nodes, node],
    });
  },

  removeNode: (nodeId) => {
    set({
      nodes: get().nodes.filter((n) => n.id !== nodeId),
      edges: get().edges.filter(
        (e) => e.source !== nodeId && e.target !== nodeId
      ),
      selectedNodeId:
        get().selectedNodeId === nodeId ? null : get().selectedNodeId,
    });
  },

  updateNodeData: (nodeId, data) => {
    set({
      nodes: get().nodes.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, ...data } }
          : node
      ),
    });
  },

  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges(changes, get().edges),
    });
  },

  onConnect: (connection) => {
    // Prevent connecting a node to itself
    if (connection.source === connection.target) {
      return;
    }

    set({
      edges: addEdge(
        {
          ...connection,
          type: 'smoothstep',
          animated: false,
          style: { strokeWidth: 2, stroke: '#94A3B8' },
        },
        get().edges
      ),
    });
  },

  removeEdge: (edgeId) => {
    set({
      edges: get().edges.filter((e) => e.id !== edgeId),
    });
  },

  setSelectedNode: (nodeId) => {
    set({ selectedNodeId: nodeId });
  },

  setDesignName: (name) => {
    set({ designName: name });
  },

  clearCanvas: () => {
    set({
      nodes: [],
      edges: [],
      selectedNodeId: null,
    });
  },
}));
