import { X, Lightbulb, BookOpen, ChevronDown, ChevronRight } from 'lucide-react';
import { useState } from 'react';
import { useArchitectureStore } from '@/stores/architectureStore';

export function ArchitectureAnnotations() {
  const { activeArchitecture, clearArchitecture } = useArchitectureStore();
  const [expandedBlocks, setExpandedBlocks] = useState<Set<string>>(new Set());

  if (!activeArchitecture) return null;

  const toggleBlock = (blockType: string) => {
    setExpandedBlocks((prev) => {
      const next = new Set(prev);
      if (next.has(blockType)) {
        next.delete(blockType);
      } else {
        next.add(blockType);
      }
      return next;
    });
  };

  return (
    <div className="fixed right-0 top-14 bottom-0 w-80 z-20 bg-white border-l border-gray-200 flex flex-col shadow-lg">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200">
        <div className="flex items-center gap-2">
          <span className="text-lg">{activeArchitecture.logo}</span>
          <div>
            <h3 className="text-sm font-semibold text-gray-900">
              {activeArchitecture.name}
            </h3>
            <p className="text-[10px] text-gray-500">
              {activeArchitecture.subtitle}
            </p>
          </div>
        </div>
        <button
          onClick={clearArchitecture}
          className="text-gray-400 hover:text-gray-600 p-1"
          title="Close annotations"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto">
        {/* Key Insight */}
        <div className="px-4 py-3 bg-indigo-50 border-b border-indigo-100">
          <div className="flex items-start gap-2">
            <Lightbulb className="w-4 h-4 text-indigo-600 mt-0.5 flex-shrink-0" />
            <div>
              <p className="text-[10px] font-semibold text-indigo-700 uppercase tracking-wider mb-0.5">
                Key Insight
              </p>
              <p className="text-xs text-indigo-800 leading-relaxed">
                {activeArchitecture.keyInsight}
              </p>
            </div>
          </div>
        </div>

        {/* Why This Architecture */}
        <div className="px-4 py-3 border-b border-gray-100">
          <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-1">
            Why This Architecture
          </p>
          <p className="text-xs text-gray-700 leading-relaxed">
            {activeArchitecture.whyThisArchitecture}
          </p>
        </div>

        {/* Per-block annotations */}
        <div className="px-4 py-3">
          <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-2">
            Component Annotations
          </p>
          <div className="space-y-1.5">
            {activeArchitecture.annotations.map((ann) => {
              const isExpanded = expandedBlocks.has(ann.blockType);
              return (
                <div
                  key={ann.blockType}
                  className="border border-gray-100 rounded-lg overflow-hidden"
                >
                  <button
                    className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-gray-50 transition-colors"
                    onClick={() => toggleBlock(ann.blockType)}
                  >
                    {isExpanded ? (
                      <ChevronDown className="w-3 h-3 text-gray-400 flex-shrink-0" />
                    ) : (
                      <ChevronRight className="w-3 h-3 text-gray-400 flex-shrink-0" />
                    )}
                    <span className="text-xs font-medium text-gray-900">
                      {ann.title}
                    </span>
                  </button>
                  {isExpanded && (
                    <div className="px-3 pb-2.5 pt-0 space-y-1.5">
                      <p className="text-xs text-gray-600 leading-relaxed pl-5">
                        {ann.explanation}
                      </p>
                      {ann.realWorldDetail && (
                        <div className="pl-5 mt-1.5">
                          <p className="text-[10px] text-blue-700 bg-blue-50 px-2 py-1.5 rounded leading-relaxed">
                            <span className="font-semibold">In production: </span>
                            {ann.realWorldDetail}
                          </p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Concepts */}
        <div className="px-4 py-3 border-t border-gray-100">
          <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-2">
            Concepts Covered
          </p>
          <div className="flex flex-wrap gap-1">
            {activeArchitecture.concepts.map((c) => (
              <span
                key={c}
                className="text-[10px] bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded"
              >
                {c}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="px-4 py-2 border-t border-gray-200 bg-gray-50 flex items-center gap-2">
        <BookOpen className="w-3.5 h-3.5 text-gray-400" />
        <span className="text-[10px] text-gray-500">
          Run the workload to see how this architecture performs
        </span>
      </div>
    </div>
  );
}
