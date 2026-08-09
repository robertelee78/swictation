//! Unix socket IPC server for toggle commands

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, info};

use crate::Daemon;

/// IPC command - JSON only
#[derive(Debug, serde::Deserialize)]
struct IpcCommand {
    action: String,
}

impl IpcCommand {
    fn parse(s: &str) -> Result<Self> {
        serde_json::from_str(s.trim())
            .context("Invalid JSON. Expected: {\"action\": \"toggle|status|quit\"}")
    }

    fn to_command_type(&self) -> Result<CommandType> {
        match self.action.to_lowercase().as_str() {
            "toggle" => Ok(CommandType::Toggle),
            "status" => Ok(CommandType::Status),
            "quit" | "exit" | "shutdown" => Ok(CommandType::Quit),
            _ => anyhow::bail!("Unknown action: {}", self.action),
        }
    }
}

#[derive(Debug)]
enum CommandType {
    Toggle,
    Status,
    Quit,
}

/// The daemon operations IPC needs.
///
/// Abstracted from `Daemon` itself so connection handling can be exercised
/// without building an audio pipeline; `Daemon` is the only production impl.
pub trait IpcTarget {
    async fn toggle(&self) -> Result<String>;
    async fn status(&self) -> String;
    /// Ask the main loop to shut down. Must not end the process itself — the
    /// loop still has to stop the broadcaster and release the socket.
    fn request_shutdown(&self);
}

/// Unix socket IPC server
pub struct IpcServer {
    listener: UnixListener,
    daemon: Arc<Daemon>,
}

impl IpcServer {
    /// Create new IPC server
    pub fn new(socket_path: &str, daemon: Arc<Daemon>) -> Result<Self> {
        // Remove existing socket if it exists
        let _ = std::fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path).context("Failed to bind Unix socket")?;

        // Set secure permissions (0600 = owner-only access)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let socket_path_buf = std::path::Path::new(socket_path);
            if socket_path_buf.exists() {
                let permissions = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(socket_path_buf, permissions)
                    .context("Failed to set socket permissions")?;
            }
        }

        info!(
            "IPC server listening on {} (permissions: 0600)",
            socket_path
        );

        Ok(Self { listener, daemon })
    }

    /// Accept next IPC connection
    pub async fn accept(&mut self) -> Result<(UnixStream, Arc<Daemon>)> {
        let (stream, _) = self
            .listener
            .accept()
            .await
            .context("Failed to accept connection")?;
        Ok((stream, self.daemon.clone()))
    }
}

/// Largest command accepted in one read.
const MAX_COMMAND_BYTES: usize = 1024;

/// How long a connected peer may go without sending its command.
///
/// The main loop serves connections inline inside its `tokio::select!` because
/// `Daemon` is `!Send`, so a peer that connects and stays silent — `nc -U` on
/// the socket — stalls the hotkey handler, the accept loop and Ctrl-C for as
/// long as it holds the socket open. This bounds that stall.
///
/// It covers the READ only. Executing the command and answering it run untimed:
/// a stop-drain routinely outlasts two seconds, and cancelling mid-toggle both
/// robs the caller of its response and strands the daemon's in-progress flag.
/// Bounding the read is what closes the silent-peer hole; the work that follows
/// is the daemon's own and is not attacker-controlled.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Handle a single IPC connection, giving up on an unresponsive peer.
pub async fn handle_connection<D: IpcTarget>(stream: UnixStream, daemon: Arc<D>) -> Result<()> {
    serve(stream, daemon, READ_TIMEOUT).await
}

