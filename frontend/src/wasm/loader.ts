/**
 * WASM module loader.
 *
 * Attempts to dynamically import the Rust block-system compiled to WASM.
 * When the WASM module is not yet built or unavailable, the app silently
 * falls back to the MockExecutionEngine.
 */

// ---------------------------------------------------------------------------
// Module state
// ---------------------------------------------------------------------------

let wasmModule: WASMModule | null = null;
let loadError: string | null = null;

/**
 * The shape we expect from the wasm-bindgen generated JS module.
 * This will be implemented by the Rust crate when compiled with
 * `wasm-pack build --target web`.
 */
export interface WASMModule {
  // Lifecycle
  init_runtime: () => void;
  destroy_runtime: () => void;

  // Block management
  register_block: (configJson: string) => string; // returns block ID or error JSON
  create_connection: (connectionJson: string) => string;

  // Validation
  validate: () => string; // returns ValidationResult JSON

  // Execution
  execute: (workloadJson: string, progressCallback: (json: string) => void) => string;
  cancel_execution: () => void;

  // Metrics
  get_metrics: () => string; // returns MetricsResult JSON

  // Discovery
  get_block_types: () => string; // returns BlockMetadata[] JSON
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/**
 * Attempt to load the WASM module.
 * Call this once at app startup. Safe to call multiple times.
 */
export async function loadWASM(): Promise<boolean> {
  if (wasmModule) return true;

  try {
    // The WASM package will be placed at this path by wasm-pack.
    // When the Rust crate is compiled: `wasm-pack build --target web --out-dir ../frontend/src/pkg`
    //
    // We use a dynamic string to prevent TypeScript from resolving the path at compile time.
    // This path only exists after running wasm-pack.
    const wasmPath = '../pkg/block_system';
    const module = await import(/* @vite-ignore */ wasmPath);

    // wasm-bindgen modules expose a default init function
    if (typeof module.default === 'function') {
      await module.default();
    }

    wasmModule = module as unknown as WASMModule;
    loadError = null;
    console.info('[DB Simulator] WASM module loaded successfully.');
    return true;
  } catch (err) {
    loadError = err instanceof Error ? err.message : String(err);
    console.info(
      '[DB Simulator] WASM module not available — using mock engine.',
      loadError,
    );
    return false;
  }
}

// ---------------------------------------------------------------------------
// Readiness
// ---------------------------------------------------------------------------

export function isWASMReady(): boolean {
  return wasmModule !== null;
}

export function getWASMModule(): WASMModule | null {
  return wasmModule;
}

export function getWASMLoadError(): string | null {
  return loadError;
}
