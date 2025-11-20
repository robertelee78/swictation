import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { HistorySession, LifetimeMetrics } from '../types';

export function useDatabase() {
  const [history, setHistory] = useState<HistorySession[]>([]);
  const [lifetimeStats, setLifetimeStats] = useState<LifetimeMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadHistory = async () => {
    setLoading(true);
    setError(null);
    try {
      const sessions = await invoke<HistorySession[]>('get_recent_sessions', { limit: 10 });
      setHistory(sessions);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load history');
      console.error('Failed to load history:', err);
    } finally {
      setLoading(false);
    }
  };

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

  useEffect(() => {
    refresh();
  }, []);

  return {
    history,
    lifetimeStats,
    loading,
    error,
    refresh,
  };
}
