import { useState } from 'react';
import { FixedSizeList as List } from 'react-window';
import InfiniteLoader from 'react-window-infinite-loader';
import { useDatabase } from '../hooks/useDatabase';

export function History() {
  const { history, totalCount, lifetimeStats, loading, refresh, resetDatabase, loadMoreSessions } = useDatabase();
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  const handleReset = async () => {
    await resetDatabase();
    setShowResetConfirm(false);
  };

  // Check if item is loaded
  const isItemLoaded = (index: number) => index < history.length && !!history[index];

  // Render individual session row
  const SessionRow = ({ index, style }: { index: number; style: React.CSSProperties }) => {
    const session = history[index];

    if (!session) {
      return (
        <div style={style} className="flex items-center justify-center">
          <div className="text-muted text-sm">Loading...</div>
        </div>
      );
    }

    // Calculate sequential session number (newest = #1, oldest = #totalCount)
    const sessionNumber = totalCount - index;

    return (
      <div style={style} className="px-5 py-1.5">
        <div className="bg-card rounded border border-border p-4 flex items-center gap-5">
          <div className="text-primary font-bold text-lg">#{sessionNumber}</div>

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
      </div>
    );
  };

  return (
    <div className="flex flex-col gap-4 p-5 h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-foreground text-xl font-bold">
          All Sessions ({totalCount})
        </h2>
        <div className="flex gap-2">
          <button
            onClick={refresh}
            disabled={loading}
            className="px-4 py-2 bg-card hover:bg-border text-foreground rounded border border-primary transition-colors disabled:opacity-50"
          >
            Refresh
          </button>
          <button
            onClick={() => setShowResetConfirm(true)}
            disabled={loading}
            className="px-4 py-2 bg-card hover:bg-destructive text-foreground rounded border border-destructive transition-colors disabled:opacity-50"
          >
            Reset
          </button>
        </div>
      </div>

      {/* Reset Confirmation Dialog */}
      {showResetConfirm && (
        <div className="bg-card rounded border border-destructive p-4">
          <p className="text-foreground mb-3">Are you sure you want to reset all data? This cannot be undone.</p>
          <div className="flex gap-2">
            <button
              onClick={handleReset}
              disabled={loading}
              className="px-4 py-2 bg-destructive hover:bg-destructive/80 text-white rounded transition-colors disabled:opacity-50"
            >
              Yes, Reset All Data
            </button>
            <button
              onClick={() => setShowResetConfirm(false)}
              className="px-4 py-2 bg-card hover:bg-border text-foreground rounded border border-border transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Virtualized Sessions List */}
      <div className="flex-1 min-h-0">
        {totalCount > 0 ? (
          <InfiniteLoader
            isItemLoaded={isItemLoaded}
            itemCount={totalCount}
            loadMoreItems={loadMoreSessions}
            threshold={15}
          >
            {({ onItemsRendered, ref }) => (
              <List
                height={window.innerHeight - 350}
                itemCount={totalCount}
                itemSize={100}
                onItemsRendered={onItemsRendered}
                ref={ref}
                width="100%"
              >
                {SessionRow}
              </List>
            )}
          </InfiniteLoader>
        ) : (
          <div className="flex items-center justify-center h-full text-muted">
            {loading ? 'Loading sessions...' : 'No sessions yet'}
          </div>
        )}
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
