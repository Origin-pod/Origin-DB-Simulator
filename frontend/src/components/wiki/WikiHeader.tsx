import { ArrowLeft, X } from 'lucide-react';
import { useWikiStore } from '@/stores/wikiStore';
import { CATEGORIES } from '@/types/blocks';
import type { BlockDefinition } from '@/types/blocks';

interface WikiHeaderProps {
  block: BlockDefinition;
}

export function WikiHeader({ block }: WikiHeaderProps) {
  const { history, back, close } = useWikiStore();
  const cat = CATEGORIES.find((c) => c.id === block.category);

  return (
    <div className="flex items-center gap-3 px-5 py-3 border-b border-gray-200 bg-white shrink-0">
      {/* Back / close */}
      {history.length > 0 ? (
        <button
          onClick={back}
          className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
          title="Back"
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
      ) : (
        <button
          onClick={close}
          className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
          title="Close"
        >
          <X className="w-4 h-4" />
        </button>
      )}

      {/* Title + badges */}
      <div className="flex-1 min-w-0">
        <h1 className="text-base font-semibold text-gray-900 truncate">
          {block.name}
        </h1>
        <div className="flex items-center gap-2 mt-0.5">
          {cat && (
            <span
              className="text-[10px] uppercase tracking-wider font-medium px-1.5 py-0.5 rounded"
              style={{
                backgroundColor: cat.color + '18',
                color: cat.color,
              }}
            >
              {cat.name}
            </span>
          )}
          {block.documentation?.complexity && (
            <>
              <span className="text-[10px] bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded font-mono">
                Time: {block.documentation.complexity.time}
              </span>
              <span className="text-[10px] bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded font-mono">
                Space: {block.documentation.complexity.space}
              </span>
            </>
          )}
        </div>
      </div>

      {/* Close (when history exists, back is on left) */}
      {history.length > 0 && (
        <button
          onClick={close}
          className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
          title="Close"
        >
          <X className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}
