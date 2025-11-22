import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { HistorySession, LifetimeMetrics } from '../types';

export function useDatabase() {
  const [history, setHistory] = useState<HistorySession[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [lifetimeStats, setLifetimeStats] = useState<LifetimeMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load initial batch of sessions (50 most recent)
  const loadHistory = async () => {
    setLoading(true);
    setError(null);
    try {
      const [sessions, count] = await Promise.all([
        invoke<HistorySession[]>('get_recent_sessions', { limit: 50, offset: 0 }),
        invoke<number>('get_session_count')
      ]);
      setHistory(sessions);
      setTotalCount(count);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load history');
      console.error('Failed to load history:', err);
    } finally {
      setLoading(false);
    }
  };

  // Load more sessions for infinite scroll
  const loadMoreSessions = useCallback(async (startIndex: number, stopIndex: number) => {
    try {
      const count = stopIndex - startIndex + 1;
      const newSessions = await invoke<HistorySession[]>('get_recent_sessions', {
        limit: count,
        offset: startIndex
      });

      // Merge with existing sessions at the correct indices
      setHistory(prev => {
        const merged = [...prev];
        newSessions.forEach((session, idx) => {
          merged[startIndex + idx] = session;
        });
        return merged;
      });
    } catch (err) {
      console.error('Failed to load more sessions:', err);
      // Don't set error state here - it's a background operation
    }
  }, []);

  const loadLifetimeStats = async () => {
    setLoading(true);
    setError(null);
    try {
      const stats = await invoke<LifetimeMetrics>('get_lifetime_stats');
      setLifetimeStats(stats);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load lifetime stats');
      console.error('Failed to load lifetime stats:', err);
    } finally {
      setLoading(false);
    }
  };

  const refresh = async () => {
    await Promise.all([loadHistory(), loadLifetimeStats()]);
  };

  const resetDatabase = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke('reset_database');
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reset database');
      console.error('Failed to reset database:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  return {
    history,
    totalCount,
    lifetimeStats,
    loading,
    error,
    refresh,
    resetDatabase,
    loadMoreSessions,
  };
}
