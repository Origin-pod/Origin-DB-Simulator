import { useState, useMemo, useCallback } from 'react';
import {
  X,
  CheckCircle2,
  ArrowRight,
  GitCompare,
  Copy,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useDesignStore, type Design } from '@/stores/designStore';
import type { ExecutionResult } from '@/engine/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface MetricRow {
  label: string;
  unit: string;
  valueA: number;
  valueB: number;
  winner: 'A' | 'B' | 'tie';
  diffPct: number; // positive = B is better for throughput-like, A is better for latency-like
  lowerIsBetter: boolean;
}

interface ComparisonResult {
  metrics: MetricRow[];
  summaries: string[];
  recommendation: string;
}

// ---------------------------------------------------------------------------
// Comparison logic
// ---------------------------------------------------------------------------

function compare(a: ExecutionResult, b: ExecutionResult): ComparisonResult {
  const rows: MetricRow[] = [];

  function add(
    label: string,
    unit: string,
    vA: number,
    vB: number,
    lowerIsBetter: boolean,
  ) {
    const diff = vA === 0 ? 0 : ((vB - vA) / vA) * 100;
    let winner: 'A' | 'B' | 'tie' = 'tie';
    if (Math.abs(diff) > 1) {
      if (lowerIsBetter) {
        winner = vB < vA ? 'B' : 'A';
      } else {
        winner = vB > vA ? 'B' : 'A';
      }
    }
    rows.push({ label, unit, valueA: vA, valueB: vB, winner, diffPct: diff, lowerIsBetter });
  }

  add('Throughput', 'ops/sec', a.metrics.throughput, b.metrics.throughput, false);
  add('Avg Latency', 'ms', a.metrics.latency.avg, b.metrics.latency.avg, true);
  add('p50 Latency', 'ms', a.metrics.latency.p50, b.metrics.latency.p50, true);
  add('p95 Latency', 'ms', a.metrics.latency.p95, b.metrics.latency.p95, true);
  add('p99 Latency', 'ms', a.metrics.latency.p99, b.metrics.latency.p99, true);
  add('Failed Ops', '', a.metrics.failedOperations, b.metrics.failedOperations, true);
  add('Duration', 'ms', a.duration, b.duration, true);

  // Summaries
  const summaries: string[] = [];
  const throughputRow = rows[0];
  if (throughputRow.winner !== 'tie') {
    const w = throughputRow.winner;
    const pct = Math.abs(throughputRow.diffPct).toFixed(0);
    summaries.push(
      `Design ${w} has ${pct}% higher throughput.`,
    );
  }
  const p99Row = rows[4];
  if (p99Row.winner !== 'tie') {
    const w = p99Row.winner;
    const pct = Math.abs(p99Row.diffPct).toFixed(0);
    summaries.push(
      `Design ${w} has ${pct}% lower p99 latency.`,
    );
  }
  const durRow = rows[6];
  if (durRow.winner !== 'tie') {
    const w = durRow.winner;
    const pct = Math.abs(durRow.diffPct).toFixed(0);
    summaries.push(
      `Design ${w} completes ${pct}% faster.`,
    );
  }

  // Recommendation
  const aWins = rows.filter((r) => r.winner === 'A').length;
  const bWins = rows.filter((r) => r.winner === 'B').length;
  let recommendation: string;
  if (aWins > bWins) {
    recommendation = 'Design A wins on more metrics and is the better overall choice for this workload.';
  } else if (bWins > aWins) {
    recommendation = 'Design B wins on more metrics and is the better overall choice for this workload.';
  } else {
    recommendation = 'Both designs perform similarly. Choose based on your priority (throughput vs latency).';
  }

  return { metrics: rows, summaries, recommendation };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtVal(val: number, unit: string): string {
  if (unit === 'ops/sec') {
    if (val >= 1_000_000) return `${(val / 1_000_000).toFixed(1)}M`;
    if (val >= 1_000) return `${(val / 1_000).toFixed(1)}K`;
    return String(Math.round(val));
  }
  if (unit === 'ms') return val.toFixed(2);
  return String(Math.round(val));
}

function diffBadge(pct: number): string {
  const sign = pct > 0 ? '+' : '';
  return `${sign}${pct.toFixed(0)}%`;
}

// ---------------------------------------------------------------------------
// Design selector for comparison
// ---------------------------------------------------------------------------

function DesignSelector({
  label,
  selectedId,
  designs,
  onChange,
}: {
  label: string;
  selectedId: string;
  designs: Design[];
  onChange: (id: string) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs font-semibold text-gray-500 uppercase">{label}</span>
      <select
        value={selectedId}
        onChange={(e) => onChange(e.target.value)}
        className="px-2 py-1 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        {designs.map((d) => (
          <option key={d.id} value={d.id}>
            {d.name}
          </option>
        ))}
      </select>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main ComparisonView
