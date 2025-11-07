use swictation_broadcaster::MetricsBroadcaster;
use swictation_metrics::{DaemonState, RealtimeMetrics};
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tempfile::tempdir;
use std::time::Duration;

// Helper removed since we don't use it in the tests

#[tokio::test]
async fn test_broadcaster_lifecycle() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();

    // Start broadcaster
    broadcaster.start().await.unwrap();
    assert!(socket_path.exists());

    // Stop broadcaster
    broadcaster.stop().await.unwrap();
    assert!(!socket_path.exists());
}

#[tokio::test]
async fn test_client_connection_and_catch_up() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_catchup.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    // Add some data before client connects
    broadcaster.start_session(123).await;
    broadcaster.add_transcription("Hello".to_string(), 120.0, 200.0, 1).await;
    broadcaster.add_transcription("world".to_string(), 130.0, 180.0, 1).await;

    // Give broadcaster time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    // Read catch-up events
    let mut reader = BufReader::new(&mut client);
    let mut lines = Vec::new();

    // Read available events (with timeout)
    for _ in 0..4 {  // Expect: state_change, session_start, 2x transcription
        let mut line = String::new();
        tokio::select! {
            result = reader.read_line(&mut line) => {
                result.unwrap();
                if !line.is_empty() {
                    lines.push(line.trim().to_string());
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                break;
            }
        }
    }

    assert!(lines.len() >= 3, "Expected at least 3 catch-up events");

    // Verify events
    let first: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(first["type"], "state_change");

    broadcaster.stop().await.unwrap();
}

#[tokio::test]
async fn test_session_start_clears_buffer() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_buffer.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    // Add transcriptions
    broadcaster.add_transcription("First".to_string(), 100.0, 200.0, 1).await;
    assert_eq!(broadcaster.buffer_size().await, 1);

    broadcaster.add_transcription("Second".to_string(), 110.0, 190.0, 1).await;
    assert_eq!(broadcaster.buffer_size().await, 2);

    // Start new session should clear
    broadcaster.start_session(456).await;
    assert_eq!(broadcaster.buffer_size().await, 0);

    // Add new transcription
    broadcaster.add_transcription("Third".to_string(), 120.0, 180.0, 1).await;
    assert_eq!(broadcaster.buffer_size().await, 1);

    broadcaster.stop().await.unwrap();
}

#[tokio::test]
async fn test_session_end_keeps_buffer() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_session_end.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    broadcaster.start_session(789).await;
    broadcaster.add_transcription("Keep me".to_string(), 100.0, 200.0, 2).await;

    let size_before = broadcaster.buffer_size().await;
    broadcaster.end_session(789).await;
    let size_after = broadcaster.buffer_size().await;

    assert_eq!(size_before, size_after, "Buffer should persist after session end");

    broadcaster.stop().await.unwrap();
}

#[tokio::test]
async fn test_broadcast_to_multiple_clients() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_multi.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect two clients
    let mut client1 = UnixStream::connect(&socket_path).await.unwrap();
    let mut client2 = UnixStream::connect(&socket_path).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Broadcast transcription
    broadcaster.add_transcription("Broadcast test".to_string(), 150.0, 220.0, 2).await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Both clients should receive it
    let mut reader1 = BufReader::new(&mut client1);
    let mut reader2 = BufReader::new(&mut client2);

    // Skip catch-up events and find transcription
    let mut found1 = false;
    let mut found2 = false;

    for _ in 0..10 {
        let mut line1 = String::new();
        let mut line2 = String::new();

        if !found1 {
            if let Ok(_) = tokio::time::timeout(
                Duration::from_millis(100),
                reader1.read_line(&mut line1)
            ).await {
                if line1.contains("\"type\":\"transcription\"") {
                    found1 = true;
                }
            }
        }

        if !found2 {
            if let Ok(_) = tokio::time::timeout(
                Duration::from_millis(100),
                reader2.read_line(&mut line2)
            ).await {
                if line2.contains("\"type\":\"transcription\"") {
                    found2 = true;
                }
            }
        }

        if found1 && found2 {
            break;
        }
    }

    assert!(found1, "Client 1 should receive transcription");
    assert!(found2, "Client 2 should receive transcription");

    broadcaster.stop().await.unwrap();
}

#[tokio::test]
async fn test_metrics_update_broadcast() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_metrics.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    // Create metrics
    let metrics = RealtimeMetrics {
        current_state: DaemonState::Recording,
        recording_duration_s: 30.5,
        silence_duration_s: 2.0,
        speech_detected: true,
        current_session_id: Some(999),
        segments_this_session: 5,
        words_this_session: 42,
        wpm_this_session: 145.2,
        gpu_memory_current_mb: 1823.4,
        gpu_memory_total_mb: 4096.0,
        gpu_memory_percent: 45.2,
        cpu_percent_current: 23.1,
        last_segment_words: 8,
        last_segment_latency_ms: 234.5,
        last_segment_wpm: 150.0,
        last_transcription: "Test transcription".to_string(),
    };

    broadcaster.update_metrics(&metrics).await;

    // Verify broadcast happened (no panic)
    assert_eq!(broadcaster.client_count().await, 0);

    broadcaster.stop().await.unwrap();
}

#[tokio::test]
async fn test_state_change_broadcast() {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test_state.sock");

    let broadcaster = MetricsBroadcaster::new(&socket_path).await.unwrap();
    broadcaster.start().await.unwrap();

    // Broadcast state changes
    broadcaster.broadcast_state_change(DaemonState::Idle).await;
    broadcaster.broadcast_state_change(DaemonState::Recording).await;
    broadcaster.broadcast_state_change(DaemonState::Processing).await;

    // No panic = success
    assert_eq!(broadcaster.client_count().await, 0);

    broadcaster.stop().await.unwrap();
}
