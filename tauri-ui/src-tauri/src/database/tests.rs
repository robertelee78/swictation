use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "swictation-ui-db-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_empty(db: &Database) {
    assert!(db.get_recent_sessions(10, 0).unwrap().is_empty());
    assert_eq!(db.get_session_count().unwrap(), 0);
    assert!(db.get_session_transcriptions(1).unwrap().is_empty());
    assert!(db.search_transcriptions("hello", 10).unwrap().is_empty());
    assert_eq!(
        serde_json::to_value(db.get_lifetime_stats().unwrap()).unwrap(),
        serde_json::to_value(LifetimeStats::default()).unwrap()
    );
}

fn assert_errors(db: &Database, expected: &str) {
    let errors = [
        db.get_recent_sessions(10, 0).unwrap_err(),
        db.get_session_count().unwrap_err(),
        db.get_session_transcriptions(1).unwrap_err(),
        db.search_transcriptions("hello", 10).unwrap_err(),
        db.get_lifetime_stats().unwrap_err(),
        db.reset_database().unwrap_err(),
    ];
    for error in errors {
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }
}

#[test]
fn missing_database_stays_absent_then_same_instance_reads_daemon_data() {
    let directory = TestDirectory::new();
    let path = directory.0.join("daemon-data/metrics.db");
    let db = Database::new(&path).unwrap();

    assert_empty(&db);
    db.reset_database().unwrap();
    assert!(!path.exists());
    assert!(!path.parent().unwrap().exists());

    // Simulate the daemon completing startup after permissions/model loading.
    fs::create_dir(path.parent().unwrap()).unwrap();
    let daemon = Connection::open(&path).unwrap();
    daemon
        .execute_batch(include_str!("fixtures/daemon_metrics.sql"))
        .unwrap();
    assert_empty(&db); // The daemon's fresh row includes nullable best/lowest values.

    daemon
        .execute_batch(
            "INSERT INTO sessions (id, start_time, end_time, duration_s, words_dictated,
             wpm, avg_latency_ms) VALUES (1, 1700000000, 1700000060, 60, 2, 2, 120);
         INSERT INTO segments (id, session_id, text, timestamp, total_latency_ms, words)
             VALUES (1, 1, 'hello world', 1700000001, 120, 2);
         UPDATE lifetime_stats SET total_words = 2, total_characters = 11,
             total_sessions = 1, total_time_minutes = 1, avg_wpm = 2,
             avg_latency_ms = 120, best_wpm_value = 2, best_wpm_session = 1,
             lowest_latency_ms = 120, lowest_latency_session = 1 WHERE id = 1;",
        )
        .unwrap();

    let before_queries = fs::read(&path).unwrap();
    let sessions = db.get_recent_sessions(10, 0).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, 1);
    assert_eq!(sessions[0].words_dictated, 2);
    assert_eq!(db.get_session_count().unwrap(), 1);
    assert_eq!(
        db.get_session_transcriptions(1).unwrap()[0].text,
        "hello world"
    );
    assert_eq!(
        db.search_transcriptions("hello", 10).unwrap()[0].session_id,
        1
    );
    let stats = db.get_lifetime_stats().unwrap();
    assert_eq!(stats.total_words, 2);
    assert_eq!(stats.best_wpm_value, 2.0);
    assert_eq!(stats.lowest_latency_ms, 120.0);
    assert_eq!(fs::read(&path).unwrap(), before_queries);

    // Explicit reset still operates on the existing daemon-owned database.
    db.reset_database().unwrap();
    assert_empty(&db);
    let tables: usize = daemon
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'
         AND name IN ('sessions', 'segments', 'lifetime_stats')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 3);
}

#[test]
fn corrupt_database_is_reported_and_not_replaced() {
    let directory = TestDirectory::new();
    let path = directory.0.join("metrics.db");
    let original = b"this is not a SQLite database";
    fs::write(&path, original).unwrap();
    let db = Database::new(&path).unwrap();

    assert_errors(&db, "not a database");
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn incompatible_schema_is_reported_and_not_reinitialized() {
    let directory = TestDirectory::new();
    let path = directory.0.join("metrics.db");
    let daemon = Connection::open(&path).unwrap();
    daemon
        .execute("CREATE TABLE unrelated (value TEXT)", [])
        .unwrap();
    drop(daemon);
    let original = fs::read(&path).unwrap();
    let db = Database::new(&path).unwrap();

    assert_errors(&db, "no such table");
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn missing_database_after_removal_is_not_recreated() {
    let directory = TestDirectory::new();
    let path = directory.0.join("metrics.db");
    let daemon = Connection::open(&path).unwrap();
    daemon
        .execute_batch(include_str!("fixtures/daemon_metrics.sql"))
        .unwrap();
    drop(daemon);
    let db = Database::new(&path).unwrap();
    assert_empty(&db);

    fs::remove_file(&path).unwrap();
    assert_empty(&db);
    db.reset_database().unwrap();
    assert!(!path.exists());
}
