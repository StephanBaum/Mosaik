//! Test-only helpers. Argon2id baseline (128 MiB, p = 4) per account creation/unlock;
//! tests that run it hold this permit so at most one derivation runs at a time.

static KDF: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn kdf_permit() -> std::sync::MutexGuard<'static, ()> {
    KDF.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
