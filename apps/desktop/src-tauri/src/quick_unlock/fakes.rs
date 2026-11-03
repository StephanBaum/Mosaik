//! Fake platforms for the command tests: no OS prompt, no device, and
//! counters and hooks to observe and disturb the flows.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_core::unlock::PrfOutput;
use mosaic_core::{Kek, SecretBytes, SecretString};

use super::{
    BiometricPlatform, CancelFlag, HardwareCandidate, HardwareKeyError, HardwareKeyPlatform,
    PlatformError, params,
};
use crate::contract::quick_unlock::BiometricPlatformKind;

/// A hook that runs inside a platform call, without any lock of the
/// backend held (so it may lock or unlock the backend).
pub(crate) type Hook = Box<dyn Fn() + Send + Sync>;

/// A biometric platform that keeps its `W` in memory.
pub(crate) struct FakeBiometric {
    /// What `is_available` answers.
    pub(crate) available: AtomicBool,
    /// The platform keys: `W` by key nonce (empty when no key exists).
    w: Mutex<HashMap<[u8; 16], [u8; 32]>>,
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
            w: Mutex::new(HashMap::new()),
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
        self.w.lock().is_ok_and(|w| !w.is_empty())
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
        if let Ok(mut w) = self.w.lock() {
            for value in w.values_mut() {
                if let Ok(other) = Kek::generate() {
                    *value = *other.expose_secret();
                }
            }
        }
    }

    /// Deletes the platform key behind the service's back.
    pub(crate) fn lose_key(&self) {
        if let Ok(mut w) = self.w.lock() {
            w.clear();
        }
    }

    /// How many platform keys exist.
    pub(crate) fn key_count(&self) -> usize {
        self.w.lock().map_or(0, |w| w.len())
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

    fn key_exists(&self, payload: &[u8]) -> bool {
        params::body_for(BiometricPlatformKind::WindowsHello, payload)
            .is_some_and(|(nonce, _)| self.w.lock().is_ok_and(|w| w.contains_key(&nonce)))
    }

    fn enroll(&self) -> Result<(Vec<u8>, Kek), PlatformError> {
        self.prompt(&self.on_enroll)?;
        let w = Kek::generate().map_err(|_| PlatformError::Failed)?;
        let nonce = params::new_key_nonce().ok_or(PlatformError::Failed)?;
        self.w
            .lock()
            .map_err(|_| PlatformError::Failed)?
            .insert(nonce, *w.expose_secret());
        let payload = params::encode(BiometricPlatformKind::WindowsHello, &nonce, &[7; 32])
            .ok_or(PlatformError::Failed)?;
        Ok((payload, w))
    }

    fn release(&self, payload: &[u8]) -> Result<Kek, PlatformError> {
        self.prompt(&self.on_release)?;
        let (nonce, _) = params::body_for(BiometricPlatformKind::WindowsHello, payload)
            .ok_or(PlatformError::Unavailable)?;
        let w = self.w.lock().map_err(|_| PlatformError::Failed)?;
        w.get(&nonce)
            .map_or(Err(PlatformError::Unavailable), FakeBiometric::kek)
    }

    fn remove(&self, payload: &[u8]) -> Result<(), PlatformError> {
        self.removals.fetch_add(1, Ordering::SeqCst);
        // A payload that is not well-formed still counts as a call.
        if let Some((nonce, _)) = params::body_for(BiometricPlatformKind::WindowsHello, payload) {
            self.w
                .lock()
                .map_err(|_| PlatformError::Failed)?
                .remove(&nonce);
        }
        Ok(())
    }
}

/// The credentials on one physical key: credential ID, PRF salt, output.
type Credentials = HashMap<Vec<u8>, HashMap<[u8; 32], [u8; 32]>>;

/// A security key (or several) that keeps its credentials in memory.
pub(crate) struct FakeHardwareKey {
    /// What `is_supported` answers.
    pub(crate) supported: AtomicBool,
    /// Whether the keys have a PIN set (`PIN`).
    pub(crate) needs_pin: AtomicBool,
    /// Every credential on every physical key: key index, credential ID,
    /// PRF salt, PRF output.
    keys: Mutex<Vec<Credentials>>,
    /// The physical key that is plugged in.
    plugged: Mutex<Option<usize>>,
    /// Calls into `register` and `evaluate`.
    pub(crate) calls: AtomicUsize,
    /// The raw PRF salts the fake was asked to evaluate (the real transport
    /// hashes them with `prf_salt_input`).
    pub(crate) salts: Mutex<Vec<[u8; 32]>>,
    /// Answer `evaluate` with an output that opens nothing.
    pub(crate) corrupt_prf: AtomicBool,
    /// Runs inside `register` and `evaluate`, before the checks.
    pub(crate) on_call: Mutex<Option<Hook>>,
}

/// The PIN of a fake key that has one.
pub(crate) const FAKE_PIN: &str = "135790";

