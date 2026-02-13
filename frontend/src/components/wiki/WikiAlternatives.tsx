import { ArrowRight } from 'lucide-react';
import { useWikiStore } from '@/stores/wikiStore';
import { getBlockDefinition } from '@/types/blocks';
import type { BlockAlternative } from '@/types/blocks';

interface WikiAlternativesProps {
  alternatives: BlockAlternative[];
}

export function WikiAlternatives({ alternatives }: WikiAlternativesProps) {
  const navigateTo = useWikiStore((s) => s.navigateTo);

  return (
    <div className="space-y-2">
      {alternatives.map((alt) => {
        const def = getBlockDefinition(alt.blockType);
        return (
          <button
            key={alt.blockType}
            onClick={() => navigateTo(alt.blockType)}
            className="w-full text-left px-3 py-2.5 rounded-lg border border-gray-200 hover:border-blue-300 hover:bg-blue-50/50 transition-colors group"
          >
            <div className="flex items-center justify-between mb-0.5">
              <span className="text-sm font-medium text-gray-900 group-hover:text-blue-700">
                {def?.name ?? alt.blockType}
              </span>
              <ArrowRight className="w-3.5 h-3.5 text-gray-300 group-hover:text-blue-500 transition-colors" />
            </div>
            <p className="text-xs text-gray-500 leading-relaxed">
              {alt.comparison}
            </p>
          </button>
        );
      })}
    </div>
  );
}
