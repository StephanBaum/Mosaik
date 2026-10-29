//! Copying to the system clipboard from Rust (`docs/DESKTOP_IPC.md`,
//! "Clipboard"): the secret never enters JavaScript.
//!
//! - Every write is marked so the OS keeps it out of clipboard history and
//!   clipboard managers: on Windows the `ExcludeClipboardContentFromMonitorProcessing`,
//!   `CanIncludeInClipboardHistory = 0` and `CanUploadToCloudClipboard = 0`
//!   formats, on macOS `org.nspasteboard.ConcealedType`, on Linux
//!   `x-kde-passwordManagerHint = secret` (all through `arboard`).
//! - After the timeout (`Settings::clipboard_clear_seconds`, default 30 s)
//!   the clipboard is cleared, but only if it still holds what Mosaic
//!   copied (compared in constant time), so a value the user copied since
//!   is never touched. Then `clipboardCleared` is emitted.
//! - Locking and quitting clear it the same way.
//!
//! One worker thread owns the OS clipboard handle (on Linux it must stay
//! alive to keep serving the content) and the pending timeout; commands talk
//! to it over a channel.
//!
//! Residual exposure (accepted, THREAT_MODEL R19): the OS clipboard itself
//! and `arboard`'s conversion buffers hold the value until it is cleared or
//! replaced; the copy Mosaic keeps for the comparison is zeroized.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::SecretString;
use subtle::ConstantTimeEq;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::events::Events;

/// Longest the worker sleeps between looks at the wall clock, so a suspend
/// (where the monotonic clock may stop) delays the clear by at most this.
pub const MAX_WAIT: Duration = Duration::from_secs(1);

/// The OS clipboard, as the worker uses it.
pub trait ClipboardBackend {
    /// Writes `text`, excluded from history and monitoring where supported.
    ///
    /// # Errors
    /// When the clipboard cannot be written.
    fn write(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// The current text, or `None` if the clipboard holds no text.
    fn read(&mut self) -> Option<SecretString>;

    /// Empties the clipboard.
    ///
    /// # Errors
    /// When the clipboard cannot be written.
    fn clear(&mut self) -> Result<(), ClipboardError>;
}

/// The clipboard could not be used. Carries nothing (no OS message, which
/// could quote clipboard content).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClipboardError;

/// Creates the backend inside the worker thread.
pub type BackendFactory = Box<dyn FnOnce() -> Option<Box<dyn ClipboardBackend>> + Send>;

/// The system clipboard through `arboard`.
pub struct SystemClipboard(arboard::Clipboard);

impl SystemClipboard {
    /// A factory for the worker; `None` from it when there is no clipboard
    /// (for example no display server).
    pub fn factory() -> BackendFactory {
        Box::new(|| {
            arboard::Clipboard::new()
                .ok()
                .map(|c| Box::new(Self(c)) as Box<dyn ClipboardBackend>)
        })
    }
}

impl ClipboardBackend for SystemClipboard {
    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        let set = self.0.set();
        #[cfg(windows)]
        let set = {
            use arboard::SetExtWindows as _;
            set.exclude_from_monitoring()
                .exclude_from_history()
                .exclude_from_cloud()
        };
        #[cfg(target_os = "macos")]
        let set = {
            use arboard::SetExtApple as _;
            set.exclude_from_history()
        };
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        let set = {
            use arboard::SetExtLinux as _;
            set.exclude_from_history()
        };
        set.text(text).map_err(|_| ClipboardError)
    }

    fn read(&mut self) -> Option<SecretString> {
        self.0.get_text().ok().map(SecretString::from)
    }

    fn clear(&mut self) -> Result<(), ClipboardError> {
        self.0.clear().map_err(|_| ClipboardError)
    }
}

/// An in-process clipboard, the default until the app installs the system
/// one, and for tests. Clones share the content, so a test can play "the
/// user copied something else". The content is a zeroizing secret string.
#[derive(Clone, Default)]
pub struct MemoryClipboard(Arc<Mutex<Option<SecretString>>>);

impl MemoryClipboard {
    /// A factory that hands this clipboard to the worker.
    pub fn factory(&self) -> BackendFactory {
        let clipboard = self.clone();
        Box::new(move || Some(Box::new(clipboard) as Box<dyn ClipboardBackend>))
    }

    /// What it holds (tests; a plain copy).
    #[cfg(test)]
    #[must_use]
    pub fn content(&self) -> Option<String> {
        self.0
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|s| s.expose_secret().to_owned()))
    }

    /// Replaces the content, as another app would (tests).
    #[cfg(test)]
    pub fn put(&self, text: &str) {
        if let Ok(mut content) = self.0.lock() {
            *content = Some(SecretString::from(text.to_owned()));
        }
    }
}

impl ClipboardBackend for MemoryClipboard {
    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.lock().map_err(|_| ClipboardError)? = Some(SecretString::from(text.to_owned()));
        Ok(())
    }

    fn read(&mut self) -> Option<SecretString> {
        let content = self.0.lock().ok()?;
        content
            .as_ref()
            .map(|s| SecretString::from(s.expose_secret().to_owned()))
    }

    fn clear(&mut self) -> Result<(), ClipboardError> {
        *self.0.lock().map_err(|_| ClipboardError)? = None;
        Ok(())
    }
}