impl FakeHardwareKey {
    /// `keys` physical keys; key 0 is plugged in.
    pub(crate) fn new(keys: usize) -> Arc<Self> {
        Arc::new(Self {
            supported: AtomicBool::new(true),
            needs_pin: AtomicBool::new(false),
            keys: Mutex::new(vec![HashMap::new(); keys]),
            plugged: Mutex::new(Some(0)),
            calls: AtomicUsize::new(0),
            salts: Mutex::new(Vec::new()),
            corrupt_prf: AtomicBool::new(false),
            on_call: Mutex::new(None),
        })
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Plugs in physical key `index`, or none.
    pub(crate) fn plug(&self, index: Option<usize>) {
        if let Ok(mut plugged) = self.plugged.lock() {
            *plugged = index;
        }
    }

    fn random() -> Result<[u8; 32], HardwareKeyError> {
        SecretBytes::<32>::random()
            .map(|b| *b.expose_secret())
            .map_err(|_| HardwareKeyError::Failed)
    }

    fn prf(bytes: &[u8; 32]) -> Result<PrfOutput, HardwareKeyError> {
        SecretBytes::<32>::from_slice(bytes)
            .map(PrfOutput::from_secret)
            .map_err(|_| HardwareKeyError::Failed)
    }

    /// The common start of both operations: counts, runs the hook, checks
    /// support, cancel, device and PIN. Returns the plugged key's index.
    fn begin(
        &self,
        pin: Option<&SecretString>,
        cancel: &CancelFlag,
    ) -> Result<usize, HardwareKeyError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let hook = self.on_call.lock().ok().and_then(|mut h| h.take());
        if let Some(hook) = hook {
            hook();
        }
        if !self.supported.load(Ordering::SeqCst) {
            return Err(HardwareKeyError::Unsupported);
        }
        if cancel.is_cancelled() {
            return Err(HardwareKeyError::Cancelled);
        }
        let plugged = (*self.plugged.lock().map_err(|_| HardwareKeyError::Failed)?)
            .ok_or(HardwareKeyError::NoDevice)?;
        if self.needs_pin.load(Ordering::SeqCst) {
            match pin.map(SecretString::expose_secret) {
                None => return Err(HardwareKeyError::PinRequired),
                Some(given) if given != FAKE_PIN => return Err(HardwareKeyError::PinInvalid),
                Some(_) => {}
            }
        }
        Ok(plugged)
    }
}

impl HardwareKeyPlatform for Arc<FakeHardwareKey> {
    fn is_supported(&self) -> bool {
        self.supported.load(Ordering::SeqCst)
    }

    fn register(
        &self,
        _rp_id: &str,
        prf_salt: &[u8; 32],
        pin: Option<&SecretString>,
        cancel: &CancelFlag,
    ) -> Result<(Vec<u8>, PrfOutput), HardwareKeyError> {
        let plugged = self.begin(pin, cancel)?;
        let credential = FakeHardwareKey::random()?.to_vec();
        let output = FakeHardwareKey::random()?;
        self.salts
            .lock()
            .map_err(|_| HardwareKeyError::Failed)?
            .push(*prf_salt);
        self.keys
            .lock()
            .map_err(|_| HardwareKeyError::Failed)?
            .get_mut(plugged)
            .ok_or(HardwareKeyError::Failed)?
            .insert(credential.clone(), HashMap::from([(*prf_salt, output)]));
        Ok((credential, FakeHardwareKey::prf(&output)?))
    }

    fn evaluate(
        &self,
        candidates: &[HardwareCandidate<'_>],
        pin: Option<&SecretString>,
        cancel: &CancelFlag,
    ) -> Result<(usize, PrfOutput), HardwareKeyError> {
        let plugged = self.begin(pin, cancel)?;
        let mut keys = self.keys.lock().map_err(|_| HardwareKeyError::Failed)?;
        let device = keys.get_mut(plugged).ok_or(HardwareKeyError::Failed)?;
        let (index, candidate) = candidates
            .iter()
            .enumerate()
            .find(|(_, c)| device.contains_key(c.credential_id))
            .ok_or(HardwareKeyError::NotRecognized)?;
        self.salts
            .lock()
            .map_err(|_| HardwareKeyError::Failed)?
            .push(*candidate.prf_salt);
        let output = if self.corrupt_prf.load(Ordering::SeqCst) {
            FakeHardwareKey::random()?
        } else {
            let salts = device
                .get_mut(candidate.credential_id)
                .ok_or(HardwareKeyError::Failed)?;
            if let Some(output) = salts.get(candidate.prf_salt) {
                *output
            } else {
                let output = FakeHardwareKey::random()?;
                salts.insert(*candidate.prf_salt, output);
                output
            }
        };
        Ok((index, FakeHardwareKey::prf(&output)?))
    }
}
