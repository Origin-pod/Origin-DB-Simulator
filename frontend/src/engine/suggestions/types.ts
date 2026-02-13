import type { Node } from '@xyflow/react';
import type { BlockNodeData } from '@/types';

// What kind of suggestion is this?
export type SuggestionKind =
  | 'connect_port'
  | 'connect_block'
  | 'break_cycle'
  | 'add_block'
  | 'remove_duplicate'
  | 'update_parameter'
  | 'remove_block'
  | 'type_mismatch'
  | 'general';

// A single auto-fix action that can be applied to the canvas
export interface AutoFixAction {
  label: string;
  description: string;
  apply: () => void;
}

// An enriched suggestion attached to a validation error/warning
export interface EnrichedSuggestion {
  kind: SuggestionKind;
  message: string;
  autoFix?: AutoFixAction;
}

// A validation item enriched with suggestion engine output
export interface EnrichedValidationItem {
  nodeId?: string;
  message: string;
  suggestion?: string;
  enriched?: EnrichedSuggestion;
}

// The full enriched result
export interface EnrichedValidationResult {
  valid: boolean;
  errors: EnrichedValidationItem[];
  warnings: EnrichedValidationItem[];
  autoFixableCount: number;
}

// Canvas actions interface — injected dependency for testability
export interface CanvasActions {
  addNode: (node: Node<BlockNodeData>) => void;
  onConnect: (connection: {
    source: string;
    target: string;
    sourceHandle: string;
    targetHandle: string;
  }) => void;
  removeEdge: (edgeId: string) => void;
  removeNode: (nodeId: string) => void;
  updateNodeData: (
    nodeId: string,
    data: Partial<BlockNodeData>,
  ) => void;
}
