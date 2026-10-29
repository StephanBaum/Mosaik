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
