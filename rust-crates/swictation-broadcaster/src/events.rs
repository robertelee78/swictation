use serde::{Deserialize, Serialize};

/// Event types broadcast to UI clients
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BroadcastEvent {
    /// Session started - clears transcription buffer
    #[serde(rename = "session_start")]
    SessionStart {
        session_id: i64,
        timestamp: f64
    },

    /// Session ended - buffer stays visible
    #[serde(rename = "session_end")]
    SessionEnd {
        session_id: i64,
        timestamp: f64
    },

    /// New transcription segment
    #[serde(rename = "transcription")]
    Transcription {
        text: String,
        timestamp: String,  // HH:MM:SS format
        wpm: f64,
        latency_ms: f64,
        words: i32,
    },

    /// Real-time metrics update
    #[serde(rename = "metrics_update")]
    MetricsUpdate {
        state: String,
        session_id: Option<i64>,
        segments: i32,
        words: i32,
        wpm: f64,
        duration_s: f64,
        latency_ms: f64,
        gpu_memory_mb: f64,
        gpu_memory_percent: f64,
        cpu_percent: f64,
    },

    /// Daemon state changed
    #[serde(rename = "state_change")]
    StateChange {
        state: String,
        timestamp: f64
    },
}

/// Transcription segment stored in RAM buffer
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionSegment {
    pub text: String,
    pub timestamp: String,
    pub wpm: f64,
    pub latency_ms: f64,
    pub words: i32,
}

impl BroadcastEvent {
    /// Convert event to JSON string with newline
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("{}\n", json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start_serialization() {
        let event = BroadcastEvent::SessionStart {
            session_id: 123,
            timestamp: 1699000000.0,
        };
        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"type\":\"session_start\""));
        assert!(json.contains("\"session_id\":123"));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn test_transcription_serialization() {
        let event = BroadcastEvent::Transcription {
            text: "Hello world".to_string(),
            timestamp: "14:23:15".to_string(),
            wpm: 145.2,
            latency_ms: 234.5,
            words: 2,
        };
        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"type\":\"transcription\""));
        assert!(json.contains("\"text\":\"Hello world\""));
        assert!(json.contains("\"wpm\":145.2"));
    }

    #[test]
    fn test_metrics_update_serialization() {
        let event = BroadcastEvent::MetricsUpdate {
            state: "recording".to_string(),
            session_id: Some(123),
            segments: 5,
            words: 42,
            wpm: 145.2,
            duration_s: 30.5,
            latency_ms: 234.5,
            gpu_memory_mb: 1823.4,
            gpu_memory_percent: 45.2,
            cpu_percent: 23.1,
        };
        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"type\":\"metrics_update\""));
        assert!(json.contains("\"state\":\"recording\""));
        assert!(json.contains("\"segments\":5"));
    }
}
