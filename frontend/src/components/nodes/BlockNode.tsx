import { memo } from 'react';
import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';
import {
  Database,
  Binary,
  HardDrive,
  Layers,
  Cpu,
  Settings,
  Lock,
  GitBranch,
  Archive,
  Grid3x3,
  Sparkles,
  Network,
  Hash,
  FileStack,
  Clock,
  Filter,
  ArrowUpDown,
  Merge,
  FileText,
  Search,
} from 'lucide-react';
import type { BlockNodeData, PortDefinition } from '@/types';
import { CATEGORY_COLORS, DATA_TYPE_COLORS } from '@/types';

type BlockNodeType = Node<BlockNodeData>;

// Icon mapping for all block types
const ICONS: Record<string, React.ReactNode> = {
  Database: <Database className="w-4 h-4" />,
  Binary: <Binary className="w-4 h-4" />,
  HardDrive: <HardDrive className="w-4 h-4" />,
  Layers: <Layers className="w-4 h-4" />,
  Cpu: <Cpu className="w-4 h-4" />,
  Settings: <Settings className="w-4 h-4" />,
  Lock: <Lock className="w-4 h-4" />,
  GitBranch: <GitBranch className="w-4 h-4" />,
  Archive: <Archive className="w-4 h-4" />,
  Grid3x3: <Grid3x3 className="w-4 h-4" />,
  Sparkles: <Sparkles className="w-4 h-4" />,
  Network: <Network className="w-4 h-4" />,
  Hash: <Hash className="w-4 h-4" />,
  FileStack: <FileStack className="w-4 h-4" />,
  Clock: <Clock className="w-4 h-4" />,
  Filter: <Filter className="w-4 h-4" />,
  ArrowUpDown: <ArrowUpDown className="w-4 h-4" />,
  Merge: <Merge className="w-4 h-4" />,
  FileText: <FileText className="w-4 h-4" />,
  Search: <Search className="w-4 h-4" />,
};

function BlockNodeComponent({ data, selected }: NodeProps<BlockNodeType>) {
  const nodeData = data as BlockNodeData;
  const categoryColor = CATEGORY_COLORS[nodeData.category];
  const icon = ICONS[nodeData.icon] || <Settings className="w-4 h-4" />;

  // Calculate handle positions based on port count
  const inputCount = nodeData.inputs.length;
  const outputCount = nodeData.outputs.length;
  const maxPorts = Math.max(inputCount, outputCount, 1);
  const portHeight = 24; // Height per port row
  const bodyMinHeight = maxPorts * portHeight + 8; // Add padding

  return (
    <div
      className={`
        bg-white rounded-lg shadow-sm border-2 min-w-[200px]
        transition-all duration-150
        ${selected ? 'shadow-md ring-2 ring-primary-500 ring-offset-2' : 'hover:shadow-md'}
        ${nodeData.state === 'running' ? 'animate-pulse' : ''}
        ${nodeData.state === 'error' ? 'border-error' : ''}
      `}
      style={{
        borderColor: nodeData.state === 'error' ? undefined : categoryColor,
      }}
    >
      {/* Header */}
      <div
        className="px-3 py-2 rounded-t-md flex items-center gap-2"
        style={{ backgroundColor: `${categoryColor}15` }}
      >
        <span style={{ color: categoryColor }}>{icon}</span>
        <span className="text-sm font-medium text-gray-900 flex-1 truncate">
          {nodeData.label}
        </span>
      </div>

      {/* Body with ports */}
      <div
        className="relative px-3 py-2"
        style={{ minHeight: bodyMinHeight }}
      >
        {/* Input ports - left side */}
        {nodeData.inputs.map((port: PortDefinition, index: number) => {
          const topOffset = 8 + index * portHeight + portHeight / 2;
          return (
            <div
              key={`input-${port.name}`}
              className="flex items-center gap-2"
              style={{
                position: 'absolute',
                left: 12,
                top: topOffset - 8,
                height: portHeight,
              }}
            >
              <Handle
                type="target"
                position={Position.Left}
                id={port.name}
                style={{
                  backgroundColor: DATA_TYPE_COLORS[port.dataType],
                  width: 10,
                  height: 10,
                  left: -17,
                  top: '50%',
                  transform: 'translateY(-50%)',
                  border: '2px solid white',
                  boxShadow: '0 1px 2px rgba(0,0,0,0.1)',
                }}
              />
              <span className="text-xs text-gray-500">{port.name}</span>
            </div>
          );
        })}

        {/* Output ports - right side */}
        {nodeData.outputs.map((port: PortDefinition, index: number) => {
          const topOffset = 8 + index * portHeight + portHeight / 2;
          return (
            <div
              key={`output-${port.name}`}
              className="flex items-center justify-end gap-2"
              style={{
                position: 'absolute',
                right: 12,
                top: topOffset - 8,
                height: portHeight,
              }}
            >
              <span className="text-xs text-gray-500">{port.name}</span>
              <Handle
                type="source"
                position={Position.Right}
                id={port.name}
                style={{
                  backgroundColor: DATA_TYPE_COLORS[port.dataType],
                  width: 10,
                  height: 10,
                  right: -17,
                  top: '50%',
                  transform: 'translateY(-50%)',
                  border: '2px solid white',
                  boxShadow: '0 1px 2px rgba(0,0,0,0.1)',
                }}
              />
            </div>
          );
        })}

        {/* Show message if no ports */}
        {inputCount === 0 && outputCount === 0 && (
          <p className="text-xs text-gray-400 text-center py-2">No ports</p>
        )}
      </div>
    </div>
  );
}

export const BlockNode = memo(BlockNodeComponent);
