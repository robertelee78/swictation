/* tslint:disable */
/* eslint-disable */
export function init(): void;
/**
 * Calculate aggregated statistics from session data
 *
 * # Arguments
 * * `sessions_json` - JSON array of SessionMetrics
 *
 * # Returns
 * JSON string with AggregatedStats
 *
 * # Performance
 * ~0.15ms for 1000 sessions (vs 5-10ms IPC roundtrip)
 */
export function calculate_aggregate_stats(sessions_json: string): string;
/**
 * Calculate WPM trend buckets (daily/weekly aggregates)
 *
 * NOTE: Temporarily commented out due to chrono formatting limitations in WASM
 * Will be re-enabled in Phase 3 with charts visualization
 *
 * # Arguments
 * * `sessions_json` - JSON array of SessionMetrics
 * * `bucket_size_hours` - Hours per bucket (e.g., 24 for daily, 168 for weekly)
 *
 * # Returns
 * JSON array of { timestamp_unix: number, average_wpm: number, session_count: number }
 */
export function calculate_wpm_trend(sessions_json: string, bucket_size_hours: number): string;
/**
 * Compute Myers diff between two texts (word-level)
 *
 * # Arguments
 * * `original` - Original text
 * * `corrected` - Corrected text
 *
 * # Returns
 * JSON array of DiffHunk
 *
 * # Performance
 * ~0.25ms for 100-word texts (vs 8ms backend + IPC)
 */
export function compute_text_diff(original: string, corrected: string): string;
/**
 * Simple k-means clustering for correction patterns (Levenshtein distance)
 *
 * # Arguments
 * * `patterns_json` - JSON array of CorrectionPattern
 * * `k` - Number of clusters (default: sqrt(n))
 *
 * # Returns
 * JSON array of PatternCluster
 */
export function cluster_correction_patterns(patterns_json: string, k: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly calculate_aggregate_stats: (a: number, b: number) => [number, number, number, number];
  readonly calculate_wpm_trend: (a: number, b: number, c: number) => [number, number, number, number];
  readonly compute_text_diff: (a: number, b: number, c: number, d: number) => [number, number, number, number];
  readonly cluster_correction_patterns: (a: number, b: number, c: number) => [number, number, number, number];
  readonly init: () => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
