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
use mosaic_qr::{OwnedFrame, PixelLayout};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::events::Events;

/// Longest the worker sleeps between looks at the wall clock, so a suspend
/// (where the monotonic clock may stop) delays the clear by at most this.
pub const MAX_WAIT: Duration = Duration::from_secs(1);

/// How long the worker waits before retrying a clear that failed.
const RETRY: Duration = MAX_WAIT;

/// Tries a clear gets at shutdown (a busy clipboard) before the worker gives
/// up and the value stays (PROGRESS M3-R7).
const SHUTDOWN_TRIES: u32 = 5;
/// Pause between those tries.
const SHUTDOWN_PAUSE: Duration = Duration::from_millis(100);

/// What the clipboard holds, as an image.
#[derive(Debug)]
pub enum ClipboardImage {
    /// No image on the clipboard.
    Empty,
    /// An image the OS could not convert.
    Unreadable,
    /// The image, RGBA with straight alpha, in a zeroizing buffer.
    Image(OwnedFrame),
}

/// The OS clipboard, as the worker uses it.
pub trait ClipboardBackend {
    /// Writes `text`, excluded from history and monitoring where supported.
    ///
    /// # Errors
    /// When the clipboard cannot be written.
    fn write(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// The current text, or `None` if the clipboard holds no text (or text
    /// that cannot be ours).
    ///
    /// # Errors
    /// When the clipboard cannot be read right now (for example another
    /// process holds it): the caller must not assume anything about its
    /// content.
    fn read(&mut self) -> Result<Option<SecretString>, ClipboardError>;

    /// Empties the clipboard.
    ///
    /// # Errors
    /// When the clipboard cannot be written.
    fn clear(&mut self) -> Result<(), ClipboardError>;

    /// The clipboard image (arboard decodes it; RGBA, straight alpha).
    /// `ContentNotAvailable` is `Empty`, `ConversionFailure` is `Unreadable`.
    ///
    /// # Errors
    /// When the clipboard cannot be read right now.
    fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError>;
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

    fn read(&mut self) -> Result<Option<SecretString>, ClipboardError> {
        match self.0.get_text() {
            Ok(text) => Ok(Some(SecretString::from(text))),
            Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => {
                Ok(None)
            }
            Err(_) => Err(ClipboardError),
        }
    }

    fn clear(&mut self) -> Result<(), ClipboardError> {
        self.0.clear().map_err(|_| ClipboardError)
    }

    fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
        match self.0.get_image() {
            Ok(image) => {
                let (Ok(width), Ok(height)) =
                    (u32::try_from(image.width), u32::try_from(image.height))
                else {
                    return Ok(ClipboardImage::Unreadable);
                };
                let pixels = Zeroizing::new(image.bytes.into_owned());
                Ok(OwnedFrame::new(width, height, PixelLayout::Rgba8, pixels)
                    .map_or(ClipboardImage::Unreadable, ClipboardImage::Image))
            }
            Err(arboard::Error::ContentNotAvailable) => Ok(ClipboardImage::Empty),
            Err(arboard::Error::ConversionFailure) => Ok(ClipboardImage::Unreadable),
            Err(_) => Err(ClipboardError),
        }
    }
}

/// An in-process clipboard, the default until the app installs the system
/// one, and for tests. Clones share the content, so a test can play "the
/// user copied something else". The content is a zeroizing secret string.
#[derive(Clone, Default)]
pub struct MemoryClipboard {
    text: Arc<Mutex<Option<SecretString>>>,
    image: Arc<Mutex<Option<OwnedFrame>>>,
}

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
        self.text
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|s| s.expose_secret().to_owned()))
    }

    /// Replaces the content, as another app would (tests).
    #[cfg(test)]
    pub fn put(&self, text: &str) {
        if let Ok(mut content) = self.text.lock() {
            *content = Some(SecretString::from(text.to_owned()));
        }
    }

    /// Puts an image on the clipboard, as a screenshot tool would (tests).
    #[cfg(test)]
    pub fn put_image(&self, frame: OwnedFrame) {
        if let Ok(mut image) = self.image.lock() {
            *image = Some(frame);
        }
    }

    /// Whether an image is on the clipboard (tests).
    #[cfg(test)]
    #[must_use]
    pub fn has_image(&self) -> bool {
        self.image.lock().is_ok_and(|image| image.is_some())
    }
}

