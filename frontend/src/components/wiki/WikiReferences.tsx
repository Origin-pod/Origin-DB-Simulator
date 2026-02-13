import { ExternalLink } from 'lucide-react';
import type { BlockReference } from '@/types/blocks';

interface WikiReferencesProps {
  references: BlockReference[];
}

const TYPE_COLORS: Record<string, string> = {
  Paper: 'bg-indigo-100 text-indigo-700',
  Book: 'bg-emerald-100 text-emerald-700',
  Blog: 'bg-amber-100 text-amber-700',
  Implementation: 'bg-blue-100 text-blue-700',
};

export function WikiReferences({ references }: WikiReferencesProps) {
  return (
    <div className="space-y-3">
      {references.map((ref, i) => (
        <div key={i} className="border-l-2 border-gray-200 pl-3">
          <div className="flex items-center gap-2 mb-0.5">
            <span className={`text-[10px] uppercase tracking-wider font-medium px-1.5 py-0.5 rounded ${TYPE_COLORS[ref.refType] ?? 'bg-gray-100 text-gray-600'}`}>
              {ref.refType}
            </span>
            <span className="text-sm font-medium text-gray-800">{ref.title}</span>
          </div>
          {ref.citation && (
            <p className="text-xs text-gray-400 italic mt-0.5">{ref.citation}</p>
          )}
          {ref.url && (
            <a
              href={ref.url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-blue-500 hover:underline inline-flex items-center gap-1 mt-1"
            >
              Read more <ExternalLink className="w-3 h-3" />
            </a>
          )}
        </div>
      ))}
    </div>
  );
}
