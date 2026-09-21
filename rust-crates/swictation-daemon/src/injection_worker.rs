//! Keep the thread-local text injector alive through permission/tool outages.
//!
//! Unavailable text is discarded, including text queued while a successful
//! initialization is in progress. No error or event contains dictated content.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

pub(crate) struct PendingText {
    text: String,
    queued_at: Instant,
}

impl PendingText {
    pub(crate) fn new(text: String) -> Self {
        Self {
            text,
            queued_at: Instant::now(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkerEvent {
    Unavailable,
    Ready { recovered: bool },
    Injected { chars: usize },
    InjectionFailed { chars: usize },
}

/// Call on the dedicated injection thread: `I` deliberately has no Send bound.
/// The factory must check availability without prompting for permissions.
pub(crate) fn run<I, E>(
    receiver: Receiver<PendingText>,
    mut create: impl FnMut() -> Result<I, E>,
    mut inject: impl FnMut(&I, &str) -> Result<(), E>,
    mut observe: impl FnMut(WorkerEvent),
    retry_interval: Duration,
) {
    assert!(!retry_interval.is_zero(), "retry interval must be positive");
    let mut injector = None;
    let mut ready_since = None;
    let mut retry_at = Instant::now();
    let mut outage_reported = false;

    loop {
        if injector.is_none() && Instant::now() >= retry_at {
            match create() {
                Ok(ready) => {
                    // Stamp AFTER initialization: queued text from an outage
                    // must not get pasted when permission finally appears.
                    ready_since = Some(Instant::now());
                    injector = Some(ready);
                    observe(WorkerEvent::Ready {
                        recovered: outage_reported,
                    });
                    outage_reported = false;
                }
                Err(_) => {
                    if !outage_reported {
                        observe(WorkerEvent::Unavailable);
                    }
                    outage_reported = true;
                    retry_at = Instant::now() + retry_interval;
                }
            }
        }

        let pending = if injector.is_some() {
            match receiver.recv() {
                Ok(pending) => pending,
                Err(_) => break,
            }
        } else {
            match receiver.recv_timeout(retry_at.saturating_duration_since(Instant::now())) {
                Ok(pending) => pending,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };

        let Some(ready) = injector.as_ref() else {
            continue;
        };
        if ready_since.is_some_and(|since| pending.queued_at < since) {
            continue;
        }

        let chars = pending.text.chars().count();
        if inject(ready, &pending.text).is_ok() {
            observe(WorkerEvent::Injected { chars });
        } else {
            // Injection may have partially succeeded. Never replay it or log
            // the error: external injection tools may echo text in stderr.
            observe(WorkerEvent::InjectionFailed { chars });
            injector = None;
            ready_since = None;
            outage_reported = true;
            retry_at = Instant::now() + retry_interval;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};
    use std::thread;

    const DEADLINE: Duration = Duration::from_secs(5);
    const RETRY: Duration = Duration::from_millis(5);

    fn event(receiver: &Receiver<WorkerEvent>) -> WorkerEvent {
        receiver.recv_timeout(DEADLINE).expect("worker event")
    }

    #[test]
    fn recovers_after_initial_failure_without_replaying_unavailable_text() {
        let (text_tx, text_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (initializing_tx, initializing_rx) = mpsc::channel();
        let (permit_tx, permit_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut attempts = 0;
            let mut injected = Vec::new();
            run(
                text_rx,
                || {
                    attempts += 1;
                    if attempts == 1 {
                        return Err(());
                    }
                    initializing_tx.send(()).unwrap();
                    permit_rx.recv_timeout(DEADLINE).unwrap();
                    // The real macOS injector also contains an Rc: this
                    // verifies that the worker does not require Send/Sync.
                    Ok(Rc::new(()))
                },
                |_, text| {
                    injected.push(text.to_owned());
                    Ok(())
                },
                |event| event_tx.send(event).unwrap(),
                RETRY,
            );
            (attempts, injected)
        });

        assert_eq!(event(&event_rx), WorkerEvent::Unavailable);
        text_tx.send(PendingText::new("denied".into())).unwrap();
        initializing_rx.recv_timeout(DEADLINE).unwrap();
        // This remains queued until initialization completes. Draining only
        // during failed attempts would incorrectly paste it after recovery.
        text_tx
            .send(PendingText::new("queued during initialization".into()))
            .unwrap();
        permit_tx.send(()).unwrap();
        assert_eq!(event(&event_rx), WorkerEvent::Ready { recovered: true });
        text_tx.send(PendingText::new("fresh".into())).unwrap();
        drop(text_tx);

        let (attempts, injected) = worker.join().unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(injected, ["fresh"]);
        assert_eq!(event(&event_rx), WorkerEvent::Injected { chars: 5 });
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn denied_text_does_not_trigger_retries_and_channel_closure_exits_promptly() {
        let (text_tx, text_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let attempts = Arc::new(AtomicUsize::new(0));
        let observed_attempts = Arc::clone(&attempts);
        let worker = thread::spawn(move || {
            run(
                text_rx,
                || -> Result<(), ()> {
                    observed_attempts.fetch_add(1, Ordering::SeqCst);
                    Err(())
                },
                |_, _| panic!("unavailable text must not be injected"),
                |event| event_tx.send(event).unwrap(),
                Duration::from_secs(60),
            );
            done_tx.send(()).unwrap();
        });
        assert_eq!(event(&event_rx), WorkerEvent::Unavailable);
        for _ in 0..100 {
            text_tx.send(PendingText::new("discard".into())).unwrap();
        }
        drop(text_tx);
        done_rx.recv_timeout(DEADLINE).expect("prompt shutdown");
        worker.join().unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn failed_injection_is_not_replayed_and_worker_recovers() {
        let (text_tx, text_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut attempts = Vec::new();
            run(
                text_rx,
                || Ok(()),
                |_, text| {
                    attempts.push(text.to_owned());
                    if attempts.len() == 1 {
                        Err(())
                    } else {
                        Ok(())
                    }
                },
                |event| event_tx.send(event).unwrap(),
                RETRY,
            );
            attempts
        });
        assert_eq!(event(&event_rx), WorkerEvent::Ready { recovered: false });
        text_tx.send(PendingText::new("failed".into())).unwrap();
        assert_eq!(event(&event_rx), WorkerEvent::InjectionFailed { chars: 6 });
        assert_eq!(event(&event_rx), WorkerEvent::Ready { recovered: true });
        text_tx.send(PendingText::new("works".into())).unwrap();
        drop(text_tx);
        assert_eq!(worker.join().unwrap(), ["failed", "works"]);
        assert_eq!(event(&event_rx), WorkerEvent::Injected { chars: 5 });
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn ready_worker_drains_queued_text_exactly_once_on_channel_close() {
        let (text_tx, text_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut injected = Vec::new();
            run(
                text_rx,
                || Ok::<_, ()>(()),
                |_, text| {
                    injected.push(text.to_owned());
                    Ok(())
                },
                |event| event_tx.send(event).unwrap(),
                RETRY,
            );
            injected
        });
        assert_eq!(event(&event_rx), WorkerEvent::Ready { recovered: false });
        for text in ["first", "second", "third"] {
            text_tx.send(PendingText::new(text.into())).unwrap();
        }
        drop(text_tx);
        assert_eq!(worker.join().unwrap(), ["first", "second", "third"]);
    }
}
