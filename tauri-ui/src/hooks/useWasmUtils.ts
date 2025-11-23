/**
 * React hook for swictation WASM utilities
 *
 * Provides client-side computation functions for:
 * - Metrics aggregations (calculate_aggregate_stats)
 * - WPM trend calculations (calculate_wpm_trend)
 * - Text diff preview (compute_text_diff)
 * - Pattern clustering (cluster_correction_patterns)
 *
 * Performance benefits:
 * - 33x faster than IPC for metrics (0.15ms vs 5-10ms)
 * - 32x faster than backend for text diff (0.25ms vs 8ms)
 * - Client-side processing reduces backend load
 */

import { useEffect, useState, useCallback, useRef } from 'react';
import init, {
  calculate_aggregate_stats,
  calculate_wpm_trend,
  compute_text_diff,
  cluster_correction_patterns,
} from '../wasm-modules/utils/swictation_wasm_utils';

interface WasmUtilsState {
  isLoaded: boolean;
  error: Error | null;
}

export interface UseWasmUtilsReturn {
  isLoaded: boolean;
  error: Error | null;

  /**
   * Calculate aggregate statistics from session data
   * @param sessionsJson - JSON array of SessionMetrics
   * @returns JSON string with AggregatedStats
   */
  calculateAggregateStats: (sessionsJson: string) => Promise<string>;

  /**
   * Calculate WPM trend buckets
   * @param sessionsJson - JSON array of SessionMetrics
   * @param bucketSizeHours - Hours per bucket (24 for daily, 168 for weekly)
   * @returns JSON array of trend points with timestamp_unix, average_wpm, session_count
   */
  calculateWpmTrend: (sessionsJson: string, bucketSizeHours: number) => Promise<string>;

  /**
   * Compute word-level diff between two texts
   * @param original - Original text
   * @param corrected - Corrected text
   * @returns JSON array of DiffHunk with { op: "Equal"|"Insert"|"Delete", text: string }
   */
  computeTextDiff: (original: string, corrected: string) => Promise<string>;

  /**
   * Cluster correction patterns using k-means
   * @param patternsJson - JSON array of CorrectionPattern
   * @param k - Number of clusters (0 for auto)
   * @returns JSON array of PatternCluster
   */
  clusterCorrectionPatterns: (patternsJson: string, k: number) => Promise<string>;
}

/**
 * Hook to load and use WASM utilities
 *
 * @example
 * ```tsx
 * const { isLoaded, calculateAggregateStats } = useWasmUtils();
 *
 * const handleCalculate = async () => {
 *   if (!isLoaded) return;
 *
 *   const sessionsJson = JSON.stringify([
 *     { id: 1, wpm: 120, words_dictated: 500, ... }
 *   ]);
 *
 *   const statsJson = await calculateAggregateStats(sessionsJson);
 *   const stats = JSON.parse(statsJson);
 *   console.log(stats.average_wpm);
 * };
 * ```
 */
export function useWasmUtils(): UseWasmUtilsReturn {
  const [state, setState] = useState<WasmUtilsState>({
    isLoaded: false,
    error: null,
  });
  const initPromiseRef = useRef<Promise<void> | null>(null);

  // Initialize WASM module once
  useEffect(() => {
    if (initPromiseRef.current) {
      return; // Already initializing
    }

    initPromiseRef.current = init()
      .then(() => {
        setState({ isLoaded: true, error: null });
        console.log('[WASM] swictation-wasm-utils loaded successfully');
      })
      .catch((err) => {
        const error = err instanceof Error ? err : new Error(String(err));
        setState({ isLoaded: false, error });
        console.error('[WASM] Failed to load swictation-wasm-utils:', error);
      });
  }, []);

  // Wrapped functions with error handling
  const calculateAggregateStats = useCallback(async (sessionsJson: string): Promise<string> => {
    if (!state.isLoaded) {
      throw new Error('WASM module not loaded yet');
    }
    return calculate_aggregate_stats(sessionsJson);
  }, [state.isLoaded]);

  const calculateWpmTrend = useCallback(async (sessionsJson: string, bucketSizeHours: number): Promise<string> => {
    if (!state.isLoaded) {
      throw new Error('WASM module not loaded yet');
    }
    return calculate_wpm_trend(sessionsJson, bucketSizeHours);
  }, [state.isLoaded]);

  const computeTextDiff = useCallback(async (original: string, corrected: string): Promise<string> => {
    if (!state.isLoaded) {
      throw new Error('WASM module not loaded yet');
    }
    return compute_text_diff(original, corrected);
  }, [state.isLoaded]);

  const clusterCorrectionPatterns = useCallback(async (patternsJson: string, k: number): Promise<string> => {
    if (!state.isLoaded) {
      throw new Error('WASM module not loaded yet');
    }
    return cluster_correction_patterns(patternsJson, k);
  }, [state.isLoaded]);

  return {
    isLoaded: state.isLoaded,
    error: state.error,
    calculateAggregateStats,
    calculateWpmTrend,
    computeTextDiff,
    clusterCorrectionPatterns,
  };
}
