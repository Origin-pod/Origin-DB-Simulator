// ---------------------------------------------------------------------------
// AI Context Builder — serializes design, metrics, and block docs
// into a compact system prompt for the teaching assistant.
// ---------------------------------------------------------------------------

import type { Node, Edge } from '@xyflow/react';
import type { BlockNodeData } from '@/types';
import type { ExecutionResult } from '@/engine/types';
import type { Insight } from '@/engine/insights';
import { getBlockDefinition } from '@/types/blocks';

/**
 * Serialize the current canvas design into a compact text representation.
 */
export function buildDesignContext(
  nodes: Node<BlockNodeData>[],
  edges: Edge[],
): string {
  if (nodes.length === 0) return 'No blocks on canvas.';

  const blocks = nodes.map((n) => {
    const d = n.data as BlockNodeData;
    const params = Object.entries(d.parameters)
      .filter(([, v]) => v !== '' && v !== false)
      .map(([k, v]) => `${k}=${v}`)
      .join(', ');
    return `  - ${d.label} (${d.blockType})${params ? ` [${params}]` : ''}`;
  });

  const connections = edges.map((e) => {
    const src = nodes.find((n) => n.id === e.source);
    const tgt = nodes.find((n) => n.id === e.target);
    const srcLabel = src ? (src.data as BlockNodeData).label : e.source;
    const tgtLabel = tgt ? (tgt.data as BlockNodeData).label : e.target;
    return `  - ${srcLabel} → ${tgtLabel}`;
  });

  return [
    'Current database design:',
    'Blocks:',
    ...blocks,
    connections.length > 0 ? 'Connections:' : '',
    ...connections,
  ]
    .filter(Boolean)
    .join('\n');
}

/**
 * Serialize execution results into a compact text representation.
 */
export function buildMetricsContext(
  result: ExecutionResult,
  insights: Insight[],
): string {
  const m = result.metrics;
  const lines = [
    'Execution results:',
    `  Throughput: ${fmt(m.throughput)} ops/sec`,
    `  Latency: avg=${fmtMs(m.latency.avg)}, p50=${fmtMs(m.latency.p50)}, p95=${fmtMs(m.latency.p95)}, p99=${fmtMs(m.latency.p99)}`,
    `  Operations: ${fmt(m.successfulOperations)} successful, ${m.failedOperations} failed`,
    `  Duration: ${fmtMs(result.duration)}`,
    '',
    'Block breakdown (sorted by time):',
  ];

  const sorted = [...result.blockMetrics].sort(
    (a, b) => b.percentage - a.percentage,
  );
  for (const bm of sorted) {
    const counters = Object.entries(bm.counters)
      .map(([k, v]) => `${k}=${fmt(v)}`)
      .join(', ');
    lines.push(
      `  - ${bm.blockName} (${bm.blockType}): ${bm.percentage.toFixed(1)}% / ${fmtMs(bm.executionTime)}${counters ? ` {${counters}}` : ''}`,
    );
  }

  if (insights.length > 0) {
    lines.push('', 'Identified insights:');
    for (const insight of insights) {
      lines.push(`  - [${insight.severity}] ${insight.title}: ${insight.explanation}`);
    }
  }

  return lines.join('\n');
}

/**
 * Pull relevant documentation from the BLOCK_REGISTRY for blocks
 * in the current design.
 */
export function buildBlockDocsContext(
  nodes: Node<BlockNodeData>[],
): string {
  const seen = new Set<string>();
  const lines: string[] = ['Block documentation for blocks in this design:'];

  for (const n of nodes) {
    const bt = (n.data as BlockNodeData).blockType;
    if (seen.has(bt)) continue;
    seen.add(bt);

    const def = getBlockDefinition(bt);
    if (!def) continue;
    const doc = def.documentation;

    lines.push(`\n## ${def.name} (${def.type})`);
    if (doc?.overview) lines.push(`Overview: ${doc.overview}`);
    if (doc?.algorithm) lines.push(`Algorithm: ${doc.algorithm}`);
    if (doc?.complexity) {
      lines.push(`Complexity: Time=${doc.complexity.time}, Space=${doc.complexity.space}`);
    }
    if (doc?.tradeoffs && doc.tradeoffs.length > 0) {
      lines.push(`Tradeoffs: ${doc.tradeoffs.join('; ')}`);
    }
    if (doc?.useCases && doc.useCases.length > 0) {
      lines.push(`Use cases: ${doc.useCases.join('; ')}`);
    }
    if (doc?.examples && doc.examples.length > 0) {
      lines.push(`Real-world: ${doc.examples.join('; ')}`);
    }
  }

  return lines.join('\n');
}

/**
 * Assemble the full system prompt with all context.
 */
export function buildSystemPrompt(
  nodes: Node<BlockNodeData>[],
  edges: Edge[],
  result: ExecutionResult | null,
  insights: Insight[],
): string {
  const parts = [
    `You are a database internals teaching assistant inside the DB Simulator app.
The user is learning how databases work by building database architectures from modular blocks and running workloads against them.

Your role:
- Explain database concepts through the lens of the user's current design and execution results
- Be Socratic when possible — ask the user what they think before giving answers
- Connect metrics to the underlying algorithms and data structures
- Reference real databases (PostgreSQL, MySQL, SQLite, RocksDB, Cassandra) to ground concepts
- Keep responses concise (2-4 paragraphs) unless the user asks for more detail
- Use plain language, but don't shy away from technical terms — just explain them

Important: The user can see their design and metrics. Don't repeat raw numbers — instead, explain what they mean and why they matter.`,
    '',
    buildDesignContext(nodes, edges),
  ];

  if (result) {
    parts.push('', buildMetricsContext(result, insights));
  }

  parts.push('', buildBlockDocsContext(nodes));

  return parts.join('\n');
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(Math.round(n));
}

function fmtMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(2)}ms`;
}
