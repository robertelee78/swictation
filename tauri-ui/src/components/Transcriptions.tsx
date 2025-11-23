import { useEffect, useRef, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptionItem } from '../types';
import { useWasmUtils } from '../hooks/useWasmUtils';

interface Props {
  transcriptions: TranscriptionItem[];
}

interface LearnModalState {
  isOpen: boolean;
  original: string;
  corrected: string;
  selectedText: string;
}

interface DiffHunk {
  op: 'Equal' | 'Insert' | 'Delete';
  text: string;
}

export function Transcriptions({ transcriptions }: Props) {
  const listRef = useRef<HTMLDivElement>(null);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [editText, setEditText] = useState<string>('');
  const [learnModal, setLearnModal] = useState<LearnModalState>({
    isOpen: false,
    original: '',
    corrected: '',
    selectedText: '',
  });
  const [learnMode, setLearnMode] = useState<string>('all');
  const [learnMatchType, setLearnMatchType] = useState<string>('exact');
  const [toast, setToast] = useState<string | null>(null);

  // WASM utilities hook
  const { isLoaded: wasmLoaded, computeTextDiff } = useWasmUtils();

  // Auto-scroll to bottom when new transcriptions arrive
  // Debounced to reduce layout thrashing during rapid speech
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      if (listRef.current) {
        listRef.current.scrollTop = listRef.current.scrollHeight;
      }
    }, 100);

    return () => clearTimeout(timeoutId);
  }, [transcriptions]);

  const copyToClipboard = async (text: string, index: number) => {
    try {
      await writeText(text);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
      alert(`Failed to copy to clipboard: ${err}`);
    }
  };

  const startEditing = (index: number, text: string) => {
    // Check for text selection
    const selection = window.getSelection();
    const selectedText = selection?.toString().trim() || '';

    setEditingIndex(index);
    setEditText(text);
    setLearnModal(prev => ({ ...prev, selectedText }));
  };

  const cancelEditing = () => {
    setEditingIndex(null);
    setEditText('');
    setLearnModal(prev => ({ ...prev, selectedText: '' }));
  };

  const saveEdit = async (originalText: string) => {
    if (editText.trim() === originalText.trim()) {
      cancelEditing();
      return;
    }

    // Open learn modal with diff
    setLearnModal({
      isOpen: true,
      original: originalText,
      corrected: editText,
      selectedText: learnModal.selectedText,
    });
  };

  // Compute diff using WASM (real-time, 32x faster than backend)
  const [liveDiff, setLiveDiff] = useState<DiffHunk[]>([]);

  useEffect(() => {
    if (!wasmLoaded || !learnModal.isOpen) {
      setLiveDiff([]);
      return;
    }

    const originalToLearn = learnModal.selectedText || learnModal.original;

    // Compute diff asynchronously
    computeTextDiff(originalToLearn, learnModal.corrected)
      .then((diffJson) => {
        const hunks: DiffHunk[] = JSON.parse(diffJson);
        setLiveDiff(hunks);
      })
      .catch((err) => {
        console.error('WASM diff computation failed:', err);
        setLiveDiff([]);
      });
  }, [wasmLoaded, learnModal.isOpen, learnModal.original, learnModal.corrected, learnModal.selectedText, computeTextDiff]);

  // Extract correction pairs from diff hunks
  const extractCorrectionsFromDiff = (hunks: DiffHunk[]): [string, string][] => {
    const corrections: [string, string][] = [];

    for (let i = 0; i < hunks.length; i++) {
      const hunk = hunks[i];

      // Look for Delete followed by Insert (substitution)
      if (hunk.op === 'Delete' && i + 1 < hunks.length && hunks[i + 1].op === 'Insert') {
        corrections.push([hunk.text, hunks[i + 1].text]);
        i++; // Skip the next Insert as we've processed it
      }
      // Standalone Insert (addition)
      else if (hunk.op === 'Insert' && (i === 0 || hunks[i - 1].op !== 'Delete')) {
        corrections.push(['', hunk.text]);
      }
      // Standalone Delete (removal)
      else if (hunk.op === 'Delete') {
        corrections.push([hunk.text, '']);
      }
    }

    return corrections;
  };

  const confirmLearn = async () => {
    try {
      // If user had selected text, use that as the original
      const originalToLearn = learnModal.selectedText || learnModal.original;

      // Use WASM diff if available, fallback to backend
      let diffs: [string, string][];

      if (wasmLoaded && liveDiff.length > 0) {
        // Extract corrections from WASM diff hunks (0.25ms - 32x faster!)
        diffs = extractCorrectionsFromDiff(liveDiff);
      } else {
        // Fallback to backend (8ms)
        diffs = await invoke('extract_corrections_diff', {
          original: originalToLearn,
          edited: learnModal.corrected,
        });
      }

      // Learn each correction
      for (const [orig, corr] of diffs) {
        if (orig !== corr && orig.trim() !== '' && corr.trim() !== '') {
          await invoke('learn_correction', {
            original: orig,
            corrected: corr,
            mode: learnMode,
            matchType: learnMatchType,
          });
        }
      }

      // If no diffs but text changed, learn the whole thing
      if (diffs.length === 0 && learnModal.original !== learnModal.corrected) {
        await invoke('learn_correction', {
          original: learnModal.selectedText || learnModal.original,
          corrected: learnModal.corrected,
          mode: learnMode,
          matchType: learnMatchType,
        });
      }

      const learnCount = diffs.filter(([o, c]) => o !== c && o.trim() && c.trim()).length;
      setToast(`✓ Learned ${learnCount} correction${learnCount !== 1 ? 's' : ''} ${wasmLoaded ? '(WASM ⚡)' : ''}`);
      setTimeout(() => setToast(null), 3000);

      setLearnModal({ isOpen: false, original: '', corrected: '', selectedText: '' });
      setEditingIndex(null);
      setEditText('');
    } catch (err) {
      console.error('Failed to learn correction:', err);
      alert(`Failed to learn correction: ${err}`);
    }
  };

  return (
    <div className="h-full bg-card rounded-lg m-5 p-4 flex flex-col">
      {/* Header */}
      <div className="mb-3">
        <h2 className="text-primary text-xl font-bold">Session Transcriptions (Ephemeral)</h2>
        <p className="text-muted text-xs italic mt-1">
          🔒 Privacy: Not saved to disk, RAM-only
        </p>
      </div>

      {/* Toast notification */}
      {toast && (
        <div className="mb-3 px-3 py-2 bg-primary text-background rounded text-sm font-mono">
          {toast}
        </div>
      )}

      {/* Transcription List */}
      <div ref={listRef} className="flex-1 overflow-auto space-y-3">
        {transcriptions.map((item, index) => (
          <div
            key={index}
            className="bg-background rounded border border-primary p-3"
          >
            <div className="flex items-center gap-2 mb-2 text-xs">
              <span className="text-muted font-mono">{item.timestamp}</span>
              <span className="text-muted">│</span>
              <span className="text-muted">{Math.round(item.wpm)} WPM</span>
              <span className="text-muted">│</span>
              <span className={item.latency_ms > 2000 ? 'text-error' : 'text-muted'}>
                {(item.latency_ms / 1000).toFixed(2)}s
              </span>

              <div className="flex-1" />

              {/* Edit button */}
              <button
                onClick={() => startEditing(index, item.text)}
                className="w-8 h-6 rounded border border-muted hover:border-primary hover:bg-border transition-colors text-sm"
                title="Edit & Learn correction"
              >
                ✏️
              </button>

              {/* Copy button */}
              <button
                onClick={() => copyToClipboard(item.text, index)}
                className={`w-8 h-6 rounded border transition-colors text-sm ${
                  copiedIndex === index
                    ? 'border-primary bg-primary text-background'
                    : 'border-muted hover:border-primary hover:bg-border'
                }`}
                title={copiedIndex === index ? 'Copied!' : 'Copy to clipboard'}
              >
                {copiedIndex === index ? '✓' : '📋'}
              </button>
            </div>

            {editingIndex === index ? (
              <div className="space-y-2">
                <textarea
                  value={editText}
                  onChange={(e) => setEditText(e.target.value)}
                  className="w-full bg-card text-foreground-bright font-mono text-sm p-2 rounded border border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                  rows={3}
                  autoFocus
                />
                <div className="flex gap-2">
                  <button
                    onClick={() => saveEdit(item.text)}
                    className="px-3 py-1 bg-primary text-background rounded text-sm hover:opacity-90"
                  >
                    Learn
                  </button>
                  <button
                    onClick={cancelEditing}
                    className="px-3 py-1 border border-muted text-muted rounded text-sm hover:border-primary"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <p className="text-foreground-bright font-mono text-sm break-words">
                {item.text}
              </p>
            )}
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="mt-3 text-muted text-xs italic">
        ⚠️  Buffer clears when you start a new session
      </div>

      {/* Learn Modal with Real-Time Diff Preview */}
      {learnModal.isOpen && (
        <div className="fixed inset-0 bg-background/80 flex items-center justify-center z-50">
          <div className="bg-card border border-primary rounded-lg p-6 max-w-2xl w-full mx-4 shadow-lg">
            <h3 className="text-primary text-lg font-bold mb-4 flex items-center gap-2">
              Learn Correction
              {wasmLoaded && <span className="text-xs text-success font-mono">WASM ⚡</span>}
            </h3>

            <div className="space-y-4">
              {/* Live Diff Preview */}
              {wasmLoaded && liveDiff.length > 0 && (
                <div>
                  <label className="text-muted text-xs block mb-1">
                    Live Diff Preview (computed in ~0.25ms):
                  </label>
                  <div className="bg-background font-mono text-sm p-3 rounded border border-primary max-h-32 overflow-y-auto">
                    {liveDiff.map((hunk, i) => {
                      if (hunk.op === 'Equal') {
                        return <span key={i} className="text-muted">{hunk.text} </span>;
                      } else if (hunk.op === 'Delete') {
                        return (
                          <span key={i} className="bg-error/20 text-error line-through">
                            {hunk.text}{' '}
                          </span>
                        );
                      } else {
                        return (
                          <span key={i} className="bg-success/20 text-success font-bold">
                            {hunk.text}{' '}
                          </span>
                        );
                      }
                    })}
                  </div>
                </div>
              )}

              <div>
                <label className="text-muted text-xs block mb-1">Original:</label>
                <div className="bg-background text-foreground font-mono text-sm p-2 rounded border border-muted">
                  {learnModal.selectedText || learnModal.original}
                </div>
              </div>

              <div>
                <label className="text-muted text-xs block mb-1">Corrected:</label>
                <div className="bg-background text-primary font-mono text-sm p-2 rounded border border-primary">
                  {learnModal.corrected}
                </div>
              </div>

              <div className="flex gap-4">
                <div className="flex-1">
                  <label className="text-muted text-xs block mb-1">Mode Scope:</label>
                  <select
                    value={learnMode}
                    onChange={(e) => setLearnMode(e.target.value)}
                    className="w-full bg-background text-foreground p-2 rounded border border-muted focus:border-primary"
                  >
                    <option value="all">All Modes</option>
                    <option value="secretary">Secretary Only</option>
                    <option value="code">Code Only</option>
                  </select>
                </div>

                <div className="flex-1">
                  <label className="text-muted text-xs block mb-1">Match Type:</label>
                  <select
                    value={learnMatchType}
                    onChange={(e) => setLearnMatchType(e.target.value)}
                    className="w-full bg-background text-foreground p-2 rounded border border-muted focus:border-primary"
                  >
                    <option value="exact">Exact Match</option>
                    <option value="phonetic">Phonetic Match</option>
                  </select>
                </div>
              </div>
            </div>

            <div className="flex gap-3 mt-6 justify-end">
              <button
                onClick={() => setLearnModal({ isOpen: false, original: '', corrected: '', selectedText: '' })}
                className="px-4 py-2 border border-muted text-muted rounded hover:border-primary"
              >
                Cancel
              </button>
              <button
                onClick={confirmLearn}
                className="px-4 py-2 bg-primary text-background rounded hover:opacity-90"
              >
                Learn Pattern
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
