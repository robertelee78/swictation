import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Correction {
  id: string;
  original: string;
  corrected: string;
  mode: string;
  match_type: string;
  learned_at: string;
  use_count: number;
}

export function LearnedPatterns() {
  const [corrections, setCorrections] = useState<Correction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<string>('all');
  const [search, setSearch] = useState<string>('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<{ corrected: string; mode: string; matchType: string }>({
    corrected: '',
    mode: 'all',
    matchType: 'exact',
  });

  const loadCorrections = async () => {
    try {
      setLoading(true);
      const result = await invoke<Correction[]>('get_corrections');
      setCorrections(result);
      setError(null);
    } catch (err) {
      setError(`Failed to load corrections: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCorrections();
  }, []);

  const deleteCorrection = async (id: string) => {
    if (!confirm('Delete this correction pattern?')) return;

    try {
      await invoke('delete_correction', { id });
      await loadCorrections();
    } catch (err) {
      alert(`Failed to delete: ${err}`);
    }
  };

  const startEdit = (correction: Correction) => {
    setEditingId(correction.id);
    setEditForm({
      corrected: correction.corrected,
      mode: correction.mode,
      matchType: correction.match_type,
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditForm({ corrected: '', mode: 'all', matchType: 'exact' });
  };

  const saveEdit = async (id: string) => {
    try {
      await invoke('update_correction', {
        id,
        corrected: editForm.corrected,
        mode: editForm.mode,
        matchType: editForm.matchType,
      });
      await loadCorrections();
      setEditingId(null);
    } catch (err) {
      alert(`Failed to update: ${err}`);
    }
  };

  // Filter and search
  const filteredCorrections = corrections.filter((c) => {
    const matchesFilter = filter === 'all' || c.mode === filter;
    const matchesSearch =
      search === '' ||
      c.original.toLowerCase().includes(search.toLowerCase()) ||
      c.corrected.toLowerCase().includes(search.toLowerCase());
    return matchesFilter && matchesSearch;
  });

  const getModeLabel = (mode: string) => {
    switch (mode) {
      case 'secretary':
        return 'Secretary';
      case 'code':
        return 'Code';
      default:
        return 'All';
    }
  };

  const getMatchTypeLabel = (matchType: string) => {
    return matchType === 'phonetic' ? 'Phonetic' : 'Exact';
  };

  return (
    <div className="h-full bg-card rounded-lg m-5 p-4 flex flex-col">
      {/* Header */}
      <div className="mb-4">
        <h2 className="text-primary text-xl font-bold">Learned Patterns</h2>
        <p className="text-muted text-xs italic mt-1">
          Corrections stored in ~/.config/swictation/corrections.toml
        </p>
      </div>

      {/* Controls */}
      <div className="flex gap-3 mb-4">
        <input
          type="text"
          placeholder="Search patterns..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 bg-background text-foreground px-3 py-2 rounded border border-muted focus:border-primary focus:outline-none font-mono text-sm"
        />

        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="bg-background text-foreground px-3 py-2 rounded border border-muted focus:border-primary"
        >
          <option value="all">All Modes</option>
          <option value="secretary">Secretary</option>
          <option value="code">Code</option>
        </select>

        <button
          onClick={loadCorrections}
          className="px-4 py-2 border border-primary text-primary rounded hover:bg-primary hover:text-background transition-colors"
        >
          Refresh
        </button>
      </div>

      {/* Error State */}
      {error && (
        <div className="mb-4 px-3 py-2 bg-error/20 text-error rounded text-sm">
          {error}
        </div>
      )}

      {/* Loading State */}
      {loading && (
        <div className="flex-1 flex items-center justify-center text-muted">
          Loading corrections...
        </div>
      )}

      {/* Empty State */}
      {!loading && filteredCorrections.length === 0 && (
        <div className="flex-1 flex flex-col items-center justify-center text-muted">
          <div className="text-4xl mb-2">📚</div>
          <div className="text-lg">No learned patterns yet</div>
          <div className="text-sm mt-1">
            Edit a transcription to teach the system new corrections
          </div>
        </div>
      )}

      {/* Table */}
      {!loading && filteredCorrections.length > 0 && (
        <div className="flex-1 overflow-auto">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-card">
              <tr className="text-muted text-left border-b border-muted">
                <th className="pb-2 font-medium">Original</th>
                <th className="pb-2 font-medium">Corrected</th>
                <th className="pb-2 font-medium">Mode</th>
                <th className="pb-2 font-medium">Match</th>
                <th className="pb-2 font-medium">Uses</th>
                <th className="pb-2 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredCorrections.map((correction) => (
                <tr
                  key={correction.id}
                  className="border-b border-muted/30 hover:bg-background/50"
                >
                  <td className="py-2 font-mono text-foreground">
                    {correction.original}
                  </td>
                  <td className="py-2 font-mono">
                    {editingId === correction.id ? (
                      <input
                        type="text"
                        value={editForm.corrected}
                        onChange={(e) =>
                          setEditForm({ ...editForm, corrected: e.target.value })
                        }
                        className="bg-background text-primary px-2 py-1 rounded border border-primary w-full"
                        autoFocus
                      />
                    ) : (
                      <span className="text-primary">{correction.corrected}</span>
                    )}
                  </td>
                  <td className="py-2">
                    {editingId === correction.id ? (
                      <select
                        value={editForm.mode}
                        onChange={(e) =>
                          setEditForm({ ...editForm, mode: e.target.value })
                        }
                        className="bg-background text-foreground px-2 py-1 rounded border border-muted"
                      >
                        <option value="all">All</option>
                        <option value="secretary">Secretary</option>
                        <option value="code">Code</option>
                      </select>
                    ) : (
                      <span
                        className={`px-2 py-0.5 rounded text-xs ${
                          correction.mode === 'all'
                            ? 'bg-primary/20 text-primary'
                            : correction.mode === 'secretary'
                            ? 'bg-blue-500/20 text-blue-400'
                            : 'bg-purple-500/20 text-purple-400'
                        }`}
                      >
                        {getModeLabel(correction.mode)}
                      </span>
                    )}
                  </td>
                  <td className="py-2">
                    {editingId === correction.id ? (
                      <select
                        value={editForm.matchType}
                        onChange={(e) =>
                          setEditForm({ ...editForm, matchType: e.target.value })
                        }
                        className="bg-background text-foreground px-2 py-1 rounded border border-muted"
                      >
                        <option value="exact">Exact</option>
                        <option value="phonetic">Phonetic</option>
                      </select>
                    ) : (
                      <span
                        className={`px-2 py-0.5 rounded text-xs ${
                          correction.match_type === 'phonetic'
                            ? 'bg-orange-500/20 text-orange-400'
                            : 'bg-green-500/20 text-green-400'
                        }`}
                      >
                        {getMatchTypeLabel(correction.match_type)}
                      </span>
                    )}
                  </td>
                  <td className="py-2 text-muted">{correction.use_count}</td>
                  <td className="py-2">
                    <div className="flex gap-1">
                      {editingId === correction.id ? (
                        <>
                          <button
                            onClick={() => saveEdit(correction.id)}
                            className="px-2 py-1 bg-primary text-background rounded text-xs hover:opacity-90"
                          >
                            Save
                          </button>
                          <button
                            onClick={cancelEdit}
                            className="px-2 py-1 border border-muted text-muted rounded text-xs hover:border-primary"
                          >
                            Cancel
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            onClick={() => startEdit(correction)}
                            className="px-2 py-1 border border-muted text-muted rounded text-xs hover:border-primary hover:text-primary"
                          >
                            Edit
                          </button>
                          <button
                            onClick={() => deleteCorrection(correction.id)}
                            className="px-2 py-1 border border-error/50 text-error/70 rounded text-xs hover:border-error hover:text-error"
                          >
                            Delete
                          </button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Footer */}
      <div className="mt-4 pt-3 border-t border-muted/30 text-muted text-xs">
        {filteredCorrections.length} pattern{filteredCorrections.length !== 1 ? 's' : ''} •
        Daemon hot-reloads on file change
      </div>
    </div>
  );
}
