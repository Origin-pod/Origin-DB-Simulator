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
import type { BlockNodeData, PortDataType } from '@/types';
import { toast } from '@/stores/toastStore';

type BlockNode = Node<BlockNodeData>;
type SchemaEdge = Edge;

// ---------------------------------------------------------------------------
// Port data-type compatibility matrix
// ---------------------------------------------------------------------------

const PORT_COMPAT: Record<PortDataType, Set<PortDataType>> = {
  DataStream: new Set(['DataStream', 'Batch']),
  Batch: new Set(['Batch', 'DataStream']),
  SingleValue: new Set(['SingleValue']),
  Signal: new Set(['Signal']),
  Transaction: new Set(['Transaction']),
  Schema: new Set(['Schema', 'DataStream']),
  Statistics: new Set(['Statistics']),
  Config: new Set(['Config']),
};

// ---------------------------------------------------------------------------
// History snapshot
// ---------------------------------------------------------------------------

interface Snapshot {
  nodes: BlockNode[];
  edges: SchemaEdge[];
}

const MAX_HISTORY = 50;

// ---------------------------------------------------------------------------
// State interface
// ---------------------------------------------------------------------------

interface CanvasState {
  // State
  nodes: BlockNode[];
  edges: SchemaEdge[];
  selectedNodeId: string | null;
  designName: string;

  // History
  _past: Snapshot[];
  _future: Snapshot[];
  canUndo: boolean;
  canRedo: boolean;
  undo: () => void;
  redo: () => void;

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
  loadDesign: (name: string, nodes: BlockNode[], edges: SchemaEdge[]) => void;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useCanvasStore = create<CanvasState>((set, get) => {
  /** Push current nodes/edges onto the undo stack before a mutation. */
  function pushHistory() {
    const { nodes, edges, _past } = get();
    const snapshot: Snapshot = {
      nodes: JSON.parse(JSON.stringify(nodes)),
      edges: JSON.parse(JSON.stringify(edges)),
    };
    const past = [..._past, snapshot].slice(-MAX_HISTORY);
    return { _past: past, _future: [] as Snapshot[], canUndo: true, canRedo: false };
  }

  return {
    nodes: [],
    edges: [],
    selectedNodeId: null,
    designName: 'Untitled Design',

    _past: [],
    _future: [],
    canUndo: false,
    canRedo: false,

    // -----------------------------------------------------------------------
    // Undo / redo
    // -----------------------------------------------------------------------

    undo: () => {
      const { _past, _future, nodes, edges } = get();
      if (_past.length === 0) return;

      const prev = _past[_past.length - 1];
      const newPast = _past.slice(0, -1);
      const currentSnapshot: Snapshot = {
        nodes: JSON.parse(JSON.stringify(nodes)),
        edges: JSON.parse(JSON.stringify(edges)),
      };

      set({
        nodes: prev.nodes,
        edges: prev.edges,
        _past: newPast,
        _future: [..._future, currentSnapshot],
        canUndo: newPast.length > 0,
        canRedo: true,
      });
    },

    redo: () => {
      const { _past, _future, nodes, edges } = get();
      if (_future.length === 0) return;

      const next = _future[_future.length - 1];
      const newFuture = _future.slice(0, -1);
      const currentSnapshot: Snapshot = {
        nodes: JSON.parse(JSON.stringify(nodes)),
        edges: JSON.parse(JSON.stringify(edges)),
      };

      set({
        nodes: next.nodes,
        edges: next.edges,
        _past: [..._past, currentSnapshot],
        _future: newFuture,
        canUndo: true,
        canRedo: newFuture.length > 0,
      });
    },

    // -----------------------------------------------------------------------
    // Node actions
    // -----------------------------------------------------------------------

    onNodesChange: (changes) => {
      // Position changes (drag) are high-frequency — don't push history per tick.
      // We only push history for remove changes.
      const hasRemove = changes.some((c) => c.type === 'remove');
      const historyPatch = hasRemove ? pushHistory() : {};

      set({
        nodes: applyNodeChanges(changes, get().nodes) as BlockNode[],
        ...historyPatch,
      });
    },

    addNode: (node) => {
      set({
        nodes: [...get().nodes, node],
        ...pushHistory(),
      });
    },

    removeNode: (nodeId) => {
      const hist = pushHistory();
      set({
        nodes: get().nodes.filter((n) => n.id !== nodeId),
        edges: get().edges.filter(
          (e) => e.source !== nodeId && e.target !== nodeId,
        ),
        selectedNodeId:
          get().selectedNodeId === nodeId ? null : get().selectedNodeId,
        ...hist,
      });
    },

    updateNodeData: (nodeId, data) => {
      set({
        nodes: get().nodes.map((node) =>
          node.id === nodeId
            ? { ...node, data: { ...node.data, ...data } }
            : node,
        ),
        ...pushHistory(),
      });
    },

    // -----------------------------------------------------------------------
    // Edge actions
    // -----------------------------------------------------------------------

    onEdgesChange: (changes) => {
      const hasRemove = changes.some((c) => c.type === 'remove');
      const historyPatch = hasRemove ? pushHistory() : {};

      set({
        edges: applyEdgeChanges(changes, get().edges),
        ...historyPatch,
      });
    },

    onConnect: (connection) => {
      // Prevent connecting a node to itself
      if (connection.source === connection.target) {
        return;
      }

      // --- Port data-type validation ---
      const nodes = get().nodes;
      const sourceNode = nodes.find((n) => n.id === connection.source);
      const targetNode = nodes.find((n) => n.id === connection.target);

      if (sourceNode && targetNode && connection.sourceHandle && connection.targetHandle) {
        const srcPort = (sourceNode.data as BlockNodeData).outputs.find(
          (p) => p.name === connection.sourceHandle,
        );
        const tgtPort = (targetNode.data as BlockNodeData).inputs.find(
          (p) => p.name === connection.targetHandle,
        );

        if (srcPort && tgtPort) {
          const compat = PORT_COMPAT[srcPort.dataType];
          if (compat && !compat.has(tgtPort.dataType)) {
            toast.warning(
              `Type mismatch: ${srcPort.dataType} → ${tgtPort.dataType}. Connection not created.`,
            );
            return;
          }
        }
      }

      // Prevent duplicate connections to the same target port
      const existing = get().edges.find(
        (e) =>
          e.target === connection.target &&
          e.targetHandle === connection.targetHandle,
      );
      if (existing) {
        toast.info('That input port already has a connection.');
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
          get().edges,
        ),
        ...pushHistory(),
      });
    },

    removeEdge: (edgeId) => {
      set({
        edges: get().edges.filter((e) => e.id !== edgeId),
        ...pushHistory(),
      });
    },

    // -----------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------

    setSelectedNode: (nodeId) => {
      set({ selectedNodeId: nodeId });
    },

    // -----------------------------------------------------------------------
    // Design
    // -----------------------------------------------------------------------

    setDesignName: (name) => {
      set({ designName: name });
    },

    clearCanvas: () => {
      set({
        nodes: [],
        edges: [],
        selectedNodeId: null,
        _past: [],
        _future: [],
        canUndo: false,
        canRedo: false,
      });
    },

    loadDesign: (name, nodes, edges) => {
      set({
        designName: name,
        nodes,
        edges,
        selectedNodeId: null,
        _past: [],
        _future: [],
        canUndo: false,
        canRedo: false,
      });
    },
  };
});
