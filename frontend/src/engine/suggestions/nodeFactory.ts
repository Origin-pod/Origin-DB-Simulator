import type { Node } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import { CATEGORY_COLORS } from '@/types';
import { getBlockDefinition } from '@/types/blocks';

/**
 * Create a canvas node from a block type with default parameters.
 * Used by the SuggestionEngine for auto-fix "add block" actions.
 */
export function createNodeFromBlockType(
  blockType: string,
  position: { x: number; y: number },
  paramOverrides?: Record<string, string | number | boolean>,
): Node<BlockNodeData> | null {
  const def = getBlockDefinition(blockType);
  if (!def) return null;

  const parameters: Record<string, string | number | boolean> = {};
  for (const p of def.parameters) {
    parameters[p.name] = p.default;
  }
  if (paramOverrides) {
    Object.assign(parameters, paramOverrides);
  }

  return {
    id: `${blockType}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    type: 'blockNode',
    position,
    data: {
      blockType,
      label: def.name,
      category: def.category,
      icon: def.icon,
      color: CATEGORY_COLORS[def.category],
      inputs: def.inputs,
      outputs: def.outputs,
      parameters,
      state: 'idle',
    },
  };
}
