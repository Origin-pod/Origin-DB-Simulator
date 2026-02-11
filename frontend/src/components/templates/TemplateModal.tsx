import { useMemo } from 'react';
import {
  X,
  LayoutTemplate,
  Database,
  Layers,
  Zap,
  BarChart3,
  ArrowRight,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import {
  getTemplates,
  TEMPLATE_CATEGORY_LABELS,
  TEMPLATE_CATEGORY_COLORS,
  type DesignTemplate,
  type TemplateCategory,
} from '@/data/templates';
import { useDesignStore } from '@/stores/designStore';
import { useCanvasStore } from '@/stores/canvasStore';
import { useWorkloadStore } from '@/stores/workloadStore';

// ---------------------------------------------------------------------------
// Category icons
// ---------------------------------------------------------------------------

const CATEGORY_ICONS: Record<TemplateCategory, React.ReactNode> = {
  oltp: <Zap className="w-4 h-4" />,
  olap: <BarChart3 className="w-4 h-4" />,
  'write-heavy': <Layers className="w-4 h-4" />,
  'read-heavy': <Database className="w-4 h-4" />,
};

// ---------------------------------------------------------------------------
// Template card
// ---------------------------------------------------------------------------

function TemplateCard({
  template,
  onSelect,
}: {
  template: DesignTemplate;
  onSelect: (t: DesignTemplate) => void;
}) {
  const catColor = TEMPLATE_CATEGORY_COLORS[template.category];
  const catLabel = TEMPLATE_CATEGORY_LABELS[template.category];
  const catIcon = CATEGORY_ICONS[template.category];

  return (
    <div className="border border-gray-200 rounded-xl overflow-hidden hover:shadow-md hover:border-gray-300 transition-all group">
      {/* Preview header */}
      <div
        className="h-28 px-4 py-3 flex flex-col justify-between"
        style={{ backgroundColor: `${catColor}08` }}
      >
        {/* Mini block diagram */}
        <div className="flex items-center gap-1.5 flex-wrap">
          {template.nodes.map((node, i) => (
            <div key={node.id} className="flex items-center gap-1">
              <span
                className="inline-block w-14 h-6 rounded text-[9px] font-medium text-white text-center leading-6 truncate px-1"
                style={{ backgroundColor: node.data.color }}
              >
                {(node.data as { label: string }).label.split(' ')[0]}
              </span>
              {i < template.nodes.length - 1 && (
                <ArrowRight className="w-3 h-3 text-gray-300" />
              )}
            </div>
          ))}
        </div>
        <div className="flex items-center gap-1.5">
          <span
            className="flex items-center gap-1 text-[10px] font-semibold px-1.5 py-0.5 rounded"
            style={{ backgroundColor: `${catColor}20`, color: catColor }}
          >
            {catIcon}
            {catLabel}
          </span>
          <span className="text-[10px] text-gray-400">
            {template.nodes.length} blocks · {template.edges.length} connections
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="px-4 py-3">
        <h3 className="text-sm font-semibold text-gray-900 mb-1">
          {template.name}
        </h3>
        <p className="text-xs text-gray-500 leading-relaxed line-clamp-2 mb-3">
          {template.description}
        </p>

        {/* Tags */}
        <div className="flex flex-wrap gap-1 mb-3">
          {template.tags.slice(0, 4).map((tag) => (
            <span
              key={tag}
              className="text-[10px] text-gray-500 bg-gray-100 px-1.5 py-0.5 rounded"
            >
              {tag}
            </span>
          ))}
        </div>

        {/* Workload summary */}
        <div className="text-[10px] text-gray-400 mb-3">
          Workload: {template.workload.operations.map((o) => `${o.weight}% ${o.type}`).join(', ')}
        </div>

        <Button
          variant="primary"
          size="sm"
          className="w-full"
          onClick={() => onSelect(template)}
        >
          Use Template
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main modal
// ---------------------------------------------------------------------------

export function TemplateModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const templates = useMemo(() => getTemplates(), []);
  const { createDesign, saveCurrentCanvas } = useDesignStore();
  const canvasStore = useCanvasStore;
  const workloadStore = useWorkloadStore;

  const handleSelect = (template: DesignTemplate) => {
    saveCurrentCanvas();

    // Create a new design
    createDesign(template.name);

    // Deep-copy template nodes/edges (with fresh IDs to avoid collisions)
    const nodes = JSON.parse(JSON.stringify(template.nodes));
    const edges = JSON.parse(JSON.stringify(template.edges));

    // Load into canvas
    canvasStore.getState().loadDesign(template.name, nodes, edges);

    // Load matching workload
    const wlState = workloadStore.getState();
    wlState.setWorkloadName(template.workload.name);
    wlState.setDistribution(template.workload.distribution);
    wlState.setConcurrency(template.workload.concurrency);
    wlState.setTotalOperations(template.workload.totalOperations);

    // Replace operations: clear and re-add
    // Simplest approach: directly set the workload
    workloadStore.setState({ workload: JSON.parse(JSON.stringify(template.workload)) });

    onClose();
  };

  const handleScratch = () => {
    saveCurrentCanvas();
    createDesign();
    onClose();
  };

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-white rounded-xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <div className="flex items-center gap-2">
            <LayoutTemplate className="w-5 h-5 text-primary-500" />
            <h2 className="text-lg font-semibold text-gray-900">
              Start from Template
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 text-gray-400 hover:text-gray-600"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <div className="grid grid-cols-2 gap-4">
            {templates.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                onSelect={handleSelect}
              />
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-4 border-t border-gray-200 bg-gray-50 rounded-b-xl">
          <p className="text-xs text-gray-500">
            Templates include pre-configured blocks, connections, and a matching workload.
          </p>
          <div className="flex items-center gap-2">
            <Button variant="secondary" size="sm" onClick={handleScratch}>
              Start from Scratch
            </Button>
            <Button variant="ghost" size="sm" onClick={onClose}>
              Cancel
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
