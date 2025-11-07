import { useDatabase } from '../hooks/useDatabase';

export function History() {
  const { history, lifetimeStats, loading, refresh } = useDatabase();

  return (
    <div className="flex flex-col gap-4 p-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-foreground text-xl font-bold">Recent Sessions (Last 10)</h2>
        <button
          onClick={refresh}
          disabled={loading}
          className="px-4 py-2 bg-card hover:bg-border text-foreground rounded border border-primary transition-colors disabled:opacity-50"
        >
          🔄 Refresh
        </button>
      </div>

      {/* Sessions List */}
      <div className="space-y-3 flex-1 overflow-auto">
        {history.map((session) => (
          <div
            key={session.id}
            className="bg-card rounded border border-border p-4 flex items-center gap-5"
          >
            <div className="text-primary font-bold text-lg">#{session.id}</div>

            <div className="flex-1">
              <div className="text-foreground text-sm mb-1">
                {new Date(session.start_time * 1000).toLocaleString()}
              </div>
              <div className="flex gap-4">
                <span className="text-success text-xs">{session.words_dictated} words</span>
                <span className="text-primary text-xs">{Math.round(session.wpm)} WPM</span>
                <span className="text-warning text-xs">{session.avg_latency_ms.toFixed(0)}ms</span>
              </div>
            </div>

            <div className="text-muted text-sm">
              {(session.duration_s / 60).toFixed(1)}m
            </div>
          </div>
        ))}
      </div>

      {/* Lifetime Stats */}
      {lifetimeStats && (
        <div className="bg-card rounded-lg p-4">
          <h3 className="text-primary text-lg font-bold mb-3">Lifetime Stats</h3>
          <div className="grid grid-cols-2 gap-x-8 gap-y-2">
            <StatRow label="Total Words" value={lifetimeStats.total_words} />
            <StatRow label="Total Sessions" value={lifetimeStats.total_sessions} />
            <StatRow label="Avg WPM" value={Math.round(lifetimeStats.average_wpm)} />
            <StatRow
              label="Time Saved"
              value={`${(lifetimeStats.estimated_time_saved_minutes / 60).toFixed(1)}h`}
            />
            <StatRow label="Best WPM" value={Math.round(lifetimeStats.best_wpm_value)} />
            <StatRow
              label="Lowest Latency"
              value={`${Math.round(lifetimeStats.lowest_latency_ms)}ms`}
            />
          </div>
        </div>
      )}
    </div>
  );
}

interface StatRowProps {
  label: string;
  value: string | number;
}

function StatRow({ label, value }: StatRowProps) {
  return (
    <div className="flex gap-3">
      <span className="text-muted text-sm min-w-[140px]">{label}:</span>
      <span className="text-foreground text-sm font-bold">{value}</span>
    </div>
  );
}
