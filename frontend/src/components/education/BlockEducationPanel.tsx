import { useState } from 'react';
import {
  BookOpen,
  Brain,
  Clock,
  Lightbulb,
  Scale,
  Bookmark,
  BarChart3,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  GraduationCap,
} from 'lucide-react';
import type { BlockDefinition } from '@/types/blocks';

interface BlockEducationPanelProps {
  block: BlockDefinition;
  /** When true, all sections start collapsed (used in sidebar) */
  compact?: boolean;
}

export function BlockEducationPanel({
  block,
  compact = false,
}: BlockEducationPanelProps) {
  const [expanded, setExpanded] = useState<Set<string>>(
    new Set(compact ? [] : ['overview']),
  );
  const doc = block.documentation;

  // Nothing to show without documentation
  if (!doc?.overview && !doc?.algorithm && !doc?.details) return null;

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="space-y-1">
      {/* Overview */}
      {(doc.overview || doc.details || doc.summary) && (
        <Section
          title="Overview"
          icon={<BookOpen className="w-3.5 h-3.5" />}
          open={expanded.has('overview')}
          onToggle={() => toggle('overview')}
        >
          <p className="text-xs text-gray-600 leading-relaxed">
            {doc.overview || doc.details || doc.summary}
          </p>
        </Section>
      )}

      {/* Algorithm */}
      {doc.algorithm && (
        <Section
          title="How It Works"
          icon={<Brain className="w-3.5 h-3.5" />}
          open={expanded.has('algorithm')}
          onToggle={() => toggle('algorithm')}
        >
          <p className="text-xs text-gray-600 leading-relaxed bg-gray-50 p-2.5 rounded-md border border-gray-100 font-mono">
            {doc.algorithm}
          </p>
        </Section>
      )}

      {/* Complexity */}
      {doc.complexity && (
        <Section
          title="Complexity"
          icon={<Clock className="w-3.5 h-3.5" />}
          open={expanded.has('complexity')}
          onToggle={() => toggle('complexity')}
        >
          <div className="flex gap-2">
            <ComplexityBadge label="Time" value={doc.complexity.time} />
            <ComplexityBadge label="Space" value={doc.complexity.space} />
          </div>
        </Section>
      )}

      {/* Use Cases */}
      {doc.useCases && doc.useCases.length > 0 && (
        <Section
          title="When To Use"
          icon={<Lightbulb className="w-3.5 h-3.5" />}
          open={expanded.has('usecases')}
          onToggle={() => toggle('usecases')}
        >
          <ul className="text-xs text-gray-600 space-y-1.5">
            {doc.useCases.map((uc, i) => (
              <li key={i} className="flex gap-2">
                <span className="text-green-500 mt-0.5 shrink-0">+</span>
                <span>{uc}</span>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Tradeoffs */}
      {doc.tradeoffs && doc.tradeoffs.length > 0 && (
        <Section
          title="Tradeoffs"
          icon={<Scale className="w-3.5 h-3.5" />}
          open={expanded.has('tradeoffs')}
          onToggle={() => toggle('tradeoffs')}
        >
          <ul className="text-xs text-gray-600 space-y-1.5">
            {doc.tradeoffs.map((t, i) => (
              <li key={i} className="flex gap-2">
                <span className="text-amber-500 mt-0.5 shrink-0">~</span>
                <span>{t}</span>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Real-World Examples */}
      {doc.examples && doc.examples.length > 0 && (
        <Section
          title="Real-World Examples"
          icon={<GraduationCap className="w-3.5 h-3.5" />}
          open={expanded.has('examples')}
          onToggle={() => toggle('examples')}
        >
          <div className="flex flex-wrap gap-1.5">
            {doc.examples.map((ex, i) => (
              <span
                key={i}
                className="text-[11px] bg-blue-50 text-blue-700 px-2 py-0.5 rounded-full border border-blue-100"
              >
                {ex}
              </span>
            ))}
          </div>
        </Section>
      )}

      {/* Metrics Tracked */}
      {block.metricDefinitions && block.metricDefinitions.length > 0 && (
        <Section
          title={`Metrics Tracked (${block.metricDefinitions.length})`}
          icon={<BarChart3 className="w-3.5 h-3.5" />}
          open={expanded.has('metrics')}
          onToggle={() => toggle('metrics')}
        >
          <div className="space-y-1.5">
            {block.metricDefinitions.map((m) => (
              <div
                key={m.id}
                className="flex items-start gap-2 text-xs"
              >
                <span className="font-mono text-gray-700 bg-gray-100 px-1.5 py-0.5 rounded text-[11px] shrink-0">
                  {m.name}
                </span>
                <span className="text-gray-500">
                  {m.description}
                  {m.unit && (
                    <span className="text-gray-400 ml-1">({m.unit})</span>
                  )}
                </span>
              </div>
            ))}
          </div>
        </Section>
      )}

      {/* References */}
      {block.references && block.references.length > 0 && (
        <Section
          title="References"
          icon={<Bookmark className="w-3.5 h-3.5" />}
          open={expanded.has('references')}
          onToggle={() => toggle('references')}
        >
          <div className="space-y-2">
            {block.references.map((ref, i) => (
              <div
                key={i}
                className="text-xs text-gray-600 border-l-2 border-gray-200 pl-2"
              >
                <div className="flex items-center gap-1.5">
                  <span className="inline-block px-1.5 py-0.5 bg-gray-100 text-gray-500 rounded text-[10px] uppercase tracking-wider">
                    {ref.refType}
                  </span>
                  <span className="font-medium text-gray-700">
                    {ref.title}
                  </span>
                </div>
                {ref.citation && (
                  <p className="text-gray-400 italic mt-0.5 text-[11px]">
                    {ref.citation}
                  </p>
                )}
                {ref.url && (
                  <a
                    href={ref.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-500 hover:underline inline-flex items-center gap-1 mt-0.5"
                  >
                    Read more <ExternalLink className="w-3 h-3" />
                  </a>
                )}
              </div>
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Reusable collapsible section
// ---------------------------------------------------------------------------

function Section({
  title,
  icon,
  open,
  onToggle,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="border border-gray-100 rounded-lg overflow-hidden">
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-gray-50 transition-colors"
      >
        <span className="text-gray-400">
          {open ? (
            <ChevronDown className="w-3 h-3" />
          ) : (
            <ChevronRight className="w-3 h-3" />
          )}
        </span>
        <span className="text-gray-400">{icon}</span>
        <span className="text-xs font-medium text-gray-700">{title}</span>
      </button>
      {open && <div className="px-3 pb-3">{children}</div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Complexity badge
// ---------------------------------------------------------------------------

function ComplexityBadge({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="flex-1 flex flex-col items-center px-3 py-2 bg-gray-50 rounded-lg border border-gray-100">
      <span className="text-[10px] uppercase tracking-wider text-gray-400 mb-0.5">
        {label}
      </span>
      <span className="text-[11px] font-mono font-semibold text-gray-800 text-center leading-tight">
        {value}
      </span>
    </div>
  );
}
