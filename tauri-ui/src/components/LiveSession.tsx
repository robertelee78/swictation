import { memo } from 'react';
import type { LiveMetrics } from '../hooks/useMetrics';

interface Props {
  metrics: LiveMetrics;
}

export const LiveSession = memo(function LiveSession({ metrics }: Props) {
  const getStateIcon = () => {
    switch (metrics.state) {
      case 'recording':
        return '🔴';
      case 'processing':
        return '🟡';
      default:
        return '🎤';
    }
  };

  const getStateColor = () => {
    switch (metrics.state) {
      case 'recording':
        return 'text-red-400';
      case 'processing':
        return 'text-yellow-400';
      default:
        return 'text-green-400';
    }
  };

  const getResourceColor = (percent: number) => {
    if (percent > 80) return 'bg-error';
    if (percent > 60) return 'bg-warning';
    return 'bg-success';
  };

  return (
    <div className="flex flex-col gap-5 p-5">
      {/* Status Header */}
      <div className="bg-card rounded-lg p-5 flex items-center justify-center gap-4">
        <span className="text-5xl">{getStateIcon()}</span>
        <span className={`text-3xl font-bold font-mono uppercase ${getStateColor()}`}>
          {metrics.state}
        </span>
      </div>

      {/* Metric Cards Grid */}
      <div className="grid grid-cols-3 gap-4">
        <MetricCard label="WPM" value={Math.round(metrics.wpm)} />
        <MetricCard label="Words" value={metrics.words} />
        <MetricCard label="Latency" value={`${(metrics.latencyMs / 1000).toFixed(2)}s`} />
        <MetricCard label="Duration" value={metrics.duration} />
        <MetricCard label="Segments" value={metrics.segments} />
        <MetricCard label="GPU Memory" value={`${(metrics.gpuMemoryMb / 1024).toFixed(1)} GB`} />
      </div>

      {/* System Resources */}
      <div className="bg-card rounded-lg p-4">
        <h3 className="text-primary text-lg font-bold mb-4">System Resources</h3>

        <div className="space-y-4">
          {/* GPU Memory Meter */}
          <div>
            <div className="text-foreground text-sm mb-2">
              GPU Memory: {metrics.gpuMemoryMb.toFixed(1)} / 8000.0 MB ({Math.round(metrics.gpuMemoryPercent)}%)
            </div>
            <div className="w-full h-6 bg-border rounded overflow-hidden">
              <div
                className={`h-full ${getResourceColor(metrics.gpuMemoryPercent)} transition-all duration-300`}
                style={{ width: `${Math.min(metrics.gpuMemoryPercent, 100)}%` }}
              />
            </div>
          </div>

          {/* CPU Usage Meter */}
          <div>
            <div className="text-foreground text-sm mb-2">
              CPU Usage: {metrics.cpuPercent.toFixed(1)} / 100.0 % ({Math.round(metrics.cpuPercent)}%)
            </div>
            <div className="w-full h-6 bg-border rounded overflow-hidden">
              <div
                className={`h-full ${getResourceColor(metrics.cpuPercent)} transition-all duration-300`}
                style={{ width: `${Math.min(metrics.cpuPercent, 100)}%` }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});

interface MetricCardProps {
  label: string;
  value: string | number;
}

function MetricCard({ label, value }: MetricCardProps) {
  return (
    <div className="bg-card rounded-lg border border-border p-4 flex flex-col items-center justify-center gap-1">
      <span className="text-muted text-xs">{label}</span>
      <span className="text-foreground text-3xl font-bold font-mono">{value}</span>
    </div>
  );
}