enum Command {
    Copy {
        value: SecretString,
        clear_after: Option<Duration>,
        reply: Sender<Result<(), ClipboardError>>,
    },
    ClearIfOurs {
        reply: Sender<Result<bool, ClipboardError>>,
    },
    Shutdown,
}

/// What the worker copied last.
struct Held {
    value: SecretString,
    /// Monotonic and wall-clock deadline: the earlier one wins, so a suspend
    /// (monotonic clock stopped on Linux and macOS) cannot extend the secret's life.
    deadline: Option<(Instant, SystemTime)>,
}

/// Whether either clock has reached the deadline.
pub(crate) fn due(deadline: (Instant, SystemTime), mono: Instant, wall: SystemTime) -> bool {
    mono >= deadline.0 || wall >= deadline.1
}

struct Worker {
    backend: Option<Box<dyn ClipboardBackend>>,
    held: Option<Held>,
    events: Events,
}

impl Worker {
    fn copy(
        &mut self,
        value: SecretString,
        clear_after: Option<Duration>,
    ) -> Result<(), ClipboardError> {
        let backend = self.backend.as_mut().ok_or(ClipboardError)?;
        // A failed write may leave the previous value in place; forget ours
        // only once the new one is in.
        backend.write(value.expose_secret())?;
        self.held = Some(Held {
            value,
            deadline: clear_after.and_then(|d| {
                Some((
                    Instant::now().checked_add(d)?,
                    SystemTime::now().checked_add(d)?,
                ))
            }),
        });
        Ok(())
    }

    /// Clears the clipboard if it still holds our value; forgets the value
    /// either way. Returns whether it cleared.
    fn clear_if_ours(&mut self) -> Result<bool, ClipboardError> {
        let Some(held) = self.held.take() else {
            return Ok(false);
        };
        let Some(backend) = self.backend.as_mut() else {
            return Ok(false);
        };
        let ours = backend.read().is_some_and(|current| {
            bool::from(
                current
                    .expose_secret()
                    .as_bytes()
                    .ct_eq(held.value.expose_secret().as_bytes()),
            )
        });
        drop(held);
        if !ours {
            return Ok(false);
        }
        backend.clear()?;
        self.events.emit(&AppEvent::ClipboardCleared);
        Ok(true)
    }

    fn deadline(&self) -> Option<(Instant, SystemTime)> {
        self.held.as_ref().and_then(|h| h.deadline)
    }

