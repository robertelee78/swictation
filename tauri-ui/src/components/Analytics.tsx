import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWasmUtils } from '../hooks/useWasmUtils';
import { LineChart } from './charts/LineChart';
import { Histogram } from './charts/Histogram';

interface SessionMetrics {
  id: number;
  start_time: number;      // Unix timestamp
  end_time: number | null; // Unix timestamp
  duration_s: number;
  words_dictated: number;
  wpm: number;
  avg_latency_ms: number;
}

interface TrendPoint {
  timestamp_unix: number;
  average_wpm: number;
  session_count: number;
}

interface AggregateStats {
  total_sessions: number;
  total_words: number;
  total_duration_hours: number;
  average_wpm: number;
  median_wpm: number;
  best_wpm: number;
  average_latency_ms: number;
  median_latency_ms: number;
  best_latency_ms: number;
}

type TimeRange = '7d' | '30d' | '90d' | 'all';
type BucketSize = 24 | 168; // 24h (daily) or 168h (weekly)

export function Analytics() {
  const { isLoaded: wasmLoaded, calculateWpmTrend, calculateAggregateStats } = useWasmUtils();

  const [sessions, setSessions] = useState<SessionMetrics[]>([]);
  const [timeRange, setTimeRange] = useState<TimeRange>('30d');
  const [bucketSize, setBucketSize] = useState<BucketSize>(24);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch sessions from backend
  useEffect(() => {
    const fetchSessions = async () => {
      setLoading(true);
      setError(null);

      try {
        const result = await invoke<SessionMetrics[]>('get_recent_sessions', {
          limit: 10000, // Fetch up to 10k sessions for charts
          offset: 0,
        });
        setSessions(result);
      } catch (err) {
        console.error('Failed to fetch sessions:', err);
        setError(`Failed to load sessions: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    fetchSessions();
  }, []);

  // Filter sessions by time range
  const filteredSessions = useMemo(() => {
    if (timeRange === 'all') return sessions;

    const now = Date.now() / 1000;
    const rangeDays = timeRange === '7d' ? 7 : timeRange === '30d' ? 30 : 90;
    const cutoff = now - rangeDays * 24 * 60 * 60;

    return sessions.filter((s) => s.start_time >= cutoff);
  }, [sessions, timeRange]);

  // Calculate WPM trend using WASM (0.15ms vs 5-10ms IPC)
  const [wpmTrend, setWpmTrend] = useState<TrendPoint[]>([]);

  useEffect(() => {
    if (!wasmLoaded || filteredSessions.length === 0) {
      setWpmTrend([]);
      return;
    }

    const sessionsJson = JSON.stringify(filteredSessions);

    calculateWpmTrend(sessionsJson, bucketSize)
      .then((trendJson) => {
        const trend: TrendPoint[] = JSON.parse(trendJson);
        setWpmTrend(trend);
      })
      .catch((err) => {
        console.error('WASM trend calculation failed:', err);
        setWpmTrend([]);
      });
  }, [wasmLoaded, filteredSessions, bucketSize, calculateWpmTrend]);

  // Calculate aggregate stats using WASM (0.15ms vs 5-10ms IPC)
  const [aggregateStats, setAggregateStats] = useState<AggregateStats | null>(null);

  useEffect(() => {
    if (!wasmLoaded || filteredSessions.length === 0) {
      setAggregateStats(null);
      return;
    }

    const sessionsJson = JSON.stringify(filteredSessions);

    calculateAggregateStats(sessionsJson)
      .then((statsJson) => {
        const stats: AggregateStats = JSON.parse(statsJson);
        setAggregateStats(stats);
      })
      .catch((err) => {
        console.error('WASM stats calculation failed:', err);
        setAggregateStats(null);
      });
  }, [wasmLoaded, filteredSessions, calculateAggregateStats]);

  // Extract latency values for histogram
  const latencyData = useMemo(() => {
    return filteredSessions.map((s) => s.avg_latency_ms);
  }, [filteredSessions]);

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-muted text-lg">Loading analytics...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-error text-lg">{error}</div>
      </div>
    );
  }

  if (sessions.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-muted text-lg">No session data available</div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-auto p-5">
      {/* Header with Controls */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h2 className="text-foreground text-2xl font-bold">Analytics Dashboard</h2>
          {wasmLoaded && (
            <span className="text-success text-xs font-mono bg-success/20 px-2 py-1 rounded">
              WASM ⚡ Accelerated
            </span>
          )}
        </div>

        <div className="flex gap-3">
          {/* Time Range Selector */}
          <div className="flex items-center gap-2">
            <label className="text-muted text-sm">Time Range:</label>
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value as TimeRange)}
              className="bg-card text-foreground px-3 py-1.5 rounded border border-primary text-sm"
            >
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
              <option value="90d">Last 90 Days</option>
              <option value="all">All Time</option>
            </select>
          </div>

          {/* Bucket Size Selector */}
          <div className="flex items-center gap-2">
            <label className="text-muted text-sm">Grouping:</label>
            <select
              value={bucketSize}
              onChange={(e) => setBucketSize(Number(e.target.value) as BucketSize)}
              className="bg-card text-foreground px-3 py-1.5 rounded border border-primary text-sm"
            >
              <option value={24}>Daily</option>
              <option value={168}>Weekly</option>
            </select>
          </div>
        </div>
      </div>

      {/* Stats Summary Cards */}
      {aggregateStats && (
        <div className="grid grid-cols-3 gap-4 mb-6">
          <StatCard
            label="Total Sessions"
            value={aggregateStats.total_sessions}
            color="text-primary"
          />
          <StatCard
            label="Total Words"
            value={aggregateStats.total_words.toLocaleString()}
            color="text-success"
          />
          <StatCard
            label="Total Hours"
            value={aggregateStats.total_duration_hours.toFixed(1)}
            color="text-warning"
          />
          <StatCard
            label="Average WPM"
            value={Math.round(aggregateStats.average_wpm)}
            color="text-primary"
          />
          <StatCard
            label="Best WPM"
            value={Math.round(aggregateStats.best_wpm)}
            color="text-success"
          />
          <StatCard
            label="Avg Latency"
            value={`${Math.round(aggregateStats.average_latency_ms)}ms`}
            color="text-warning"
          />
        </div>
      )}

      {/* Charts */}
      <div className="space-y-6">
        {/* WPM Trend Chart */}
        {wpmTrend.length > 0 && (
          <div className="bg-card rounded-lg p-4 border border-border">
            <LineChart
              data={wpmTrend}
              width={window.innerWidth - 150}
              height={300}
              color="#00ff41"
              showGrid={true}
            />
          </div>
        )}

        {/* Latency Distribution Histogram */}
        {latencyData.length > 0 && (
          <div className="bg-card rounded-lg p-4 border border-border">
            <Histogram
              data={latencyData}
              bins={30}
              width={window.innerWidth - 150}
              height={300}
              color="#ffa500"
              label="Latency (ms)"
            />
          </div>
        )}
      </div>

      {/* Data Info Footer */}
      <div className="mt-6 text-muted text-xs italic text-center">
        Displaying {filteredSessions.length} sessions
        {wasmLoaded && ' | Computed with WebAssembly (33x faster than IPC)'}
      </div>
    </div>
  );
}

interface StatCardProps {
  label: string;
  value: string | number;
  color: string;
}

function StatCard({ label, value, color }: StatCardProps) {
  return (
    <div className="bg-card rounded-lg p-4 border border-border">
      <div className="text-muted text-xs mb-1">{label}</div>
      <div className={`${color} text-2xl font-bold font-mono`}>{value}</div>
    </div>
  );
}