// ---------------------------------------------------------------------------

export function ComparisonView({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { designs, setActiveDesign, saveCurrentCanvas } = useDesignStore();

  // Filter designs that have results
  const designsWithResults = useMemo(
    () => designs.filter((d) => d.lastResult !== null),
    [designs],
  );

  const [designAId, setDesignAId] = useState<string>(
    designsWithResults[0]?.id ?? '',
  );
  const [designBId, setDesignBId] = useState<string>(
    designsWithResults[1]?.id ?? designsWithResults[0]?.id ?? '',
  );
  const [copied, setCopied] = useState(false);

  // Sync selections when modal opens
  const designA = designs.find((d) => d.id === designAId);
  const designB = designs.find((d) => d.id === designBId);

  const comparison = useMemo(() => {
    if (!designA?.lastResult || !designB?.lastResult) return null;
    return compare(designA.lastResult, designB.lastResult);
  }, [designA, designB]);

  const handleChoose = useCallback(
    (designId: string) => {
      saveCurrentCanvas();
      setActiveDesign(designId);
      onClose();
    },
    [saveCurrentCanvas, setActiveDesign, onClose],
  );

  const handleCopy = useCallback(() => {
    if (!comparison || !designA || !designB) return;
    const lines = [
      `Comparison: ${designA.name} vs ${designB.name}`,
      '─'.repeat(50),
      ...comparison.metrics.map(
        (r) =>
          `${r.label.padEnd(16)} ${fmtVal(r.valueA, r.unit).padStart(10)} ${r.unit}  vs  ${fmtVal(r.valueB, r.unit).padStart(10)} ${r.unit}  ${r.winner === 'tie' ? '  tie' : `  → ${r.winner}`}`,
      ),
      '',
      ...comparison.summaries,
      '',
      comparison.recommendation,
    ];
    navigator.clipboard.writeText(lines.join('\n'));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [comparison, designA, designB]);

  if (!open) return null;

  // Not enough designs with results
  if (designsWithResults.length < 2) {
    return (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
        onClick={onClose}
      >
        <div
          className="bg-white rounded-xl shadow-2xl w-full max-w-md p-6 text-center"
          onClick={(e) => e.stopPropagation()}
        >
          <GitCompare className="w-12 h-12 text-gray-300 mx-auto mb-3" />
          <h2 className="text-lg font-semibold text-gray-900 mb-2">
            Nothing to Compare
          </h2>
          <p className="text-sm text-gray-500 mb-4">
            Run at least 2 designs to compare their results. Create a new design
            tab, configure it differently, and run both.
          </p>
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    );
  }

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
            <GitCompare className="w-5 h-5 text-primary-500" />
            <h2 className="text-lg font-semibold text-gray-900">Compare Designs</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 text-gray-400 hover:text-gray-600"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Design selectors */}
        <div className="flex items-center justify-between px-6 py-3 bg-gray-50 border-b border-gray-200">
          <DesignSelector
            label="A"
            selectedId={designAId}
            designs={designsWithResults}
            onChange={setDesignAId}
          />
          <ArrowRight className="w-4 h-4 text-gray-400" />
          <DesignSelector
            label="B"
            selectedId={designBId}
            designs={designsWithResults}
            onChange={setDesignBId}
          />
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-5">
          {designAId === designBId ? (
            <p className="text-sm text-gray-500 text-center py-8">
              Select two different designs to compare.
            </p>
          ) : comparison ? (
            <>
              {/* Metric table */}
              <div>
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-200 text-xs text-gray-500 uppercase">
                      <th className="py-2 text-left font-medium">Metric</th>
                      <th className="py-2 text-right font-medium">
                        {designA?.name ?? 'A'}
                      </th>
                      <th className="py-2 text-right font-medium">
                        {designB?.name ?? 'B'}
                      </th>
                      <th className="py-2 text-center font-medium w-20">Diff</th>
                    </tr>
                  </thead>
                  <tbody>
                    {comparison.metrics.map((row) => (
                      <tr
                        key={row.label}
                        className="border-b border-gray-100"
                      >
                        <td className="py-2 text-gray-700 font-medium">
                          {row.label}
                        </td>
                        <td className="py-2 text-right tabular-nums">
                          <span
                            className={
                              row.winner === 'A'
                                ? 'text-green-700 font-semibold'
                                : 'text-gray-700'
                            }
                          >
                            {fmtVal(row.valueA, row.unit)}
                            <span className="text-gray-400 ml-0.5 text-xs">
                              {row.unit}
                            </span>
                          </span>
                          {row.winner === 'A' && (
                            <CheckCircle2 className="w-3.5 h-3.5 text-green-500 inline ml-1" />
                          )}
                        </td>
                        <td className="py-2 text-right tabular-nums">
                          <span
                            className={
                              row.winner === 'B'
                                ? 'text-green-700 font-semibold'
                                : 'text-gray-700'
                            }
                          >
                            {fmtVal(row.valueB, row.unit)}
                            <span className="text-gray-400 ml-0.5 text-xs">
                              {row.unit}
                            </span>
                          </span>
                          {row.winner === 'B' && (
                            <CheckCircle2 className="w-3.5 h-3.5 text-green-500 inline ml-1" />
                          )}
                        </td>
                        <td className="py-2 text-center">
                          {row.winner !== 'tie' ? (
                            <span
                              className={`text-xs font-mono px-1.5 py-0.5 rounded ${
                                row.winner === 'B'
                                  ? row.lowerIsBetter
                                    ? 'bg-green-100 text-green-700'
                                    : row.diffPct > 0
                                      ? 'bg-green-100 text-green-700'
                                      : 'bg-red-100 text-red-700'
                                  : row.lowerIsBetter
                                    ? 'bg-red-100 text-red-700'
                                    : row.diffPct < 0
                                      ? 'bg-green-100 text-green-700'
                                      : 'bg-red-100 text-red-700'
                              }`}
                            >
                              {diffBadge(row.diffPct)}
                            </span>
                          ) : (
                            <span className="text-xs text-gray-400">tie</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Summary */}
              {comparison.summaries.length > 0 && (
                <div className="bg-gray-50 rounded-lg p-4 space-y-1">
                  <h4 className="text-xs font-semibold text-gray-700 uppercase tracking-wider mb-2">
                    Summary
                  </h4>
                  {comparison.summaries.map((s, i) => (
                    <p key={i} className="text-sm text-gray-700">
                      • {s}
                    </p>
                  ))}
                  <p className="text-sm text-gray-900 font-medium mt-3">
                    {comparison.recommendation}
                  </p>
                </div>
              )}
            </>
          ) : (
            <p className="text-sm text-gray-500 text-center py-8">
              No results available for the selected designs.
            </p>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-4 border-t border-gray-200 bg-gray-50 rounded-b-xl">
          <div className="flex items-center gap-2">
            <button
              onClick={handleCopy}
              className="flex items-center gap-1 px-3 py-1.5 text-xs text-gray-600 hover:text-gray-800 border border-gray-200 rounded-lg hover:bg-gray-100 transition-colors"
            >
              {copied ? (
                <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
              ) : (
                <Copy className="w-3.5 h-3.5" />
              )}
              {copied ? 'Copied!' : 'Copy Report'}
            </button>
          </div>
          <div className="flex items-center gap-2">
            {comparison && designAId !== designBId && (
              <>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => handleChoose(designAId)}
                >
                  Choose {designA?.name ?? 'A'}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => handleChoose(designBId)}
                >
                  Choose {designB?.name ?? 'B'}
                </Button>
              </>
            )}
            <Button variant="ghost" size="sm" onClick={onClose}>
              Close
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
