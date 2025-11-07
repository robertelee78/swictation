// TypeScript types matching Rust structs

export type DaemonState = 'idle' | 'recording' | 'processing' | 'error';

export interface SessionMetrics {
  session_id?: number;
  session_start?: string;
  session_end?: string;
  total_duration_s: number;
  active_dictation_time_s: number;
  pause_time_s: number;
  words_dictated: number;
  characters_typed: number;
  segments_processed: number;
  words_per_minute: number;
  typing_speed_equivalent: number;
  average_latency_ms: number;
  median_latency_ms: number;
  p95_latency_ms: number;
  transformations_count: number;
  keyboard_actions_count: number;
  average_segment_words: number;
  average_segment_duration_s: number;
  gpu_memory_peak_mb: number;
  gpu_memory_mean_mb: number;
  cpu_usage_mean_percent: number;
  cpu_usage_peak_percent: number;
}

export interface SegmentMetrics {
  segment_id?: number;
  session_id?: number;
  timestamp?: string;
  duration_s: number;
  words: number;
  characters: number;
  text: string;
  vad_latency_ms: number;
  audio_save_latency_ms: number;
  stt_latency_ms: number;
  transform_latency_us: number;
  injection_latency_ms: number;
  total_latency_ms: number;
  transformations_count: number;
  keyboard_actions_count: number;
}

export interface LifetimeMetrics {
  total_words: number;
  total_characters: number;
  total_sessions: number;
  total_dictation_time_minutes: number;
  total_segments: number;
  average_wpm: number;
  average_latency_ms: number;
  typing_speed_equivalent: number;
  speedup_factor: number;
  estimated_time_saved_minutes: number;
  wpm_trend_7day: number;
  latency_trend_7day: number;
  cuda_errors_total: number;
  cuda_errors_recovered: number;
  memory_pressure_events: number;
  high_latency_warnings: number;
  best_wpm_session?: number;
  best_wpm_value: number;
  longest_session_words: number;
  longest_session_id?: number;
  lowest_latency_session?: number;
  lowest_latency_ms: number;
  last_updated?: string;
}

export interface RealtimeMetrics {
  current_state: DaemonState;
  recording_duration_s: number;
  silence_duration_s: number;
  speech_detected: boolean;
  current_session_id?: number;
  segments_this_session: number;
  words_this_session: number;
  wpm_this_session: number;
  gpu_memory_current_mb: number;
  gpu_memory_total_mb: number;
  gpu_memory_percent: number;
  cpu_percent_current: number;
  last_segment_words: number;
  last_segment_latency_ms: number;
  last_segment_wpm: number;
  last_transcription: string;
}

// Broadcast events from daemon
export type BroadcastEvent =
  | {
      type: 'session_start';
      session_id: number;
      timestamp: number;
    }
  | {
      type: 'session_end';
      session_id: number;
      timestamp: number;
    }
  | {
      type: 'transcription';
      text: string;
      timestamp: string;
      wpm: number;
      latency_ms: number;
      words: number;
    }
  | {
      type: 'metrics_update';
      state: DaemonState;
      session_id?: number;
      segments: number;
      words: number;
      wpm: number;
      duration_s: number;
      latency_ms: number;
      gpu_memory_mb: number;
      gpu_memory_percent: number;
      cpu_percent: number;
    }
  | {
      type: 'state_change';
      state: DaemonState;
      timestamp: number;
    };

export interface TranscriptionItem {
  text: string;
  timestamp: string;
  wpm: number;
  latency_ms: number;
  words: number;
}

// Database query results
export interface HistorySession {
  id: number;
  start_time: number;
  end_time?: number;
  duration_s: number;
  words_dictated: number;
  wpm: number;
  avg_latency_ms: number;
}