    fn run(mut self, commands: &Receiver<Command>) {
        loop {
            let next = match self.deadline() {
                Some(deadline) => {
                    let wait = deadline
                        .0
                        .saturating_duration_since(Instant::now())
                        .min(MAX_WAIT);
                    match commands.recv_timeout(wait) {
                        Ok(command) => Some(command),
                        Err(RecvTimeoutError::Timeout) => {
                            if due(deadline, Instant::now(), SystemTime::now()) {
                                None
                            } else {
                                continue;
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => Some(Command::Shutdown),
                    }
                }
                None => Some(commands.recv().unwrap_or(Command::Shutdown)),
            };
            match next {
                None => {
                    // The timeout: a failed clear is retried by nobody; the
                    // value is forgotten so the next copy starts fresh.
                    let _ = self.clear_if_ours();
                }
                Some(Command::Copy {
                    value,
                    clear_after,
                    reply,
                }) => {
                    let _ = reply.send(self.copy(value, clear_after));
                }
                Some(Command::ClearIfOurs { reply }) => {
                    let _ = reply.send(self.clear_if_ours());
                }
                Some(Command::Shutdown) => {
                    let _ = self.clear_if_ours();
                    return;
                }
            }
        }
    }
}

/// The clipboard worker's handle.
pub struct ClipboardService {
    commands: Mutex<Sender<Command>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl ClipboardService {
    /// Starts the worker. `factory` runs on the worker thread; if it gives
    /// `None`, every copy fails with `clipboardUnavailable`.
    #[must_use]
    pub fn spawn(factory: BackendFactory, events: Events) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("mosaic-clipboard".into())
            .spawn(move || {
                Worker {
                    backend: factory(),
                    held: None,
                    events,
                }
                .run(&rx);
            })
            .ok();
        Self {
            commands: Mutex::new(tx),
            thread: Mutex::new(thread),
        }
    }

    fn send(&self, command: Command) -> IpcResult<()> {
        self.commands
            .lock()
            .map_err(|_| IpcError::INTERNAL)?
            .send(command)
            .map_err(|_| IpcError::new(ErrorCode::ClipboardUnavailable))
    }

    /// Writes `value` and clears it after `clear_after` (never if `None`).
    ///
    /// # Errors
    /// `clipboardUnavailable`.
    pub fn copy(&self, value: SecretString, clear_after: Option<Duration>) -> IpcResult<()> {
        let (reply, answer) = mpsc::channel();
        self.send(Command::Copy {
            value,
            clear_after,
            reply,
        })?;
        answer
            .recv()
            .map_err(|_| IpcError::new(ErrorCode::ClipboardUnavailable))?
            .map_err(|ClipboardError| IpcError::new(ErrorCode::ClipboardUnavailable))
    }

    /// Clears the clipboard now if it still holds what Mosaic copied.
    /// Returns whether it did.
    ///
    /// # Errors
    /// `clipboardUnavailable`.
    pub fn clear_if_ours(&self) -> IpcResult<bool> {
        let (reply, answer) = mpsc::channel();
        self.send(Command::ClearIfOurs { reply })?;
        answer
            .recv()
            .map_err(|_| IpcError::new(ErrorCode::ClipboardUnavailable))?
            .map_err(|ClipboardError| IpcError::new(ErrorCode::ClipboardUnavailable))
    }

    /// Clears what Mosaic copied and stops the worker (on exit).
    pub fn shutdown(&self) {
        let _ = self.send(Command::Shutdown);
        if let Ok(mut thread) = self.thread.lock()
            && let Some(handle) = thread.take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for ClipboardService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::testing::recording_channel;

    fn service() -> (
        ClipboardService,
        MemoryClipboard,
        crate::events::testing::Log,
    ) {
        let events = Events::default();
        let (channel, log) = recording_channel();
        events.subscribe(channel).unwrap();
        let clipboard = MemoryClipboard::default();
        (
            ClipboardService::spawn(clipboard.factory(), events),
            clipboard,
            log,
        )
    }

    fn secret(s: &str) -> SecretString {
        SecretString::from(s.to_owned())
    }

    fn wait_until(mut done: impl FnMut() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(start.elapsed() < Duration::from_secs(10), "timed out");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn due_on_either_clock() {
        let now = Instant::now();
        let wall = SystemTime::now();
        let later = Duration::from_secs(60);
        let deadline = (now + later, wall + later);
        assert!(!due(deadline, now, wall));
        assert!(due(deadline, now + later, wall), "monotonic alone");
        assert!(due(deadline, now, wall + later), "wall clock alone");
    }

    #[test]
    fn clears_after_the_timeout_and_tells_the_webview() {
        let (service, clipboard, log) = service();
        service
            .copy(secret("hunter2"), Some(Duration::from_millis(50)))
            .unwrap();
        assert_eq!(clipboard.content().as_deref(), Some("hunter2"));
        wait_until(|| clipboard.content().is_none());
        wait_until(|| !log.lock().unwrap().is_empty());
        assert_eq!(*log.lock().unwrap(), vec![r#"{"type":"clipboardCleared"}"#]);
    }

    #[test]
    fn never_clears_what_the_user_copied_since() {
        let (service, clipboard, log) = service();
        service
            .copy(secret("hunter2"), Some(Duration::from_millis(30)))
            .unwrap();
        clipboard.put("my own text");
        thread::sleep(Duration::from_millis(120));
        assert_eq!(clipboard.content().as_deref(), Some("my own text"));
        assert!(!service.clear_if_ours().unwrap(), "nothing of ours left");
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn a_new_copy_replaces_the_pending_timeout() {
        let (service, clipboard, _log) = service();
        service
            .copy(secret("first"), Some(Duration::from_millis(30)))
            .unwrap();
        service.copy(secret("second"), None).unwrap();
        thread::sleep(Duration::from_millis(120));
        assert_eq!(clipboard.content().as_deref(), Some("second"), "no clear");
        assert!(service.clear_if_ours().unwrap());
        assert_eq!(clipboard.content(), None);
    }

    #[test]
    fn shutdown_clears_our_value() {
        let (service, clipboard, _log) = service();
        service.copy(secret("hunter2"), None).unwrap();
        drop(service);
        assert_eq!(clipboard.content(), None);
    }

    #[test]
    fn no_clipboard_means_clipboard_unavailable() {
        let service = ClipboardService::spawn(Box::new(|| None), Events::default());
        assert_eq!(
            service.copy(secret("x"), None).unwrap_err().code,
            ErrorCode::ClipboardUnavailable
        );
        assert!(!service.clear_if_ours().unwrap());
    }

    struct Broken;
    impl ClipboardBackend for Broken {
        fn write(&mut self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError)
        }
        fn read(&mut self) -> Option<SecretString> {
            None
        }
        fn clear(&mut self) -> Result<(), ClipboardError> {
            Err(ClipboardError)
        }
    }

    #[test]
    fn a_failing_write_is_reported() {
        let service = ClipboardService::spawn(
            Box::new(|| Some(Box::new(Broken) as Box<dyn ClipboardBackend>)),
            Events::default(),
        );
        assert_eq!(
            service.copy(secret("x"), None).unwrap_err().code,
            ErrorCode::ClipboardUnavailable
        );
        assert!(!service.clear_if_ours().unwrap(), "nothing held");
    }
}