impl ClipboardBackend for MemoryClipboard {
    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.text.lock().map_err(|_| ClipboardError)? = Some(SecretString::from(text.to_owned()));
        Ok(())
    }

    fn read(&mut self) -> Result<Option<SecretString>, ClipboardError> {
        let content = self.text.lock().map_err(|_| ClipboardError)?;
        Ok(content
            .as_ref()
            .map(|s| SecretString::from(s.expose_secret().to_owned())))
    }

    fn clear(&mut self) -> Result<(), ClipboardError> {
        *self.text.lock().map_err(|_| ClipboardError)? = None;
        *self.image.lock().map_err(|_| ClipboardError)? = None;
        Ok(())
    }

    fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
        let image = self.image.lock().map_err(|_| ClipboardError)?.take();
        Ok(image.map_or(ClipboardImage::Empty, ClipboardImage::Image))
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
    ReadImage {
        reply: Sender<Result<ClipboardImage, ClipboardError>>,
    },
    ClearAny {
        reply: Sender<Result<(), ClipboardError>>,
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

    /// Schedules another clear attempt after [`RETRY`] (also for values
    /// copied without a timeout: once a clear was asked for, it is retried).
    fn retry_later(held: &mut Option<Held>) {
        if let Some(h) = held.as_mut() {
            h.deadline = Instant::now()
                .checked_add(RETRY)
                .zip(SystemTime::now().checked_add(RETRY));
        }
    }

    /// Clears the clipboard if it still holds our value. The value is
    /// forgotten once the outcome is known: cleared, or not ours any more.
    /// If the clipboard cannot be read or cleared, the value is kept and the
    /// clear is retried after [`RETRY`]. Returns whether it cleared.
    fn clear_if_ours(&mut self) -> Result<bool, ClipboardError> {
        let (Some(held), Some(backend)) = (self.held.as_ref(), self.backend.as_mut()) else {
            self.held = None;
            return Ok(false);
        };
        let current = match backend.read() {
            Ok(current) => current,
            Err(e) => {
                Self::retry_later(&mut self.held);
                return Err(e);
            }
        };
        let ours = current.is_some_and(|current| {
            bool::from(
                current
                    .expose_secret()
                    .as_bytes()
                    .ct_eq(held.value.expose_secret().as_bytes()),
            )
        });
        if !ours {
            self.held = None;
            return Ok(false);
        }
        if let Err(e) = backend.clear() {
            Self::retry_later(&mut self.held);
            return Err(e);
        }
        self.held = None;
        self.events.emit(&AppEvent::ClipboardCleared);
        Ok(true)
    }

    /// Empties the clipboard whatever it holds (a pasted screenshot is not
    /// ours, but it is the secret in another form), and forgets any held
    /// value: there is nothing left to clear later.
    fn clear_any(&mut self) -> Result<(), ClipboardError> {
        let backend = self.backend.as_mut().ok_or(ClipboardError)?;
        backend.clear()?;
        self.held = None;
        Ok(())
    }

    fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
        self.backend.as_mut().ok_or(ClipboardError)?.read_image()
    }

    fn deadline(&self) -> Option<(Instant, SystemTime)> {
        self.held.as_ref().and_then(|h| h.deadline)
    }

    /// Waits for the next command, or for the held value's deadline. `None`:
    /// woke up early, nothing is due yet.
    fn wait(&self, commands: &Receiver<Command>) -> Option<Wake> {
        let Some(deadline) = self.deadline() else {
            return Some(Wake::Command(commands.recv().unwrap_or(Command::Shutdown)));
        };
        let wait = deadline
            .0
            .saturating_duration_since(Instant::now())
            .min(MAX_WAIT);
        match commands.recv_timeout(wait) {
            Ok(command) => Some(Wake::Command(command)),
            Err(RecvTimeoutError::Timeout) => {
                due(deadline, Instant::now(), SystemTime::now()).then_some(Wake::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => Some(Wake::Command(Command::Shutdown)),
        }
    }

    /// Clears our value before the worker ends, with a few retries.
    fn shutdown(&mut self) {
        for attempt in 0..SHUTDOWN_TRIES {
            if self.clear_if_ours().is_ok() {
                break;
            }
            if attempt + 1 < SHUTDOWN_TRIES {
                thread::sleep(SHUTDOWN_PAUSE);
            }
        }
    }

    fn run(mut self, commands: &Receiver<Command>) {
        loop {
            let Some(wake) = self.wait(commands) else {
                continue;
            };
            match wake {
                Wake::Timeout => {
                    // A failed clear keeps the value and retries every RETRY
                    // until it succeeds or the content changed.
                    let _ = self.clear_if_ours();
                }
                Wake::Command(Command::Copy {
                    value,
                    clear_after,
                    reply,
                }) => {
                    let _ = reply.send(self.copy(value, clear_after));
                }
                Wake::Command(Command::ClearIfOurs { reply }) => {
                    let _ = reply.send(self.clear_if_ours());
                }
                Wake::Command(Command::ReadImage { reply }) => {
                    let _ = reply.send(self.read_image());
                }
                Wake::Command(Command::ClearAny { reply }) => {
                    let _ = reply.send(self.clear_any());
                }
                Wake::Command(Command::Shutdown) => {
                    self.shutdown();
                    return;
                }
            }
        }
    }
}

/// What woke the worker.
enum Wake {
    /// A command arrived.
    Command(Command),
    /// The held value's deadline passed.
    Timeout,
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

    /// The image on the clipboard, read on the worker (ADR-0031).
    ///
    /// # Errors
    /// `clipboardUnavailable`.
    pub fn read_image(&self) -> IpcResult<ClipboardImage> {
        let (reply, answer) = mpsc::channel();
        self.send(Command::ReadImage { reply })?;
        answer
            .recv()
            .map_err(|_| IpcError::new(ErrorCode::ClipboardUnavailable))?
            .map_err(|ClipboardError| IpcError::new(ErrorCode::ClipboardUnavailable))
    }

    /// Empties the clipboard whatever it holds.
    ///
    /// # Errors
    /// `clipboardUnavailable`.
    pub fn clear_any(&self) -> IpcResult<()> {
        let (reply, answer) = mpsc::channel();
        self.send(Command::ClearAny { reply })?;
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        fn read(&mut self) -> Result<Option<SecretString>, ClipboardError> {
            Ok(None)
        }
        fn clear(&mut self) -> Result<(), ClipboardError> {
            Err(ClipboardError)
        }
        fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
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

    /// A clipboard that fails some reads and clears first.
    struct Flaky {
        inner: MemoryClipboard,
        fail_reads: Arc<AtomicUsize>,
        fail_clears: Arc<AtomicUsize>,
    }

    fn take_one(n: &AtomicUsize) -> bool {
        n.try_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
            .is_ok()
    }

    impl ClipboardBackend for Flaky {
        fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
            self.inner.write(text)
        }
        fn read(&mut self) -> Result<Option<SecretString>, ClipboardError> {
            if take_one(&self.fail_reads) {
                return Err(ClipboardError);
            }
            self.inner.read()
        }
        fn clear(&mut self) -> Result<(), ClipboardError> {
            if take_one(&self.fail_clears) {
                return Err(ClipboardError);
            }
            self.inner.clear()
        }
        fn read_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
            self.inner.read_image()
        }
    }

    fn flaky_service(
        reads: usize,
        clears: usize,
    ) -> (
        ClipboardService,
        MemoryClipboard,
        crate::events::testing::Log,
        Arc<AtomicUsize>,
    ) {
        let events = Events::default();
        let (channel, log) = recording_channel();
        events.subscribe(channel).unwrap();
        let clipboard = MemoryClipboard::default();
        let fail_reads = Arc::new(AtomicUsize::new(reads));
        let backend = Flaky {
            inner: clipboard.clone(),
            fail_reads: Arc::clone(&fail_reads),
            fail_clears: Arc::new(AtomicUsize::new(clears)),
        };
        let service = ClipboardService::spawn(
            Box::new(move || Some(Box::new(backend) as Box<dyn ClipboardBackend>)),
            events,
        );
        (service, clipboard, log, fail_reads)
    }

    #[test]
    fn a_busy_clipboard_is_cleared_on_retry() {
        let (service, clipboard, log, _) = flaky_service(2, 1);
        service
            .copy(secret("pw"), Some(Duration::from_millis(10)))
            .unwrap();
        wait_until(|| clipboard.content().is_none());
        wait_until(|| !log.lock().unwrap().is_empty());
        assert_eq!(*log.lock().unwrap(), vec![r#"{"type":"clipboardCleared"}"#]);
    }

    #[test]
    fn a_failed_lock_clear_keeps_retrying() {
        let (service, clipboard, _log, _) = flaky_service(1, 0);
        service.copy(secret("pw"), None).unwrap();
        assert!(service.clear_if_ours().is_err());
        wait_until(|| clipboard.content().is_none());
    }

    #[test]
    fn content_changed_during_retry_is_left_alone() {
        let (service, clipboard, log, reads) = flaky_service(1, 0);
        service
            .copy(secret("pw"), Some(Duration::from_millis(10)))
            .unwrap();
        wait_until(|| reads.load(Ordering::SeqCst) == 0);
        clipboard.put("mine");
        thread::sleep(Duration::from_millis(1_500));
        assert_eq!(clipboard.content().as_deref(), Some("mine"));
        assert!(log.lock().unwrap().is_empty());
        drop(service);
    }
}
