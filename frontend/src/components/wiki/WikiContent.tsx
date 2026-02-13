import {
  BookOpen,
  HelpCircle,
  Brain,
  Clock,
  SlidersHorizontal,
  Lightbulb,
  Scale,
  GitCompareArrows,
  GraduationCap,
  BarChart3,
  Bookmark,
} from 'lucide-react';
import { Markdown } from '@/lib/markdown';
import { WikiSection } from './WikiSection';
import { WikiAlgorithm } from './WikiAlgorithm';
import { WikiAlternatives } from './WikiAlternatives';
import { WikiMetrics } from './WikiMetrics';
import { WikiReferences } from './WikiReferences';
import type { BlockDefinition } from '@/types/blocks';
import type { TOCItem } from './WikiTOC';

interface WikiContentProps {
  block: BlockDefinition;
}

/** Build the TOC item list for the current block's available content. */
export function buildTOCItems(block: BlockDefinition): TOCItem[] {
  const doc = block.documentation;
  const items: TOCItem[] = [];

  if (doc?.overview || doc?.details || doc?.summary)
    items.push({ id: 'overview', label: 'Overview' });
  if (doc?.motivation)
    items.push({ id: 'motivation', label: 'Why It Exists' });
  if (doc?.algorithm)
    items.push({ id: 'algorithm', label: 'Algorithm' });
  if (doc?.complexity)
    items.push({ id: 'complexity', label: 'Complexity' });
  if (doc?.parameterGuide && Object.keys(doc.parameterGuide).length > 0)
    items.push({ id: 'parameters', label: 'Parameters' });
  if (doc?.useCases && doc.useCases.length > 0)
    items.push({ id: 'usecases', label: 'When To Use' });
  if (doc?.tradeoffs && doc.tradeoffs.length > 0)
    items.push({ id: 'tradeoffs', label: 'Tradeoffs' });
  if (doc?.alternatives && doc.alternatives.length > 0)
    items.push({ id: 'alternatives', label: 'Compared To' });
  if (doc?.examples && doc.examples.length > 0)
    items.push({ id: 'examples', label: 'Real-World Usage' });
  if (block.metricDefinitions && block.metricDefinitions.length > 0)
    items.push({ id: 'metrics', label: 'Metrics' });
  if (block.references && block.references.length > 0)
    items.push({ id: 'references', label: 'References' });

  return items;
}

export function WikiContent({ block }: WikiContentProps) {
  const doc = block.documentation;

  return (
    <div className="space-y-6">
      {/* Overview */}
      {(doc?.overview || doc?.details || doc?.summary) && (
        <WikiSection id="overview" title="Overview" icon={<BookOpen className="w-4 h-4" />}>
          <Markdown
            text={doc.overview || doc.details || doc.summary}
            className="text-sm text-gray-700 leading-relaxed"
          />
        </WikiSection>
      )}

      {/* Motivation */}
      {doc?.motivation && (
        <WikiSection id="motivation" title="Why It Exists" icon={<HelpCircle className="w-4 h-4" />}>
          <Markdown
            text={doc.motivation}
            className="text-sm text-gray-700 leading-relaxed"
          />
        </WikiSection>
      )}

      {/* Algorithm */}
      {doc?.algorithm && (
        <WikiSection id="algorithm" title="Algorithm" icon={<Brain className="w-4 h-4" />}>
          <WikiAlgorithm algorithm={doc.algorithm} />
        </WikiSection>
      )}

      {/* Complexity */}
      {doc?.complexity && (
        <WikiSection id="complexity" title="Complexity" icon={<Clock className="w-4 h-4" />}>
          <div className="flex gap-3">
            <div className="flex-1 flex flex-col items-center px-4 py-3 bg-gray-50 rounded-lg border border-gray-200">
              <span className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Time</span>
              <span className="text-sm font-mono font-semibold text-gray-800">{doc.complexity.time}</span>
            </div>
            <div className="flex-1 flex flex-col items-center px-4 py-3 bg-gray-50 rounded-lg border border-gray-200">
              <span className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Space</span>
              <span className="text-sm font-mono font-semibold text-gray-800">{doc.complexity.space}</span>
            </div>
          </div>
        </WikiSection>
      )}

      {/* Parameters Explained */}
      {doc?.parameterGuide && Object.keys(doc.parameterGuide).length > 0 && (
        <WikiSection id="parameters" title="Parameters Explained" icon={<SlidersHorizontal className="w-4 h-4" />}>
          <div className="space-y-3">
            {Object.entries(doc.parameterGuide).map(([param, explanation]) => (
              <div key={param}>
                <h4 className="text-xs font-mono font-semibold text-gray-800 bg-gray-100 inline-block px-2 py-0.5 rounded mb-1">
                  {param}
                </h4>
                <p className="text-sm text-gray-600 leading-relaxed">{explanation}</p>
              </div>
            ))}
          </div>
        </WikiSection>
      )}

      {/* When To Use */}
      {doc?.useCases && doc.useCases.length > 0 && (
        <WikiSection id="usecases" title="When To Use" icon={<Lightbulb className="w-4 h-4" />}>
          <ul className="space-y-1.5">
            {doc.useCases.map((uc, i) => (
              <li key={i} className="flex gap-2 text-sm text-gray-700">
                <span className="text-green-500 mt-0.5 shrink-0">+</span>
                <span>{uc}</span>
              </li>
            ))}
          </ul>
        </WikiSection>
      )}

      {/* Tradeoffs */}
      {doc?.tradeoffs && doc.tradeoffs.length > 0 && (
        <WikiSection id="tradeoffs" title="Tradeoffs" icon={<Scale className="w-4 h-4" />}>
          <ul className="space-y-1.5">
            {doc.tradeoffs.map((t, i) => (
              <li key={i} className="flex gap-2 text-sm text-gray-700">
                <span className="text-amber-500 mt-0.5 shrink-0">~</span>
                <span>{t}</span>
              </li>
            ))}
          </ul>
        </WikiSection>
      )}

      {/* Compared To */}
      {doc?.alternatives && doc.alternatives.length > 0 && (
        <WikiSection id="alternatives" title="Compared To" icon={<GitCompareArrows className="w-4 h-4" />}>
          <WikiAlternatives alternatives={doc.alternatives} />
        </WikiSection>
      )}

      {/* Real-World Usage */}
      {doc?.examples && doc.examples.length > 0 && (
        <WikiSection id="examples" title="Real-World Usage" icon={<GraduationCap className="w-4 h-4" />}>
          <ul className="space-y-1.5">
            {doc.examples.map((ex, i) => (
              <li key={i} className="flex gap-2 text-sm text-gray-700">
                <span className="text-blue-400 mt-0.5 shrink-0">-</span>
                <span>{ex}</span>
              </li>
            ))}
          </ul>
        </WikiSection>
      )}

      {/* Metrics */}
      {block.metricDefinitions && block.metricDefinitions.length > 0 && (
        <WikiSection id="metrics" title="Metrics" icon={<BarChart3 className="w-4 h-4" />}>
          <WikiMetrics metrics={block.metricDefinitions} />
        </WikiSection>
      )}

      {/* References */}
      {block.references && block.references.length > 0 && (
        <WikiSection id="references" title="References" icon={<Bookmark className="w-4 h-4" />}>
          <WikiReferences references={block.references} />
        </WikiSection>
      )}
    </div>
  );
}
