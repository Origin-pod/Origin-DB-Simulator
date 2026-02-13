import { X, Library, Box } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useArchitectureStore } from '@/stores/architectureStore';
import {
  getArchitectures,
  ARCHITECTURE_CATEGORY_LABELS,
  type ArchitectureCategory,
} from '@/data/architectures';

export function ArchitectureModal() {
  const { modalOpen, closeModal, loadArchitecture } = useArchitectureStore();

  if (!modalOpen) return null;

  const architectures = getArchitectures();

  // Group by category
  const grouped = architectures.reduce(
    (acc, arch) => {
      if (!acc[arch.category]) acc[arch.category] = [];
      acc[arch.category].push(arch);
      return acc;
    },
    {} as Record<ArchitectureCategory, typeof architectures>,
  );

  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/30 z-50" onClick={closeModal} />

      {/* Modal */}
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="bg-white rounded-xl shadow-xl w-full max-w-3xl max-h-[85vh] flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
            <div className="flex items-center gap-2">
              <Library className="w-5 h-5 text-indigo-600" />
              <h2 className="text-lg font-semibold text-gray-900">
                DB Architectures
              </h2>
            </div>
            <button
              onClick={closeModal}
              className="text-gray-400 hover:text-gray-600"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <p className="px-6 py-3 text-sm text-gray-600 border-b border-gray-100">
            Study the internal architecture of famous databases. Each design
            shows how real databases compose storage, indexing, buffering, and
            concurrency components — with annotations explaining <em>why</em>.
          </p>

          {/* Architecture list */}
          <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
            {(Object.entries(grouped) as [ArchitectureCategory, typeof architectures][]).map(
              ([category, archs]) => (
                <div key={category}>
                  <h3 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">
                    {ARCHITECTURE_CATEGORY_LABELS[category]}
                  </h3>
                  <div className="space-y-3">
                    {archs.map((arch) => (
                      <div
                        key={arch.id}
                        className="border border-gray-200 rounded-xl p-4 hover:border-indigo-300 hover:bg-indigo-50/30 transition-colors"
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex-1">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="text-xl">{arch.logo}</span>
                              <h4 className="text-sm font-semibold text-gray-900">
                                {arch.name}
                              </h4>
                            </div>
                            <p className="text-xs text-gray-500 mb-1.5">
                              {arch.subtitle}
                            </p>
                            <p className="text-xs text-gray-600 mb-2">
                              {arch.description}
                            </p>
                            <div className="flex items-center gap-3 mb-2">
                              <span className="flex items-center gap-1 text-[10px] text-gray-400">
                                <Box className="w-3 h-3" />
                                {arch.nodes.length} blocks
                              </span>
                              <span className="text-[10px] text-gray-400">
                                {arch.annotations.length} annotations
                              </span>
                            </div>
                            <div className="flex flex-wrap gap-1">
                              {arch.concepts.map((c) => (
                                <span
                                  key={c}
                                  className="text-[10px] bg-indigo-50 text-indigo-600 px-1.5 py-0.5 rounded"
                                >
                                  {c}
                                </span>
                              ))}
                            </div>
                          </div>
                          <Button
                            variant="primary"
                            size="sm"
                            onClick={() => loadArchitecture(arch.id)}
                            className="flex-shrink-0"
                          >
                            Study
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ),
            )}
          </div>

          {/* Footer */}
          <div className="px-6 py-3 border-t border-gray-200 bg-gray-50 rounded-b-xl flex items-center justify-between">
            <span className="text-xs text-gray-400">
              {architectures.length} database architectures
            </span>
            <Button variant="ghost" size="sm" onClick={closeModal}>
              Close
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}
