import { useEffect, useRef } from 'react';
import { writeText } from '@tauri-apps/api/clipboard';
import type { TranscriptionItem } from '../types';

interface Props {
  transcriptions: TranscriptionItem[];
}

export function Transcriptions({ transcriptions }: Props) {
  const listRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new transcriptions arrive
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [transcriptions]);

  const copyToClipboard = async (text: string) => {
    try {
      await writeText(text);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
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

              <button
                onClick={() => copyToClipboard(item.text)}
                className="w-8 h-6 rounded border border-muted hover:border-primary hover:bg-border transition-colors text-sm"
                title="Copy to clipboard"
              >
                📋
              </button>
            </div>

            <p className="text-foreground-bright font-mono text-sm break-words">
              {item.text}
            </p>
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="mt-3 text-muted text-xs italic">
        ⚠️  Buffer clears when you start a new session
      </div>
    </div>
  );
}
