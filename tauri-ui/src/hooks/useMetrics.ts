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
    // In Tauri v2, listen() returns Promise<UnlistenFn>
    const unlistenFns: Array<() => void> = [];

    // Use async IIFE to properly await the Promises
    (async () => {
      // Listen to connection status
      const unlistenConnected = await listen<boolean>('metrics-connected', (event) => {
        setMetrics((prev) => ({
          ...prev,
          connected: event.payload,
        }));
      });
      unlistenFns.push(unlistenConnected);

      // Listen to metrics updates
      const unlistenMetrics = await listen<BroadcastEvent & { type: 'metrics_update' }>('metrics-update', (event) => {
        const payload = event.payload;
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
      });
      unlistenFns.push(unlistenMetrics);

      // Listen to state changes
      const unlistenState = await listen<BroadcastEvent & { type: 'state_change' }>('state-change', (event) => {
        setMetrics((prev) => ({
          ...prev,
          state: event.payload.state,
        }));
      });
      unlistenFns.push(unlistenState);

      // Listen to transcriptions
      const unlistenTranscription = await listen<BroadcastEvent & { type: 'transcription' }>('transcription', (event) => {
        const payload = event.payload;
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
      });
      unlistenFns.push(unlistenTranscription);

      // Listen to session start
      const unlistenSessionStart = await listen<BroadcastEvent & { type: 'session_start' }>('session-start', (event) => {
        // Clear transcriptions on new session
        setTranscriptions([]);
        setMetrics((prev) => ({
          ...prev,
          sessionId: event.payload.session_id,
          segments: 0,
          words: 0,
          wpm: 0,
          duration: '00:00',
        }));
      });
      unlistenFns.push(unlistenSessionStart);

      // Listen to session end
      const unlistenSessionEnd = await listen<BroadcastEvent & { type: 'session_end' }>('session-end', () => {
        // Keep transcriptions visible
        // Session end doesn't reset metrics, just marks session as complete
      });
      unlistenFns.push(unlistenSessionEnd);
    })();

    return () => {
      unlistenFns.forEach((fn) => fn());
    };
  }, []);

  return { metrics, transcriptions };
}

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}
