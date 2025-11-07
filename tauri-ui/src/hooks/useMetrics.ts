import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { BroadcastEvent, TranscriptionItem, DaemonState } from '../types';

export interface LiveMetrics {
  state: DaemonState;
  sessionId: number | null;
  segments: number;
  words: number;
  wpm: number;
  duration: string;
  latencyMs: number;
  gpuMemoryMb: number;
  gpuMemoryPercent: number;
  cpuPercent: number;
  connected: boolean;
}

export function useMetrics() {
  const [metrics, setMetrics] = useState<LiveMetrics>({
    state: 'idle',
    sessionId: null,
    segments: 0,
    words: 0,
    wpm: 0,
    duration: '00:00',
    latencyMs: 0,
    gpuMemoryMb: 0,
    gpuMemoryPercent: 0,
    cpuPercent: 0,
    connected: false,
  });

  const [transcriptions, setTranscriptions] = useState<TranscriptionItem[]>([]);

  useEffect(() => {
    // Listen for metrics updates from daemon
    const unlistenMetrics = listen<BroadcastEvent>('metrics-event', (event) => {
      const payload = event.payload;

      switch (payload.type) {
        case 'metrics_update':
          setMetrics((prev) => ({
            ...prev,
            state: payload.state,
            sessionId: payload.session_id ?? null,
            segments: payload.segments,
            words: payload.words,
            wpm: payload.wpm,
            duration: formatDuration(payload.duration_s),
            latencyMs: payload.latency_ms,
            gpuMemoryMb: payload.gpu_memory_mb,
            gpuMemoryPercent: payload.gpu_memory_percent,
            cpuPercent: payload.cpu_percent,
            connected: true,
          }));
          break;

        case 'state_change':
          setMetrics((prev) => ({
            ...prev,
            state: payload.state,
          }));
          break;

        case 'transcription':
          setTranscriptions((prev) => [
            ...prev,
            {
              text: payload.text,
              timestamp: payload.timestamp,
              wpm: payload.wpm,
              latency_ms: payload.latency_ms,
              words: payload.words,
            },
          ]);
          break;

        case 'session_start':
          // Clear transcriptions on new session
          setTranscriptions([]);
          setMetrics((prev) => ({
            ...prev,
            sessionId: payload.session_id,
            segments: 0,
            words: 0,
            wpm: 0,
            duration: '00:00',
          }));
          break;

        case 'session_end':
          // Keep transcriptions visible
          break;
      }
    });

    return () => {
      unlistenMetrics.then((fn) => fn());
    };
  }, []);

  return { metrics, transcriptions };
}

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}
