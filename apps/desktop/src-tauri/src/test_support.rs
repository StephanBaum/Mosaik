//! Test-only helpers. Argon2id baseline (128 MiB, p = 4) per account creation/unlock;
//! tests that run it hold this permit so at most one derivation runs at a time.

static KDF: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn kdf_permit() -> std::sync::MutexGuard<'static, ()> {
    KDF.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A private endpoint for an SSH agent under test; the directory (Unix) lives
/// as long as this value.
pub(crate) struct TestEndpoint {
    pub(crate) endpoint: mosaic_ssh_agent::Endpoint,
    _dir: Option<tempfile::TempDir>,
}

pub(crate) fn ssh_test_endpoint() -> TestEndpoint {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    if cfg!(windows) {
        TestEndpoint {
            endpoint: mosaic_ssh_agent::Endpoint::NamedPipe(format!(
                r"\\.\pipe\mosaic-desktop-test-{}-{n}",
                std::process::id()
            )),
            _dir: None,
        }
    } else {
        let dir = tempfile::tempdir().unwrap();
        // tempdir() is 0755 under the usual umask, which the agent rightly refuses;
        // the agent creates this subdirectory itself with mode 0700.
        let endpoint = mosaic_ssh_agent::Endpoint::platform_default(&dir.path().join("ssh"));
        TestEndpoint {
            endpoint,
            _dir: Some(dir),
        }
    }
}

/// A screen that returns the frames it was given (once), counts the captures
/// and can run a hook "during" the capture (a lock, for example).
type CaptureHook = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub(crate) struct FakeScreenCapture {
    supported: bool,
    frames: std::sync::Arc<std::sync::Mutex<Vec<mosaic_qr::OwnedFrame>>>,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    on_capture: std::sync::Arc<std::sync::Mutex<Option<CaptureHook>>>,
}

impl FakeScreenCapture {
    pub(crate) fn new(supported: bool, frames: Vec<mosaic_qr::OwnedFrame>) -> Self {
        Self {
            supported,
            frames: std::sync::Arc::new(std::sync::Mutex::new(frames)),
            calls: std::sync::Arc::default(),
            on_capture: std::sync::Arc::default(),
        }
    }

    /// Replaces the frames the next capture returns.
    pub(crate) fn load(&self, frames: Vec<mosaic_qr::OwnedFrame>) {
        *self.frames.lock().unwrap() = frames;
    }

    /// How many times the screen was captured.
    pub(crate) fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Runs `hook` inside the next captures.
    pub(crate) fn on_capture(&self, hook: CaptureHook) {
        *self.on_capture.lock().unwrap() = Some(hook);
    }
}

impl crate::screen_capture::ScreenCapture for FakeScreenCapture {
    fn supported(&self) -> bool {
        self.supported
    }

    fn capture(&self) -> crate::contract::errors::IpcResult<Vec<mosaic_qr::OwnedFrame>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(hook) = self.on_capture.lock().unwrap().as_ref() {
            hook();
        }
        Ok(std::mem::take(&mut *self.frames.lock().unwrap()))
    }
}
