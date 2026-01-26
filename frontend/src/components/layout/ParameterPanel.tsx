import { X, Settings, Trash2, Info, HelpCircle } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useCanvasStore } from '@/stores/canvasStore';
import { CATEGORY_COLORS, DATA_TYPE_COLORS, type BlockNodeData, type PortDefinition } from '@/types';
import { getBlockDefinition } from '@/types/blocks';

export function ParameterPanel() {
  const { nodes, selectedNodeId, setSelectedNode, removeNode } = useCanvasStore();

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);

  if (!selectedNode) {
    return (
      <aside className="w-72 bg-white border-l border-gray-200 flex flex-col">
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center px-6">
            <Settings className="w-12 h-12 text-gray-300 mx-auto mb-3" />
            <p className="text-sm text-gray-500">
              Select a block on the canvas to view its configuration
            </p>
          </div>
        </div>
      </aside>
    );
  }

  const data = selectedNode.data as BlockNodeData;
  const categoryColor = CATEGORY_COLORS[data.category];
  const blockDef = getBlockDefinition(data.blockType);

  const handleDelete = () => {
    removeNode(selectedNode.id);
  };

  return (
    <aside className="w-72 bg-white border-l border-gray-200 flex flex-col">
      {/* Header */}
      <div
        className="p-4 border-b"
        style={{ borderBottomColor: categoryColor, borderBottomWidth: 3 }}
      >
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold text-gray-900">{data.label}</h3>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDelete}
              className="text-gray-400 hover:text-error"
              title="Delete block"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSelectedNode(null)}
              className="text-gray-400 hover:text-gray-600"
              title="Close panel"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span
            className="text-xs font-medium px-2 py-0.5 rounded"
            style={{ backgroundColor: `${categoryColor}20`, color: categoryColor }}
          >
            {data.category.charAt(0).toUpperCase() + data.category.slice(1)}
          </span>
        </div>
        {blockDef?.description && (
          <p className="text-xs text-gray-500 mt-2">{blockDef.description}</p>
        )}
      </div>

      {/* Configuration */}
      <div className="flex-1 overflow-y-auto">
        {/* Parameters Section */}
        {blockDef && blockDef.parameters.length > 0 && (
          <div className="p-4 border-b border-gray-100">
            <h4 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
              <Settings className="w-4 h-4 text-gray-400" />
              Configuration
            </h4>
            <div className="space-y-3">
              {blockDef.parameters.map((param) => {
                const value = data.parameters[param.name] ?? param.default;
                return (
                  <div key={param.name} className="space-y-1">
                    <div className="flex items-center justify-between">
                      <label className="text-xs font-medium text-gray-700">
                        {param.name}
                      </label>
                      <button
                        className="text-gray-400 hover:text-gray-600"
                        title={param.description}
                      >
                        <HelpCircle className="w-3 h-3" />
                      </button>
                    </div>
                    {param.type === 'boolean' ? (
                      <div className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={Boolean(value)}
                          readOnly
                          className="w-4 h-4 rounded border-gray-300 text-primary-500 focus:ring-primary-500"
                        />
                        <span className="text-xs text-gray-500">
                          {value ? 'Enabled' : 'Disabled'}
                        </span>
                      </div>
                    ) : param.type === 'enum' && param.constraints?.options ? (
                      <select
                        value={String(value)}
                        disabled
                        className="w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-gray-50 text-gray-700"
                      >
                        {param.constraints.options.map((opt) => (
                          <option key={opt} value={opt}>
                            {opt}
                          </option>
                        ))}
                      </select>
                    ) : param.uiHint === 'slider' ? (
                      <div className="space-y-1">
                        <input
                          type="range"
                          value={Number(value)}
                          min={param.constraints?.min}
                          max={param.constraints?.max}
                          step={param.constraints?.step}
                          disabled
                          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                        />
                        <div className="flex justify-between text-xs text-gray-500">
                          <span>{param.constraints?.min}</span>
                          <span className="font-medium text-gray-700">{value}</span>
                          <span>{param.constraints?.max}</span>
                        </div>
                      </div>
                    ) : (
                      <input
                        type={param.type === 'number' ? 'number' : 'text'}
                        value={String(value)}
                        readOnly
                        className="w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-gray-50 text-gray-700"
                      />
                    )}
                  </div>
                );
              })}
            </div>
            <p className="text-xs text-gray-400 mt-3 italic">
              Editing will be enabled in Milestone 4
            </p>
          </div>
        )}

        {/* Ports Section */}
        <div className="p-4">
          <h4 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
            <Info className="w-4 h-4 text-gray-400" />
            Ports
          </h4>

          {data.inputs.length > 0 && (
            <div className="mb-4">
              <p className="text-xs font-medium text-gray-500 mb-2">Inputs</p>
              <div className="space-y-2">
                {data.inputs.map((port: PortDefinition) => (
                  <div
                    key={port.name}
                    className="flex items-center gap-2 p-2 bg-gray-50 rounded-lg"
                  >
                    <span
                      className="w-3 h-3 rounded-full flex-shrink-0"
                      style={{ backgroundColor: DATA_TYPE_COLORS[port.dataType] }}
                    />
                    <div className="flex-1 min-w-0">
                      <span className="text-sm text-gray-700 block truncate">
                        {port.name}
                      </span>
                      <span className="text-xs text-gray-400">
                        {port.dataType}
                        {port.required && (
                          <span className="text-error ml-1">*</span>
                        )}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {data.outputs.length > 0 && (
            <div>
              <p className="text-xs font-medium text-gray-500 mb-2">Outputs</p>
              <div className="space-y-2">
                {data.outputs.map((port: PortDefinition) => (
                  <div
                    key={port.name}
                    className="flex items-center gap-2 p-2 bg-gray-50 rounded-lg"
                  >
                    <span
                      className="w-3 h-3 rounded-full flex-shrink-0"
                      style={{ backgroundColor: DATA_TYPE_COLORS[port.dataType] }}
                    />
                    <div className="flex-1 min-w-0">
                      <span className="text-sm text-gray-700 block truncate">
                        {port.name}
                      </span>
                      <span className="text-xs text-gray-400">{port.dataType}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {data.inputs.length === 0 && data.outputs.length === 0 && (
            <p className="text-sm text-gray-500 text-center py-4">
              This block has no ports
            </p>
          )}
        </div>

        {/* Documentation Section */}
        {blockDef?.documentation && (
          <div className="p-4 border-t border-gray-100">
            <h4 className="text-sm font-medium text-gray-900 mb-2">Documentation</h4>
            <p className="text-xs text-gray-600 leading-relaxed">
              {blockDef.documentation.details || blockDef.documentation.summary}
            </p>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-gray-200 bg-gray-50">
        <p className="text-xs text-gray-400 font-mono">
          ID: {selectedNode.id}
        </p>
      </div>
    </aside>
  );
}
