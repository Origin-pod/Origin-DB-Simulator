import { useState, useCallback, useMemo } from 'react';
import {
  ChevronDown,
  ChevronUp,
  Gauge,
  Clock,
  Activity,
  HardDrive,
  AlertTriangle,
  Lightbulb,
  Download,
  Copy,
  CheckCircle2,
  X,
} from 'lucide-react';
import { useExecutionStore } from '@/stores/executionStore';
import { useCanvasStore } from '@/stores/canvasStore';
import { useWorkloadStore } from '@/stores/workloadStore';
import { CATEGORY_COLORS, type BlockCategory, type BlockNodeData } from '@/types';
import type { BlockMetrics, ExecutionResult } from '@/engine/types';
import { downloadFile } from '@/lib/persistence';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(Math.round(n));
}

function fmtMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(2)}ms`;
}

/** Infer category from blockType for color coding */
function blockCategory(blockType: string): BlockCategory {
  const map: Record<string, BlockCategory> = {
    schema_definition: 'storage',
    heap_storage: 'storage',
    clustered_storage: 'storage',
    lsm_tree: 'storage',
    btree_index: 'index',
    hash_index: 'index',
    covering_index: 'index',
    lru_buffer: 'buffer',
    clock_buffer: 'buffer',
    sequential_scan: 'execution',
    index_scan: 'execution',
    filter: 'execution',
    sort: 'execution',
    hash_join: 'execution',
    row_lock: 'concurrency',
    mvcc: 'concurrency',
    wal: 'transaction',
  };
  return map[blockType] ?? 'storage';
}

// ---------------------------------------------------------------------------
// Bottleneck / suggestion engine
// ---------------------------------------------------------------------------

interface Suggestion {
  type: 'performance' | 'warning';
  icon: 'alert' | 'lightbulb';
  message: string;
  detail: string;
}

function generateSuggestions(result: ExecutionResult): Suggestion[] {
  const suggestions: Suggestion[] = [];
  const sorted = [...result.blockMetrics].sort(
    (a, b) => b.percentage - a.percentage,
  );

  if (sorted.length === 0) return suggestions;

  const top = sorted[0];

  // Bottleneck: top block > 40%
  if (top.percentage > 40) {
    const cat = blockCategory(top.blockType);

    if (cat === 'storage') {
      suggestions.push({
        type: 'performance',
        icon: 'alert',
        message: `Storage is the bottleneck — "${top.blockName}" consumed ${top.percentage}% of execution time.`,
        detail:
          'Consider adding a buffer pool (LRU or Clock) to reduce disk I/O, or switch to clustered storage for better locality.',
      });
    } else if (cat === 'index') {
      suggestions.push({
        type: 'performance',
        icon: 'alert',
        message: `Index lookup is the bottleneck — "${top.blockName}" consumed ${top.percentage}% of time.`,
        detail:
          'Try increasing the B-tree fanout to reduce tree depth, or use a hash index for point lookups.',
      });
    } else if (cat === 'execution') {
      suggestions.push({
        type: 'performance',
        icon: 'alert',
        message: `Query execution is the bottleneck — "${top.blockName}" consumed ${top.percentage}% of time.`,
        detail:
          'Add an index to avoid sequential scans, or increase the memory limit for sort/join operations.',
      });
    } else {
      suggestions.push({
        type: 'performance',
        icon: 'alert',
        message: `"${top.blockName}" consumed ${top.percentage}% of execution time.`,
        detail: 'Review its parameters to see if tuning can reduce overhead.',
      });
    }
  }

  // Low cache hit rate
  for (const bm of result.blockMetrics) {
    const hitRate = bm.counters['hit_rate_pct'];
    if (hitRate !== undefined && hitRate < 70) {
      suggestions.push({
        type: 'warning',
        icon: 'lightbulb',
        message: `Cache hit rate is low (${hitRate}%) on "${bm.blockName}".`,
        detail:
          'Increase the buffer pool size to keep more pages in memory and reduce disk reads.',
      });
    }
  }

  // High latency
  if (result.metrics.latency.p99 > result.metrics.latency.avg * 10) {
    suggestions.push({
      type: 'warning',
      icon: 'lightbulb',
      message: 'High tail latency — p99 is 10x the average.',
      detail:
        'This typically indicates contention or spilling to disk. Check concurrency settings and memory limits.',
    });
  }

  return suggestions;
}

// ---------------------------------------------------------------------------
// Export helpers
// ---------------------------------------------------------------------------

function exportJSON(result: ExecutionResult): void {
  const json = JSON.stringify(result, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'db-simulator-metrics.json';
  a.click();
  URL.revokeObjectURL(url);
}

function exportCSV(result: ExecutionResult): void {
  const header = 'Block,Type,Execution Time (ms),Percentage (%)';
  const rows = result.blockMetrics.map(
    (bm) => `"${bm.blockName}","${bm.blockType}",${bm.executionTime},${bm.percentage}`,
  );
  const summaryHeader = '\nMetric,Value';
  const summaryRows = [
    `Throughput (ops/sec),${result.metrics.throughput}`,
    `Avg Latency (ms),${result.metrics.latency.avg}`,
    `p50 Latency (ms),${result.metrics.latency.p50}`,
    `p95 Latency (ms),${result.metrics.latency.p95}`,
    `p99 Latency (ms),${result.metrics.latency.p99}`,
    `Total Operations,${result.metrics.totalOperations}`,
    `Successful,${result.metrics.successfulOperations}`,
    `Failed,${result.metrics.failedOperations}`,
    `Total Duration (ms),${result.duration}`,
  ];
  const csv = [header, ...rows, summaryHeader, ...summaryRows].join('\n');
  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'db-simulator-metrics.csv';
  a.click();
  URL.revokeObjectURL(url);
}

function copySummary(result: ExecutionResult): string {
  const m = result.metrics;
  return [
    `DB Simulator — Execution Results`,
    `────────────────────────────────`,
    `Throughput:  ${fmtNum(m.throughput)} ops/sec`,
    `Latency:     avg ${fmtMs(m.latency.avg)}  |  p50 ${fmtMs(m.latency.p50)}  |  p95 ${fmtMs(m.latency.p95)}  |  p99 ${fmtMs(m.latency.p99)}`,
    `Operations:  ${fmtNum(m.successfulOperations)} / ${fmtNum(m.totalOperations)} (${m.failedOperations} failed)`,
    `Duration:    ${fmtMs(result.duration)}`,
    ``,
    `Block Breakdown:`,
    ...result.blockMetrics
      .sort((a, b) => b.percentage - a.percentage)
      .map((bm) => `  ${bm.blockName.padEnd(24)} ${bm.percentage.toFixed(1).padStart(5)}%  ${fmtMs(bm.executionTime)}`),
  ].join('\n');
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function MetricCard({
  icon,
  label,
  value,
  sub,
  iconColor,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  sub: string;
  iconColor: string;
}) {
  return (
    <div className="flex items-center gap-3 px-4 py-3 bg-white rounded-lg border border-gray-100">
      <div
        className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
        style={{ backgroundColor: `${iconColor}15`, color: iconColor }}
      >
        {icon}
      </div>
      <div>
        <p className="text-lg font-bold text-gray-900 tabular-nums leading-tight">
          {value}
        </p>
        <p className="text-xs text-gray-500">{label}</p>
        <p className="text-[10px] text-gray-400">{sub}</p>
      </div>
    </div>
  );
}

function BlockBar({ bm, maxPct }: { bm: BlockMetrics; maxPct: number }) {
  const cat = blockCategory(bm.blockType);
  const color = CATEGORY_COLORS[cat];
  const barWidth = maxPct > 0 ? (bm.percentage / maxPct) * 100 : 0;
  const [expanded, setExpanded] = useState(false);
  const counters = Object.entries(bm.counters);

  return (
    <div>
      <button
        onClick={() => counters.length > 0 && setExpanded(!expanded)}
        className="w-full flex items-center gap-3 py-1.5 group text-left"
      >
        <span className="text-xs text-gray-700 w-36 truncate flex-shrink-0 font-medium">
          {bm.blockName}
        </span>
        <div className="flex-1 h-5 bg-gray-100 rounded overflow-hidden">
          <div
            className="h-full rounded transition-all duration-500"
            style={{ width: `${barWidth}%`, backgroundColor: color }}
          />
        </div>
        <span className="text-xs text-gray-500 tabular-nums w-20 text-right flex-shrink-0">
          {bm.percentage.toFixed(1)}% ({fmtMs(bm.executionTime)})
        </span>
        {counters.length > 0 && (
          <span className="text-gray-300 group-hover:text-gray-500 w-4">
            {expanded ? (
              <ChevronUp className="w-3 h-3" />
            ) : (
              <ChevronDown className="w-3 h-3" />
            )}
          </span>
        )}
      </button>
      {expanded && counters.length > 0 && (
        <div className="ml-40 mb-2 grid grid-cols-3 gap-x-4 gap-y-0.5">
          {counters.map(([key, val]) => (
            <div key={key} className="flex justify-between text-[11px]">
              <span className="text-gray-400">{key.replace(/_/g, ' ')}</span>
              <span className="font-mono text-gray-600">{fmtNum(val)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SuggestionsPanel({ suggestions }: { suggestions: Suggestion[] }) {
  if (suggestions.length === 0) return null;

  return (
    <div className="space-y-2">
      <h4 className="text-xs font-semibold text-gray-700 uppercase tracking-wider">
        Insights
      </h4>
      {suggestions.map((s, i) => (
        <div
          key={i}
          className={`flex items-start gap-2 px-3 py-2 rounded-lg text-sm ${
            s.type === 'performance'
              ? 'bg-amber-50 border border-amber-200'
              : 'bg-blue-50 border border-blue-200'
          }`}
        >
          {s.icon === 'alert' ? (
            <AlertTriangle className="w-4 h-4 text-amber-500 mt-0.5 flex-shrink-0" />
          ) : (
            <Lightbulb className="w-4 h-4 text-blue-500 mt-0.5 flex-shrink-0" />
          )}
          <div>
            <p
              className={`text-xs font-medium ${
                s.type === 'performance' ? 'text-amber-800' : 'text-blue-800'
              }`}
            >
              {s.message}
            </p>
            <p className="text-xs text-gray-600 mt-0.5">{s.detail}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

function generateMarkdownReport(result: ExecutionResult): string {
  const m = result.metrics;
  const { designName, nodes } = useCanvasStore.getState();
  const { workload } = useWorkloadStore.getState();

  const architecture = nodes
    .map((n) => {
      const data = n.data as BlockNodeData;
      const params = Object.entries(data.parameters)
        .filter(([, v]) => v !== '' && v !== false)
        .map(([k, v]) => `${k}: ${v}`)
        .join(', ');
      return `- **${data.label}** (${data.category})${params ? ` — ${params}` : ''}`;
    })
    .join('\n');

  const workloadOps = workload.operations
    .map((op) => `${op.weight}% ${op.type}`)
    .join(', ');

  const blockTable = result.blockMetrics
    .sort((a, b) => b.percentage - a.percentage)
    .map((bm) => `| ${bm.blockName} | ${fmtMs(bm.executionTime)} | ${bm.percentage.toFixed(1)}% |`)
    .join('\n');

  return `# Database Design Report

