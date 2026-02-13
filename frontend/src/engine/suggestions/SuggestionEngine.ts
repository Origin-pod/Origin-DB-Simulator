import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData, PortDataType } from '@/types';
import type {
  ValidationResult,
  ValidationError,
  ValidationWarning,
} from '../types';
import type {
  CanvasActions,
  EnrichedValidationResult,
  EnrichedValidationItem,
  EnrichedSuggestion,
} from './types';
import { createNodeFromBlockType } from './nodeFactory';

// ---------------------------------------------------------------------------
// Port compatibility (mirrors canvasStore)
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
// SuggestionEngine
// ---------------------------------------------------------------------------

export class SuggestionEngine {
  constructor(
    private nodes: Node<BlockNodeData>[],
    private edges: Edge[],
    private actions: CanvasActions,
  ) {}

  /**
   * Enrich a validation result with actionable suggestions and auto-fix actions.
   */
  enrich(validation: ValidationResult): EnrichedValidationResult {
    const errors = validation.errors.map((e) => this.enrichItem(e));
    const warnings = validation.warnings.map((w) => this.enrichItem(w));
    const autoFixableCount =
      errors.filter((e) => e.enriched?.autoFix).length +
      warnings.filter((w) => w.enriched?.autoFix).length;

    return { valid: validation.valid, errors, warnings, autoFixableCount };
  }

  /**
   * Apply all auto-fixable suggestions at once.
   */
  applyAllFixes(result: EnrichedValidationResult): number {
    let applied = 0;
    for (const item of [...result.errors, ...result.warnings]) {
      if (item.enriched?.autoFix) {
        item.enriched.autoFix.apply();
        applied++;
      }
    }
    return applied;
  }

  // -------------------------------------------------------------------
  // Pattern matching on error messages
  // -------------------------------------------------------------------

  private enrichItem(
    item: ValidationError | ValidationWarning,
  ): EnrichedValidationItem {
    const msg = item.message;
    let enriched: EnrichedSuggestion | undefined;

    if (msg.includes('missing required input') || msg.includes('Required input port')) {
      enriched = this.suggestConnectPort(item);
    } else if (msg.includes('disconnected') || msg.includes('not connected to any other block')) {
      enriched = this.suggestRemoveBlock(item);
    } else if (msg.includes('cycle')) {
      enriched = this.suggestBreakCycle();
    } else if (msg.includes('empty') && msg.includes('block')) {
      enriched = this.suggestAddStorageBlock();
    } else if (msg.includes('storage block')) {
      enriched = this.suggestAddStorageBlock();
    } else if (msg.includes('small buffer') || msg.includes('Small buffer')) {
      enriched = this.suggestIncreaseBuffer(item);
    } else if (msg.includes('Duplicate connection') || msg.includes('duplicate')) {
      enriched = this.suggestRemoveDuplicate();
    } else if (msg.includes('Incompatible port') || msg.includes('type mismatch')) {
      enriched = this.suggestFixTypeMismatch(item);
    }

    // Fallback: pass through the original suggestion if no enrichment matched
    if (!enriched && item.suggestion) {
      enriched = { kind: 'general', message: item.suggestion };
    }

    return {
      nodeId: item.nodeId,
      message: item.message,
      suggestion: item.suggestion,
      enriched,
    };
  }

  // -------------------------------------------------------------------
  // Enrichment rules
  // -------------------------------------------------------------------

  /**
   * "Required input port 'X' on block 'Y' is not connected"
   * Find a compatible output on an existing block and offer to connect.
   */
  private suggestConnectPort(
    item: ValidationError | ValidationWarning,
  ): EnrichedSuggestion {
    const targetNodeId = item.nodeId;
    if (!targetNodeId) {
      return { kind: 'connect_port', message: 'Connect the missing input port.' };
    }

    const targetNode = this.nodes.find((n) => n.id === targetNodeId);
    if (!targetNode) {
      return { kind: 'connect_port', message: 'Connect the missing input port.' };
    }

    const targetData = targetNode.data as BlockNodeData;

    // Extract port name from message: "port 'X'" or "input \"X\""
    const portMatch = item.message.match(/(?:port|input)\s+['"]([^'"]+)['"]/);
    const portName = portMatch?.[1];

    if (!portName) {
      return { kind: 'connect_port', message: 'Connect the missing input port.' };
    }

    const inputPort = targetData.inputs.find((p) => p.name === portName);
    if (!inputPort) {
      return { kind: 'connect_port', message: `Connect the "${portName}" input.` };
    }

    // Find a compatible output port on another node
    const candidate = this.findCompatibleSource(targetNodeId, inputPort.dataType);

    if (candidate) {
      const sourceData = candidate.node.data as BlockNodeData;
      return {
        kind: 'connect_port',
        message: `Connect "${sourceData.label}" output "${candidate.portName}" to this input.`,
        autoFix: {
          label: `Connect to ${sourceData.label}`,
          description: `Create edge from ${sourceData.label}:${candidate.portName} to ${targetData.label}:${portName}`,
          apply: () => {
            this.actions.onConnect({
              source: candidate.node.id,
              target: targetNodeId,
              sourceHandle: candidate.portName,
              targetHandle: portName,
            });
          },
        },
      };
    }

    return {
      kind: 'connect_port',
      message: `Add a block with a compatible output and connect it to "${portName}".`,
    };
  }