/// `read_limit` is a parameter rather than a constant so tests can wait
/// milliseconds for a silent peer instead of [`READ_TIMEOUT`]'s two seconds.
async fn serve<D: IpcTarget>(
    mut stream: UnixStream,
    daemon: Arc<D>,
    read_limit: std::time::Duration,
) -> Result<()> {
    let mut buffer = [0u8; MAX_COMMAND_BYTES];
    let n = match tokio::time::timeout(read_limit, stream.read(&mut buffer)).await {
        Ok(result) => result?,
        Err(_) => {
            // Returning drops the stream, so the peer sees EOF instead of a
            // connection nobody is reading.
            debug!(
                "IPC connection sent no command within {:?}; dropping",
                read_limit
            );
            return Ok(());
        }
    };

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    debug!("Received IPC command: {}", request.trim());

    // Create JSON response
    let mut quitting = false;
    let response = match IpcCommand::parse(&request) {
        Ok(cmd) => match cmd.to_command_type() {
            Ok(CommandType::Toggle) => match daemon.toggle().await {
                Ok(msg) => serde_json::json!({
                    "status": "success",
                    "message": msg
                }),
                Err(e) => serde_json::json!({
                    "status": "error",
                    "error": format!("{}", e)
                }),
            },
            Ok(CommandType::Status) => {
                let status = daemon.status().await;
                serde_json::json!({
                    "status": "success",
                    "state": status
                })
            }
            Ok(CommandType::Quit) => {
                info!("Received quit command");
                quitting = true;
                serde_json::json!({
                    "status": "success",
                    "message": "Shutting down"
                })
            }
            Err(e) => {
                serde_json::json!({
                    "status": "error",
                    "error": format!("{}", e)
                })
            }
        },
        Err(e) => {
            serde_json::json!({
                "status": "error",
                "error": format!("{}", e)
            })
        }
    };

    let response_str = serde_json::to_string(&response)?;

    if quitting {
        // Quit cannot use the spawned write below: requesting shutdown tears the
        // runtime down, and a write still sitting in the queue at that moment is
        // aborted, handing the caller truncated JSON. Put the acknowledgement on
        // the wire first, then ask the main loop to stop — it owns the teardown
        // (broadcaster, socket) that an inline std::process::exit(0) here
        // skipped.
        //
        // Bounded because this is the one await after the read that a peer has
        // any say over. A ~40-byte response into a fresh socket cannot actually
        // fill the send buffer, but shutdown must not hinge on that: whatever
        // the write does, the daemon is still told to stop.
        let acknowledged = tokio::time::timeout(read_limit, async {
            stream.write_all(response_str.as_bytes()).await?;
            stream.flush().await
        })
        .await;
        match acknowledged {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::error!("Failed to write IPC quit response: {}", e),
            Err(_) => tracing::error!("Quit response not accepted within {:?}", read_limit),
        }

        daemon.request_shutdown();
        return Ok(());
    }

    // CRITICAL: Spawn the response write to prevent blocking the main event loop
    // The main tokio::select! loop can deadlock if write_all/flush are awaited inline
    // because the event loop can't poll while waiting for the write to complete.
    // By spawning, we immediately return control to the event loop.
    tokio::spawn(async move {
        if let Err(e) = stream.write_all(response_str.as_bytes()).await {
            tracing::error!("Failed to write IPC response: {}", e);
            return;
        }
        if let Err(e) = stream.flush().await {
            tracing::error!("Failed to flush IPC response: {}", e);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// Stand-in for `Daemon`, which owns an audio pipeline and cannot be built
    /// in a unit test.
    #[derive(Default)]
    struct TestTarget {
        toggles: AtomicUsize,
        shutdown_requested: AtomicBool,
        /// Applied to the FIRST toggle only, so a test can model one slow
        /// stop-drain followed by ordinary toggles.
        first_toggle_takes: Duration,
    }

    impl IpcTarget for TestTarget {
        async fn toggle(&self) -> Result<String> {
            let previous = self.toggles.fetch_add(1, Ordering::SeqCst);
            if previous == 0 && !self.first_toggle_takes.is_zero() {
                tokio::time::sleep(self.first_toggle_takes).await;
            }
            Ok("Recording started".to_string())
        }

        async fn status(&self) -> String {
            "idle".to_string()
        }

        fn request_shutdown(&self) {
            self.shutdown_requested.store(true, Ordering::SeqCst);
        }
    }

    fn temp_socket(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "swictation-ipc-{}-{}.sock",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    /// Short enough to keep the suite fast; the production value is
    /// [`READ_TIMEOUT`].
    const TEST_LIMIT: Duration = Duration::from_millis(150);

    #[tokio::test]
    async fn a_silent_connection_is_dropped_once_the_limit_expires() {
        let path = temp_socket("silent");
        let listener = UnixListener::bind(&path).unwrap();

        let mut client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        // The peer connects and says nothing — `nc -U <socket>` left open.
        let started = Instant::now();
        serve(server, Arc::new(TestTarget::default()), TEST_LIMIT)
            .await
            .unwrap();
        assert!(started.elapsed() >= TEST_LIMIT);

        // The handler let go of its end, so the peer sees EOF rather than a
        // connection the daemon still believes it owns.
        let mut buf = [0u8; 8];
        assert_eq!(client.read(&mut buf).await.unwrap(), 0);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_silent_peer_cannot_freeze_later_commands() {
        let path = temp_socket("freeze");
        let listener = UnixListener::bind(&path).unwrap();
        let target = Arc::new(TestTarget::default());

        // Mirrors main()'s event loop, which serves connections INLINE because
        // Daemon is !Send: nothing else runs until the handler returns.
        let served = target.clone();
        let accept_loop = tokio::spawn(async move {
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                serve(stream, served.clone(), TEST_LIMIT).await.unwrap();
            }
        });

        let silent = UnixStream::connect(&path).await.unwrap();

        let mut caller = UnixStream::connect(&path).await.unwrap();
        caller.write_all(br#"{"action":"status"}"#).await.unwrap();

        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(5), caller.read_to_string(&mut response))
            .await
            .expect("the silent connection froze the accept loop")
            .unwrap();
        assert!(response.contains("\"state\":\"idle\""), "got {response}");

        drop(silent);
        accept_loop.await.unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_peer_that_closes_without_writing_returns_at_once() {
        let path = temp_socket("eof");
        let listener = UnixListener::bind(&path).unwrap();

        let client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        drop(client);

        let started = Instant::now();
        serve(server, Arc::new(TestTarget::default()), TEST_LIMIT)
            .await
            .unwrap();
        assert!(started.elapsed() < TEST_LIMIT);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn quit_requests_shutdown_instead_of_ending_the_process() {
        let path = temp_socket("quit");
        let listener = UnixListener::bind(&path).unwrap();
        let target = Arc::new(TestTarget::default());

        let mut client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        client.write_all(br#"{"action":"quit"}"#).await.unwrap();

        serve(server, target.clone(), TEST_LIMIT).await.unwrap();

        // Reaching this line is itself the assertion: the handler used to call
        // std::process::exit(0) here, which took the test binary with it.
        assert!(target.shutdown_requested.load(Ordering::SeqCst));

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(
            response.contains("\"status\":\"success\""),
            "got {response}"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_payload_larger_than_the_cap_is_answered_with_an_error() {
        let path = temp_socket("oversized");
        let listener = UnixListener::bind(&path).unwrap();

        let mut client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        client
            .write_all(&vec![b'a'; MAX_COMMAND_BYTES * 4])
            .await
            .unwrap();

        serve(server, Arc::new(TestTarget::default()), TEST_LIMIT)
            .await
            .unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("\"status\":\"error\""), "got {response}");

        let _ = std::fs::remove_file(&path);
    }

    /// Records what the caller had actually received at the instant shutdown
    /// was signalled.
    struct AckWatcher {
        /// The caller's end of the socket, non-blocking so the check cannot
        /// itself wait for the write it is looking for.
        caller: std::os::unix::net::UnixStream,
        seen: std::sync::Mutex<String>,
    }

    impl IpcTarget for AckWatcher {
        async fn toggle(&self) -> Result<String> {
            unreachable!("this target only answers quit")
        }

        async fn status(&self) -> String {
            unreachable!("this target only answers quit")
        }

        fn request_shutdown(&self) {
            use std::io::Read as _;
            let mut buffer = [0u8; MAX_COMMAND_BYTES];
            let n = (&self.caller).read(&mut buffer).unwrap_or(0);
            *self.seen.lock().unwrap() = String::from_utf8_lossy(&buffer[..n]).into_owned();
        }
    }

    /// A stop-drain can legitimately outlast the read timeout. Cancelling the
    /// command mid-flight loses the caller's response and, in the real daemon,
    /// strands `toggling` — so the timeout must cover the read alone.
    #[tokio::test]
    async fn a_command_slower_than_the_timeout_still_completes() {
        let path = temp_socket("slow-command");
        let listener = UnixListener::bind(&path).unwrap();
        let target = Arc::new(TestTarget {
            first_toggle_takes: Duration::from_secs(3),
            ..Default::default()
        });

        let mut client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        client.write_all(br#"{"action":"toggle"}"#).await.unwrap();

        // Production limits, not TEST_LIMIT: three seconds of work under the
        // shipped two-second READ_TIMEOUT is the case that was cancelled.
        handle_connection(server, target.clone()).await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("Recording started"), "got {response}");

        // ...and the daemon still serves the next toggle, rather than reporting
        // one permanently in progress.
        let mut next = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        next.write_all(br#"{"action":"toggle"}"#).await.unwrap();
        handle_connection(server, target.clone()).await.unwrap();

        let mut response = String::new();
        next.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("Recording started"), "got {response}");
        assert_eq!(target.toggles.load(Ordering::SeqCst), 2);

        let _ = std::fs::remove_file(&path);
    }

    /// The caller must hold its acknowledgement before shutdown is signalled:
    /// the main loop tears the runtime down on that signal, and a write still
    /// queued at that point is aborted, handing the caller truncated JSON.
    #[tokio::test]
    async fn quit_is_acknowledged_before_shutdown_is_signalled() {
        let path = temp_socket("quit-ack");
        let listener = UnixListener::bind(&path).unwrap();

        let caller = std::os::unix::net::UnixStream::connect(&path).unwrap();
        let (server, _) = listener.accept().await.unwrap();
        {
            use std::io::Write as _;
            (&caller).write_all(br#"{"action":"quit"}"#).unwrap();
        }
        caller.set_nonblocking(true).unwrap();

        let target = Arc::new(AckWatcher {
            caller,
            seen: std::sync::Mutex::new(String::new()),
        });

        serve(server, target.clone(), TEST_LIMIT).await.unwrap();

        let seen = target.seen.lock().unwrap().clone();
        assert!(
            seen.contains("\"status\":\"success\""),
            "shutdown was signalled while the caller still had {seen:?}"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_well_formed_toggle_reaches_the_daemon() {
        let path = temp_socket("toggle");
        let listener = UnixListener::bind(&path).unwrap();
        let target = Arc::new(TestTarget::default());

        let mut client = UnixStream::connect(&path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        client.write_all(br#"{"action":"toggle"}"#).await.unwrap();

        serve(server, target.clone(), TEST_LIMIT).await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("Recording started"), "got {response}");
        assert_eq!(target.toggles.load(Ordering::SeqCst), 1);

        let _ = std::fs::remove_file(&path);
    }
}
