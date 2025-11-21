import { useEffect, useRef, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptionItem } from '../types';

interface Props {
  transcriptions: TranscriptionItem[];
}

interface LearnModalState {
  isOpen: boolean;
  original: string;
  corrected: string;
  selectedText: string;
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

  // Auto-scroll to bottom when new transcriptions arrive
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
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

  const confirmLearn = async () => {
    try {
      // If user had selected text, use that as the original
      const originalToLearn = learnModal.selectedText || learnModal.original;

      // Extract corrections diff
      const diffs: [string, string][] = await invoke('extract_corrections_diff', {
        original: originalToLearn,
        edited: learnModal.corrected,
      });

      // Learn each correction
      for (const [orig, corr] of diffs) {
        if (orig !== corr) {
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

      setToast(`Learned: "${learnModal.selectedText || learnModal.original}" → "${learnModal.corrected}"`);
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

      {/* Learn Modal */}
      {learnModal.isOpen && (
        <div className="fixed inset-0 bg-background/80 flex items-center justify-center z-50">
          <div className="bg-card border border-primary rounded-lg p-6 max-w-lg w-full mx-4 shadow-lg">
            <h3 className="text-primary text-lg font-bold mb-4">Learn Correction</h3>

            <div className="space-y-4">
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