  /**
   * "Block 'X' is disconnected"
   * Offer to remove the orphan block.
   */
  private suggestRemoveBlock(
    item: ValidationError | ValidationWarning,
  ): EnrichedSuggestion {
    const nodeId = item.nodeId;
    if (!nodeId) {
      return { kind: 'remove_block', message: 'Remove the disconnected block or connect it.' };
    }

    const node = this.nodes.find((n) => n.id === nodeId);
    const label = node ? (node.data as BlockNodeData).label : 'this block';

    return {
      kind: 'remove_block',
      message: `Remove "${label}" or connect it to the design.`,
      autoFix: {
        label: `Remove ${label}`,
        description: `Delete the disconnected "${label}" block from the canvas`,
        apply: () => {
          this.actions.removeNode(nodeId);
        },
      },
    };
  }

  /**
   * "Graph contains a cycle involving blocks: [a, b, c]"
   * Find the last edge in the cycle and offer to remove it.
   */
  private suggestBreakCycle(): EnrichedSuggestion {
    // The last edge added is most likely the one causing the cycle
    if (this.edges.length > 0) {
      const lastEdge = this.edges[this.edges.length - 1];
      return {
        kind: 'break_cycle',
        message: `Remove the most recently added connection to break the cycle.`,
        autoFix: {
          label: 'Remove last edge',
          description: `Remove the connection "${lastEdge.id}" to break the cycle`,
          apply: () => {
            this.actions.removeEdge(lastEdge.id);
          },
        },
      };
    }

    return { kind: 'break_cycle', message: 'Remove a connection to break the cycle.' };
  }

  /**
   * "Canvas is empty" or "No storage block"
   * Add a Heap Storage block with defaults.
   */
  private suggestAddStorageBlock(): EnrichedSuggestion {
    const position = this.computeNewNodePosition();

    return {
      kind: 'add_block',
      message: 'Add a Heap Storage block to start your design.',
      autoFix: {
        label: 'Add Heap Storage',
        description: 'Add a Heap File Storage block with default parameters',
        apply: () => {
          const node = createNodeFromBlockType('heap_storage', position);
          if (node) {
            this.actions.addNode(node);
          }
        },
      },
    };
  }

  /**
   * "Small buffer size (X MB)"
   * Suggest increasing to at least 128 MB.
   */
  private suggestIncreaseBuffer(
    item: ValidationError | ValidationWarning,
  ): EnrichedSuggestion {
    const nodeId = item.nodeId;
    if (!nodeId) {
      return { kind: 'update_parameter', message: 'Increase buffer size to at least 128 MB.' };
    }

    const node = this.nodes.find((n) => n.id === nodeId);
    if (!node) {
      return { kind: 'update_parameter', message: 'Increase buffer size to at least 128 MB.' };
    }

    const data = node.data as BlockNodeData;
    const currentSize = Number(data.parameters.size ?? 0);
    const suggestedSize = Math.max(128, currentSize * 2);

    return {
      kind: 'update_parameter',
      message: `Increase buffer size from ${currentSize} MB to ${suggestedSize} MB.`,
      autoFix: {
        label: `Set to ${suggestedSize} MB`,
        description: `Increase buffer size to ${suggestedSize} MB for better cache performance`,
        apply: () => {
          this.actions.updateNodeData(nodeId, {
            parameters: { ...data.parameters, size: suggestedSize },
          });
        },
      },
    };
  }

  /**
   * "Duplicate connection"
   * Remove the duplicate edge.
   */
  private suggestRemoveDuplicate(): EnrichedSuggestion {
    // Find duplicate edges
    const seen = new Set<string>();
    for (const edge of this.edges) {
      const key = `${edge.source}:${edge.sourceHandle}-${edge.target}:${edge.targetHandle}`;
      if (seen.has(key)) {
        return {
          kind: 'remove_duplicate',
          message: 'Remove the duplicate connection.',
          autoFix: {
            label: 'Remove duplicate',
            description: `Remove duplicate edge "${edge.id}"`,
            apply: () => {
              this.actions.removeEdge(edge.id);
            },
          },
        };
      }
      seen.add(key);
    }

    return { kind: 'remove_duplicate', message: 'Remove the duplicate connection.' };
  }

  /**
   * "Incompatible port types"
   * Explain what types are compatible.
   */
  private suggestFixTypeMismatch(
    item: ValidationError | ValidationWarning,
  ): EnrichedSuggestion {
    return {
      kind: 'type_mismatch',
      message: item.suggestion ?? 'Connected ports must have compatible data types (DataStream and Batch are interchangeable).',
    };
  }

  // -------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------

  /**
   * Find a node with a compatible output port for the given data type.
   */
  private findCompatibleSource(
    excludeNodeId: string,
    targetDataType: PortDataType,
  ): { node: Node<BlockNodeData>; portName: string } | null {
    const compatibleTypes = PORT_COMPAT[targetDataType];
    if (!compatibleTypes) return null;

    for (const node of this.nodes) {
      if (node.id === excludeNodeId) continue;
      const data = node.data as BlockNodeData;
      for (const output of data.outputs) {
        if (compatibleTypes.has(output.dataType as PortDataType)) {
          // Check this output isn't already connected to the target
          const alreadyConnected = this.edges.some(
            (e) =>
              e.source === node.id &&
              e.sourceHandle === output.name &&
              e.target === excludeNodeId,
          );
          if (!alreadyConnected) {
            return { node, portName: output.name };
          }
        }
      }
    }
    return null;
  }

  /**
   * Compute a position for a new node (offset from existing nodes).
   */
  private computeNewNodePosition(): { x: number; y: number } {
    if (this.nodes.length === 0) {
      return { x: 200, y: 200 };
    }

    let maxX = -Infinity;
    let avgY = 0;
    for (const node of this.nodes) {
      if (node.position.x > maxX) maxX = node.position.x;
      avgY += node.position.y;
    }
    avgY /= this.nodes.length;

    return { x: maxX + 250, y: avgY };
  }
}
