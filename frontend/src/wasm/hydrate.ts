/**
 * Registry hydration — enriches the static BLOCK_REGISTRY with rich
 * educational metadata fetched from the WASM module at startup.
 *
 * Call once after WASM loads. Falls back gracefully if WASM is unavailable.
 */

import { getWASMBridge } from './bridge';
import { isWASMReady } from './loader';
import { BLOCK_REGISTRY } from '@/types/blocks';
import type { BlockReference, BlockMetricInfo } from '@/types/blocks';
import type { WASMBlockDetail } from './types';

/**
 * Fetch all block details from WASM and merge rich documentation
 * into the existing static BLOCK_REGISTRY entries in-place.
 *
 * Returns true if hydration succeeded.
 */
export function hydrateBlockRegistry(): boolean {
  if (!isWASMReady()) return false;

  try {
    const bridge = getWASMBridge();
    const details = bridge.getAllBlockDetails();

    // Build a lookup map: blockType -> WASMBlockDetail
    const detailMap = new Map<string, WASMBlockDetail>();
    for (const d of details) {
      detailMap.set(d.blockType, d);
    }

    // Merge into existing registry entries
    for (const blockDef of BLOCK_REGISTRY) {
      const detail = detailMap.get(blockDef.type);
      if (!detail) continue;

      // Enrich documentation
      blockDef.documentation = {
        ...blockDef.documentation,
        summary: blockDef.documentation?.summary ?? detail.description,
        details: blockDef.documentation?.details,
        overview: detail.documentation.overview,
        algorithm: detail.documentation.algorithm,
        complexity: detail.documentation.complexity,
        useCases: detail.documentation.use_cases,
        tradeoffs: detail.documentation.tradeoffs,
        examples: detail.documentation.examples,
      };

      // Add references
      blockDef.references = detail.references.map(
        (r): BlockReference => ({
          refType: r.refType as BlockReference['refType'],
          title: r.title,
          url: r.url ?? undefined,
          citation: r.citation ?? undefined,
        }),
      );

      // Add metric definitions
      blockDef.metricDefinitions = detail.metrics.map(
        (m): BlockMetricInfo => ({
          id: m.id,
          name: m.name,
          type: m.metricType,
          unit: m.unit,
          description: m.description,
        }),
      );
    }

    console.info(
      `[DB Simulator] Block registry hydrated with WASM documentation (${details.length} blocks).`,
    );
    return true;
  } catch (err) {
    console.warn('[DB Simulator] Failed to hydrate block registry:', err);
    return false;
  }
}