Generated: ${new Date().toISOString().slice(0, 19).replace('T', ' ')}

## Design: ${designName}

### Architecture
${architecture}

### Workload
- **${workload.name}**
- ${fmtNum(workload.totalOperations)} operations
- ${workloadOps}
- ${workload.distribution} distribution
- ${workload.concurrency} concurrent operations

### Results

| Metric | Value |
|--------|-------|
| Throughput | ${fmtNum(m.throughput)} ops/sec |
| Latency (avg) | ${fmtMs(m.latency.avg)} |
| Latency (p50) | ${fmtMs(m.latency.p50)} |
| Latency (p95) | ${fmtMs(m.latency.p95)} |
| Latency (p99) | ${fmtMs(m.latency.p99)} |
| Total Operations | ${fmtNum(m.totalOperations)} |
| Successful | ${fmtNum(m.successfulOperations)} |
| Failed | ${fmtNum(m.failedOperations)} |
| Duration | ${fmtMs(result.duration)} |

### Block Breakdown

| Block | Time | Percentage |
|-------|------|------------|
${blockTable}

---
*Generated by DB Simulator*
`;
}

function ExportMenu({ result }: { result: ExecutionResult }) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(copySummary(result));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    setOpen(false);
  }, [result]);

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1 px-2 py-1 text-xs text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors"
      >
        <Download className="w-3.5 h-3.5" />
        Export
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-8 z-50 w-44 bg-white rounded-lg shadow-lg border border-gray-200 py-1">
            <button
              onClick={() => {
                exportJSON(result);
                setOpen(false);
              }}
              className="w-full px-3 py-1.5 text-left text-xs text-gray-700 hover:bg-gray-50 flex items-center gap-2"
            >
              <Download className="w-3 h-3" />
              Export JSON
            </button>
            <button
              onClick={() => {
                exportCSV(result);
                setOpen(false);
              }}
              className="w-full px-3 py-1.5 text-left text-xs text-gray-700 hover:bg-gray-50 flex items-center gap-2"
            >
              <Download className="w-3 h-3" />
              Export CSV
            </button>
            <button
              onClick={() => {
                const md = generateMarkdownReport(result);
                downloadFile('db-simulator-report.md', md, 'text/markdown');
                setOpen(false);
              }}
              className="w-full px-3 py-1.5 text-left text-xs text-gray-700 hover:bg-gray-50 flex items-center gap-2"
            >
              <Download className="w-3 h-3" />
              Export Report (.md)
            </button>
            <div className="border-t border-gray-100 my-1" />
            <button
              onClick={handleCopy}
              className="w-full px-3 py-1.5 text-left text-xs text-gray-700 hover:bg-gray-50 flex items-center gap-2"
            >
              {copied ? (
                <CheckCircle2 className="w-3 h-3 text-green-500" />
              ) : (
                <Copy className="w-3 h-3" />
              )}
              {copied ? 'Copied!' : 'Copy to clipboard'}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Dashboard
// ---------------------------------------------------------------------------

export function MetricsDashboard() {
  const { status, result, clearResults } = useExecutionStore();
  const [collapsed, setCollapsed] = useState(false);

  const suggestions = useMemo(
    () => (result ? generateSuggestions(result) : []),
    [result],
  );

  if (status !== 'complete' || !result) return null;

  const m = result.metrics;
  const sortedBlocks = [...result.blockMetrics].sort(
    (a, b) => b.percentage - a.percentage,
  );
  const maxPct = sortedBlocks.length > 0 ? sortedBlocks[0].percentage : 100;

  // Aggregate some useful counters
  const totalCacheHits = result.blockMetrics.reduce(
    (s, bm) => s + (bm.counters['cache_hits'] ?? 0),
    0,
  );
  const totalCacheMisses = result.blockMetrics.reduce(
    (s, bm) => s + (bm.counters['cache_misses'] ?? 0),
    0,
  );
  const cacheTotal = totalCacheHits + totalCacheMisses;
  const cacheHitRate = cacheTotal > 0 ? Math.round((totalCacheHits / cacheTotal) * 100) : null;

  const totalPagesRead = result.blockMetrics.reduce(
    (s, bm) => s + (bm.counters['pages_read'] ?? 0) + (bm.counters['pages_scanned'] ?? 0),
    0,
  );
  const totalPagesWritten = result.blockMetrics.reduce(
    (s, bm) => s + (bm.counters['pages_written'] ?? 0),
    0,
  );

  return (
    <div className="border-t border-gray-200 bg-gray-50 flex flex-col">
      {/* Header bar — always visible */}
      <div className="flex items-center justify-between px-4 py-2 bg-white border-b border-gray-200">
        <div className="flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 text-green-600" />
          <span className="text-sm font-semibold text-gray-900">
            Execution Complete
          </span>
          <span className="text-xs text-gray-500">({fmtMs(result.duration)})</span>
          {suggestions.length > 0 && (
            <span className="text-xs bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded font-medium">
              {suggestions.length} insight{suggestions.length > 1 ? 's' : ''}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <ExportMenu result={result} />
          <button
            onClick={() => setCollapsed(!collapsed)}
            className="flex items-center gap-1 px-2 py-1 text-xs text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors"
          >
            {collapsed ? (
              <>
                <ChevronUp className="w-3.5 h-3.5" />
                Expand
              </>
            ) : (
              <>
                <ChevronDown className="w-3.5 h-3.5" />
                Collapse
              </>
            )}
          </button>
          <button
            onClick={clearResults}
            className="p-1 text-gray-400 hover:text-gray-600"
            title="Close dashboard"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Collapsible body */}
      {!collapsed && (
        <div className="px-4 py-4 space-y-5 max-h-[45vh] overflow-y-auto">
          {/* Summary Cards */}
          <div className="grid grid-cols-4 gap-3">
            <MetricCard
              icon={<Gauge className="w-5 h-5" />}
              label="Throughput"
              value={fmtNum(m.throughput)}
              sub="ops/sec"
              iconColor="#3B82F6"
            />
            <MetricCard
              icon={<Clock className="w-5 h-5" />}
              label="Latency (p99)"
              value={fmtMs(m.latency.p99)}
              sub={`avg ${fmtMs(m.latency.avg)} · p50 ${fmtMs(m.latency.p50)}`}
              iconColor="#F59E0B"
            />
            {cacheHitRate !== null ? (
              <MetricCard
                icon={<Activity className="w-5 h-5" />}
                label="Cache Hit Rate"
                value={`${cacheHitRate}%`}
                sub={`${fmtNum(totalCacheHits)} hits · ${fmtNum(totalCacheMisses)} misses`}
                iconColor="#10B981"
              />
            ) : (
              <MetricCard
                icon={<Activity className="w-5 h-5" />}
                label="Operations"
                value={fmtNum(m.successfulOperations)}
                sub={`${m.failedOperations} failed of ${fmtNum(m.totalOperations)}`}
                iconColor="#10B981"
              />
            )}
            <MetricCard
              icon={<HardDrive className="w-5 h-5" />}
              label="I/O"
              value={fmtNum(totalPagesRead + totalPagesWritten)}
              sub={`${fmtNum(totalPagesRead)} reads · ${fmtNum(totalPagesWritten)} writes`}
              iconColor="#8B5CF6"
            />
          </div>

          {/* Latency mini-row */}
          <div className="flex gap-6 text-xs px-1">
            <span className="text-gray-500">
              avg{' '}
              <span className="font-mono text-gray-700">{fmtMs(m.latency.avg)}</span>
            </span>
            <span className="text-gray-500">
              p50{' '}
              <span className="font-mono text-gray-700">{fmtMs(m.latency.p50)}</span>
            </span>
            <span className="text-gray-500">
              p95{' '}
              <span className="font-mono text-gray-700">{fmtMs(m.latency.p95)}</span>
            </span>
            <span className="text-gray-500">
              p99{' '}
              <span className="font-mono text-gray-700">{fmtMs(m.latency.p99)}</span>
            </span>
            {m.failedOperations > 0 && (
              <span className="text-red-500 ml-auto">
                {fmtNum(m.failedOperations)} failed ops
              </span>
            )}
          </div>

          {/* Block Breakdown */}
          {sortedBlocks.length > 0 && (
            <div>
              <h4 className="text-xs font-semibold text-gray-700 uppercase tracking-wider mb-2">
                Block Breakdown
              </h4>
              <div className="space-y-0.5">
                {sortedBlocks.map((bm) => (
                  <BlockBar key={bm.blockId} bm={bm} maxPct={maxPct} />
                ))}
              </div>
            </div>
          )}

          {/* Suggestions */}
          <SuggestionsPanel suggestions={suggestions} />
        </div>
      )}
    </div>
  );
}
