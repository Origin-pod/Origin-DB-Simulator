import type { Node, Edge } from '@xyflow/react';

// Re-export block types
export * from './blocks';

// Block Categories - matches Rust block-system
export type BlockCategory =
  | 'storage'
  | 'index'
  | 'buffer'
  | 'concurrency'
  | 'execution'
  | 'transaction'
  | 'compression'
  | 'partitioning'
  | 'optimization'
  | 'distribution';

// Port data types for connections
export type PortDataType =
  | 'DataStream'
  | 'SingleValue'
  | 'Batch'
  | 'Signal'
  | 'Transaction'
  | 'Schema'
  | 'Statistics'
  | 'Config';

// Port definition for block inputs/outputs
export interface PortDefinition {
  name: string;
  type: 'input' | 'output';
  dataType: PortDataType;
  description: string;
  required: boolean;
}

// Parameter types for block configuration
export type ParameterType = 'string' | 'number' | 'boolean' | 'enum';

export interface ParameterConstraints {
  min?: number;
  max?: number;
  step?: number;
  pattern?: string;
  options?: string[];
}

export interface ParameterDefinition {
  name: string;
  type: ParameterType;
  default: string | number | boolean;
  description: string;
  constraints?: ParameterConstraints;
  uiHint?: 'input' | 'slider' | 'select' | 'checkbox';
}

// Static block type definition
export interface BlockDefinition {
  type: string;
  name: string;
  description: string;
  category: BlockCategory;
  icon: string;
  color: string;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  parameters: ParameterDefinition[];
}

// Block instance on canvas
export interface BlockInstance {
  id: string;
  type: string;
  position: { x: number; y: number };
  parameters: Record<string, string | number | boolean>;
  state: 'idle' | 'running' | 'error' | 'complete';
}

// Block node data type for React Flow
export interface BlockNodeData extends Record<string, unknown> {
  blockType: string;
  label: string;
  category: BlockCategory;
  icon: string;
  color: string;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  parameters: Record<string, string | number | boolean>;
  state: 'idle' | 'running' | 'error' | 'complete';
}

// Typed node and edge for React Flow
export type BlockNode = Node<BlockNodeData>;
export type SchemaEdge = Edge;

// Design represents a complete database schema design
export interface Design {
  id: string;
  name: string;
  nodes: BlockNode[];
  edges: SchemaEdge[];
  createdAt: Date;
  updatedAt: Date;
}

// Category colors mapping
export const CATEGORY_COLORS: Record<BlockCategory, string> = {
  storage: '#8B5CF6',
  index: '#3B82F6',
  buffer: '#14B8A6',
  concurrency: '#F59E0B',
  execution: '#EC4899',
  transaction: '#6366F1',
  compression: '#84CC16',
  partitioning: '#F97316',
  optimization: '#06B6D4',
  distribution: '#A855F7',
};

// Data type colors for ports
export const DATA_TYPE_COLORS: Record<PortDataType, string> = {
  DataStream: '#3B82F6',
  SingleValue: '#10B981',
  Batch: '#8B5CF6',
  Signal: '#F59E0B',
  Transaction: '#6366F1',
  Schema: '#EC4899',
  Statistics: '#06B6D4',
  Config: '#6B7280',
};
