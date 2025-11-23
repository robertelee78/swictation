import { useState } from 'react';

interface Correction {
  id: string;
  original: string;
  corrected: string;
  use_count: number;
  mode: string;
  match_type: string;
}

interface PatternCluster {
  cluster_id: number;
  centroid_original: string;
  centroid_corrected: string;
  members: number[];
  size: number;
}

interface ClusterVisualizationProps {
  clusters: PatternCluster[];
  corrections: Correction[];
  onBack: () => void;
}

export function ClusterVisualization({ clusters, corrections, onBack }: ClusterVisualizationProps) {
  const [selectedClusterId, setSelectedClusterId] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Create lookup map for corrections by array index
  const correctionsByIndex = corrections.reduce((map, corr, idx) => {
    map[idx] = corr;
    return map;
  }, {} as Record<number, Correction>);

  // Filter clusters by search
  const filteredClusters = searchQuery
    ? clusters.filter((c) => {
        return (
          c.centroid_original.toLowerCase().includes(searchQuery.toLowerCase()) ||
          c.centroid_corrected.toLowerCase().includes(searchQuery.toLowerCase())
        );
      })
    : clusters;

  const selectedCluster = selectedClusterId !== null
    ? clusters.find((c) => c.cluster_id === selectedClusterId)
    : null;

  // Calculate cluster statistics
  const totalPatterns = corrections.length;
  const largestCluster = clusters.reduce((max, c) => (c.size > max.size ? c : max), clusters[0]);
  const avgClusterSize = clusters.reduce((sum, c) => sum + c.size, 0) / clusters.length;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h3 className="text-foreground text-lg font-bold">Pattern Clusters</h3>
          <p className="text-muted text-xs italic mt-1">
            K-means clustering with Levenshtein distance (WASM accelerated)
          </p>
        </div>
        <button
          onClick={onBack}
          className="px-4 py-2 border border-muted text-muted rounded hover:border-primary hover:text-primary transition-colors"
        >
          ← Back to Table
        </button>
      </div>

      {/* Statistics Bar */}
      <div className="grid grid-cols-4 gap-3 mb-4">
        <StatCard label="Total Clusters" value={clusters.length} />
        <StatCard label="Total Patterns" value={totalPatterns} />
        <StatCard label="Largest Cluster" value={largestCluster?.size || 0} />
        <StatCard label="Avg Cluster Size" value={avgClusterSize.toFixed(1)} />
      </div>

      {/* Search */}
      <div className="mb-4">
        <input
          type="text"
          placeholder="Search clusters by pattern..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full bg-background text-foreground px-3 py-2 rounded border border-muted focus:border-primary focus:outline-none font-mono text-sm"
        />
      </div>

      <div className="flex-1 flex gap-4 overflow-hidden">
        {/* Cluster List */}
        <div className="w-1/3 flex flex-col overflow-hidden">
          <div className="text-muted text-xs mb-2 font-bold uppercase">
            Clusters ({filteredClusters.length})
          </div>
          <div className="flex-1 overflow-auto space-y-2">
            {filteredClusters.map((cluster) => (
              <button
                key={cluster.cluster_id}
                onClick={() => setSelectedClusterId(cluster.cluster_id)}
                className={`w-full text-left p-3 rounded border transition-colors ${
                  selectedClusterId === cluster.cluster_id
                    ? 'bg-primary/20 border-primary'
                    : 'bg-card border-border hover:border-primary/50'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-primary font-bold text-sm">
                    Cluster #{cluster.cluster_id}
                  </span>
                  <span className="text-muted text-xs bg-muted/20 px-2 py-0.5 rounded">
                    {cluster.size} patterns
                  </span>
                </div>
                <div className="text-foreground font-mono text-xs mb-1">
                  {cluster.centroid_original}
                </div>
                <div className="text-primary font-mono text-xs">
                  → {cluster.centroid_corrected}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Cluster Details */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {selectedCluster ? (
            <>
              <div className="mb-3">
                <div className="text-foreground font-bold mb-1">
                  Cluster #{selectedCluster.cluster_id} Details
                </div>
                <div className="bg-card rounded p-3 border border-primary">
                  <div className="text-xs text-muted mb-2">Centroid (most representative pattern):</div>
                  <div className="font-mono text-sm mb-1">
                    <span className="text-foreground">{selectedCluster.centroid_original}</span>
                  </div>
                  <div className="font-mono text-sm">
                    <span className="text-primary">→ {selectedCluster.centroid_corrected}</span>
                  </div>
                  <div className="mt-2 text-xs text-muted">
                    Contains {selectedCluster.size} similar pattern{selectedCluster.size !== 1 ? 's' : ''}
                  </div>
                </div>
              </div>

              <div className="text-muted text-xs mb-2 font-bold uppercase">
                Cluster Members
              </div>
              <div className="flex-1 overflow-auto">
                <table className="w-full text-sm">
                  <thead className="sticky top-0 bg-card">
                    <tr className="text-muted text-left border-b border-muted">
                      <th className="pb-2 font-medium">Original</th>
                      <th className="pb-2 font-medium">Corrected</th>
                      <th className="pb-2 font-medium">Uses</th>
                      <th className="pb-2 font-medium">Mode</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedCluster.members.map((memberIdx) => {
                      const corr = correctionsByIndex[memberIdx];
                      if (!corr) return null;

                      return (
                        <tr
                          key={corr.id}
                          className="border-b border-muted/30 hover:bg-background/50"
                        >
                          <td className="py-2 font-mono text-foreground">{corr.original}</td>
                          <td className="py-2 font-mono text-primary">{corr.corrected}</td>
                          <td className="py-2 text-muted">{corr.use_count}</td>
                          <td className="py-2">
                            <span
                              className={`px-2 py-0.5 rounded text-xs ${
                                corr.mode === 'all'
                                  ? 'bg-primary/20 text-primary'
                                  : corr.mode === 'secretary'
                                  ? 'bg-blue-500/20 text-blue-400'
                                  : 'bg-purple-500/20 text-purple-400'
                              }`}
                            >
                              {corr.mode === 'secretary' ? 'Sec' : corr.mode === 'code' ? 'Code' : 'All'}
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center text-muted">
              <div className="text-center">
                <div className="text-4xl mb-2">🎯</div>
                <div className="text-lg">Select a cluster to explore</div>
                <div className="text-sm mt-1">
                  Click any cluster to see its members
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

interface StatCardProps {
  label: string;
  value: string | number;
}

function StatCard({ label, value }: StatCardProps) {
  return (
    <div className="bg-card rounded p-3 border border-border">
      <div className="text-muted text-xs mb-1">{label}</div>
      <div className="text-primary text-xl font-bold font-mono">{value}</div>
    </div>
  );
}
