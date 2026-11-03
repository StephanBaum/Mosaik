//! Fake platforms for the command tests: no OS prompt, no device, and
//! counters and hooks to observe and disturb the flows.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_core::{AccountId, Kek, SecretBytes};

use super::{BiometricPlatform, PlatformError, params};
use crate::contract::quick_unlock::BiometricPlatformKind;

/// A hook that runs inside a platform call, without any lock of the
/// backend held (so it may lock or unlock the backend).
pub(crate) type Hook = Box<dyn Fn() + Send + Sync>;

/// A biometric platform that keeps its `W` in memory.
pub(crate) struct FakeBiometric {
    /// What `is_available` answers.
    pub(crate) available: AtomicBool,
    /// The platform key's `W`; `None` when the key does not exist.
    w: Mutex<Option<[u8; 32]>>,
    /// Errors the next `enroll` and `release` calls return, in order.
    pub(crate) next: Mutex<VecDeque<PlatformError>>,
    /// User prompts so far (`enroll` and `release`).
    pub(crate) prompts: AtomicUsize,
    /// Calls of `remove`.
    pub(crate) removals: AtomicUsize,
    /// Runs inside `enroll`, before it answers.
    pub(crate) on_enroll: Mutex<Option<Hook>>,
    /// Runs inside `release`, before it answers.
    pub(crate) on_release: Mutex<Option<Hook>>,
}

impl FakeBiometric {
    pub(crate) fn new(available: bool) -> Arc<Self> {
        Arc::new(Self {
            available: AtomicBool::new(available),
            w: Mutex::new(None),
            next: Mutex::default(),
            prompts: AtomicUsize::new(0),
            removals: AtomicUsize::new(0),
            on_enroll: Mutex::new(None),
            on_release: Mutex::new(None),
        })
    }

    pub(crate) fn prompts(&self) -> usize {
        self.prompts.load(Ordering::SeqCst)
    }

    pub(crate) fn removals(&self) -> usize {
        self.removals.load(Ordering::SeqCst)
    }

    pub(crate) fn has_key(&self) -> bool {
        self.w.lock().is_ok_and(|w| w.is_some())
    }

    /// The next prompt answers `error`.
    pub(crate) fn fail_next(&self, error: PlatformError) {
        if let Ok(mut next) = self.next.lock() {
            next.push_back(error);
        }
    }

    /// The platform key now produces another `W` (a changed fingerprint
    /// set, a reset Hello).
    pub(crate) fn rotate_key(&self) {
        if let (Ok(mut w), Ok(other)) = (self.w.lock(), Kek::generate()) {
            *w = Some(*other.expose_secret());
        }
    }

    /// Deletes the platform key behind the service's back.
    pub(crate) fn lose_key(&self) {
        if let Ok(mut w) = self.w.lock() {
            *w = None;
        }
    }

    /// Creates a platform key behind the service's back (an orphan).
    pub(crate) fn plant_key(&self) {
        self.rotate_key();
    }

    fn run_hook(hook: &Mutex<Option<Hook>>) {
        // Not held while the hook runs: it may call back into the platform.
        let hook = hook.lock().ok().and_then(|mut h| h.take());
        if let Some(hook) = hook {
            hook();
        }
    }

    fn prompt(&self, hook: &Mutex<Option<Hook>>) -> Result<(), PlatformError> {
        self.prompts.fetch_add(1, Ordering::SeqCst);
        Self::run_hook(hook);
        match self.next.lock().ok().and_then(|mut n| n.pop_front()) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn kek(bytes: &[u8; 32]) -> Result<Kek, PlatformError> {
        SecretBytes::<32>::from_slice(bytes)
            .map(Kek::from_secret)
            .map_err(|_| PlatformError::Failed)
    }
}

impl BiometricPlatform for Arc<FakeBiometric> {
    fn kind(&self) -> BiometricPlatformKind {
        BiometricPlatformKind::WindowsHello
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    fn key_exists(&self, _account: AccountId) -> bool {
        self.has_key()
    }

    fn enroll(&self, _account: AccountId) -> Result<(Vec<u8>, Kek), PlatformError> {
        self.prompt(&self.on_enroll)?;
        let w = Kek::generate().map_err(|_| PlatformError::Failed)?;
        *self.w.lock().map_err(|_| PlatformError::Failed)? = Some(*w.expose_secret());
        let payload = params::encode(BiometricPlatformKind::WindowsHello, &[7; 32])
            .ok_or(PlatformError::Failed)?;
        Ok((payload, w))
    }

    fn release(&self, _account: AccountId, payload: &[u8]) -> Result<Kek, PlatformError> {
        self.prompt(&self.on_release)?;
        if params::body_for(BiometricPlatformKind::WindowsHello, payload).is_none() {
            return Err(PlatformError::Unavailable);
        }
        let w = self.w.lock().map_err(|_| PlatformError::Failed)?;
        (*w).as_ref()
            .map_or(Err(PlatformError::Unavailable), FakeBiometric::kek)
    }

    fn remove(&self, _account: AccountId) -> Result<(), PlatformError> {
        self.removals.fetch_add(1, Ordering::SeqCst);
        self.lose_key();
        Ok(())
    }
}
