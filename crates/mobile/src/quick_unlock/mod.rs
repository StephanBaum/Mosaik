//! Quick unlock on a phone (CRYPTO_SPEC §6.2, §6.9.2, ADR-0090): the
//! enrolment flow, the availability the platform acts on, and removal.
//!
//! The platform makes the key and encrypts `W`; Rust draws `W`, names the key
//! and decides what is stored and deleted. The enrolment is two calls: `begin`
//! verifies the master password and returns `W` once (a pending slot of 120 s);
//! `finish` stores the wrapping with the platform's parameters.

pub mod params;

use std::sync::Arc;

use mosaic_core::Kek;
use mosaic_vault::session::{LockedSession, QuickMethod, QuickUnlockPolicy, Reauthenticated};
use zeroize::Zeroizing;

use crate::Core;
use crate::config::Platform;
use crate::error::{MobileError, Result};
use crate::quick_unlock::params::{KEY_NONCE_LEN, Params, key_alias, quick_unlock_aad};
use crate::secret::{SecretValue, text_of};
use crate::state::{Phase, QuickUnlockAvailability, QuickUnlockPlatform};

/// How long a pending enrolment waits for the platform (CRYPTO_SPEC §6.2).
const PENDING_TTL_MS: u64 = 120_000;

/// A pending enrolment: the proof of the password check and `W`.
pub(crate) struct PendingEnroll {
    reauth: Reauthenticated,
    w: Kek,
    key_nonce: [u8; KEY_NONCE_LEN],
    expires_at_ms: u64,
}

/// What the platform needs to make the key and encrypt `W`.
pub struct QuickUnlockSetup {
    /// The alias (Android) or tag suffix (iOS) of the platform key.
    pub key_alias: String,
    /// The random per-enrolment nonce.
    pub key_nonce: Vec<u8>,
    /// The GCM AAD of the Android body (empty on iOS).
    pub aad: Vec<u8>,
    /// `W`, 32 bytes. The platform encrypts it, wipes its copy and calls
    /// `wipe()`.
    pub w: Arc<SecretValue>,
    /// When the pending enrolment is dropped (Unix ms).
    pub expires_at_ms: u64,
}

/// The result of an enrolment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickUnlockEnrolled {
    /// The key of the enrolment this one replaced; the platform deletes it.
    pub replaced_key_alias: Option<String>,
}

/// What the platform can do now (see [`QuickUnlockAvailability`]). Deletes an
/// enrolment that ran out or that this platform cannot use, and remembers its
/// key alias until a new enrolment or [`Core::quick_unlock_remove`].
pub(crate) fn availability(
    platform: Platform,
    policy: &QuickUnlockPolicy,
    now_ms: u64,
    locked: &mut LockedSession,
    expired_alias: &mut Option<String>,
) -> QuickUnlockAvailability {
    let enrollment = locked.quick_unlock_enrollment().ok().flatten();
    let Some(enrollment) = enrollment else {
        return expired_alias
            .clone()
            .map_or(QuickUnlockAvailability::NotEnrolled, |key_alias| {
                QuickUnlockAvailability::Expired { key_alias }
            });
    };
    let now = i64::try_from(now_ms).unwrap_or(i64::MAX);
    let decoded = Params::decode(&enrollment.platform_params, platform)
        .ok()
        .filter(|_| !enrollment.is_expired(policy, now));
    let Some(decoded) = decoded else {
        let alias = Params::decode(&enrollment.platform_params, platform)
            .ok()
            .map(|p| p.key_alias());
        let _ = locked.remove_quick_unlock();
        expired_alias.clone_from(&alias);
        return alias.map_or(QuickUnlockAvailability::NotEnrolled, |key_alias| {
            QuickUnlockAvailability::Expired { key_alias }
        });
    };
    if locked.password_required(QuickMethod::Biometric) {
        return QuickUnlockAvailability::PasswordRequired;
    }
    QuickUnlockAvailability::Available {
        platform: decoded.platform,
        key_alias: decoded.key_alias(),
        aad: decoded.aad(),
        body: decoded.body,
    }
}

impl Core {
    /// The platform row of this core's platform, for names and AADs (the
    /// `StrongBox` flag is not part of either).
    const fn platform_row(&self) -> QuickUnlockPlatform {
        match self.config.platform {
            Platform::Ios => QuickUnlockPlatform::IosSecureEnclave,
            Platform::Android => QuickUnlockPlatform::AndroidKeystore { strongbox: false },
        }
    }

    /// Starts an enrolment: checks the master password (a second Argon2id),
    /// draws `W` and the key nonce, and holds them for 120 s.
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `locked`, `emptyPassword`,
    /// `wrongCredentials`, `secretKeyRequired`.
    pub fn quick_unlock_enroll_begin(&self, password: Vec<u8>) -> Result<QuickUnlockSetup> {
        self.require_app()?;
        let password = text_of(password)?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        let reauth = self.verify_password(&password)?;
        let w = Kek::generate()?;
        let mut key_nonce = [0u8; KEY_NONCE_LEN];
        getrandom::fill(&mut key_nonce).map_err(|_| MobileError::Internal)?;
        let platform = self.platform_row();
        let w_value = SecretValue::new(Zeroizing::new(w.expose_secret().to_vec()));
        let expires_at_ms = self.now_ms().saturating_add(PENDING_TTL_MS);
        {
            let mut guard = self.guard()?;
            if !matches!(guard.phase, Phase::Unlocked(_)) {
                return Err(MobileError::Locked);
            }
            guard.slots.quick = Some(PendingEnroll {
                reauth,
                w,
                key_nonce,
                expires_at_ms,
            });
        }
        Ok(QuickUnlockSetup {
            key_alias: key_alias(platform, &key_nonce),
            key_nonce: key_nonce.to_vec(),
            aad: match platform {
                QuickUnlockPlatform::IosSecureEnclave => Vec::new(),
                QuickUnlockPlatform::AndroidKeystore { .. } => quick_unlock_aad(&key_nonce),
            },
            w: self.issue(w_value),
            expires_at_ms,
        })
    }

    /// Finishes an enrolment with the platform's ciphertext of `W`. The old
    /// enrolment is replaced in one transaction.
    ///
    /// # Errors
    /// `invalidInput` (the platform or the body does not fit this core),
    /// `reauthRequired` (no pending enrolment, or it ran out), `locked`.
    pub fn quick_unlock_enroll_finish(
        &self,
        platform: QuickUnlockPlatform,
        body: Vec<u8>,
    ) -> Result<QuickUnlockEnrolled> {
        self.require_app()?;
        let fits = matches!(
            (platform, self.config.platform),
            (QuickUnlockPlatform::IosSecureEnclave, Platform::Ios)
                | (
                    QuickUnlockPlatform::AndroidKeystore { .. },
                    Platform::Android
                )
        );
        if !fits {
            return Err(MobileError::InvalidInput);
        }
        let now = self.now_ms();
        let pending = {
            let mut guard = self.guard()?;
            if !matches!(guard.phase, Phase::Unlocked(_)) {
                return Err(MobileError::Locked);
            }
            guard.slots.quick.take()
        };
        let pending = pending
            .filter(|p| now <= p.expires_at_ms)
            .ok_or(MobileError::ReauthRequired)?;
        let params = match Params::new(platform, pending.key_nonce, body) {
            Ok(p) => p,
            Err(e) => {
                // A wrong body does not spend the pending enrolment.
                if let Ok(mut guard) = self.guard() {
                    guard.slots.quick = Some(pending);
                }
                return Err(e);
            }
        };
        let (_, replaced) = self.with_session(|s| {
            s.enroll_quick_unlock(&pending.w, &params.encode(), &pending.reauth)
                .map_err(MobileError::from)
        })?;
        self.update_info(|i| i.quick_unlock_enrolled = true);
        if let Ok(mut guard) = self.guard() {
            guard.expired_alias = None;
        }
        Ok(QuickUnlockEnrolled {
            replaced_key_alias: replaced
                .and_then(|e| Params::decode(&e.platform_params, self.config.platform).ok())
                .map(|p| p.key_alias()),
        })
    }

    /// Deletes the enrolment (locked or unlocked) and returns the key alias
    /// for the platform to delete: the enrolment's, or the one of an enrolment
    /// that ran out earlier and has not been reported as deleted yet.
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `busy`, `storeUnavailable`.
    pub fn quick_unlock_remove(&self) -> Result<Option<String>> {
        self.require_app()?;
        self.remove_enrollment(false)
    }

    /// The platform's key stopped working (the OS invalidated it). Deletes the
    /// wrapping, counts a failed attempt while locked, and returns the alias.
    ///
    /// # Errors
    /// `busy`, `storeUnavailable`.
    pub fn quick_unlock_platform_failed(&self) -> Result<Option<String>> {
        self.remove_enrollment(true)
    }

    fn remove_enrollment(&self, failed: bool) -> Result<Option<String>> {
        use mosaic_sync::wire::SessionHandle as _;
        let platform = self.config.platform;
        let alias_of = |e: mosaic_vault::session::QuickUnlockEnrollment| {
            Params::decode(&e.platform_params, platform)
                .ok()
                .map(|p| p.key_alias())
        };
        let owned = {
            let mut guard = self.guard()?;
            let inner = &mut *guard;
            match &mut inner.phase {
                Phase::Locked(ls) => {
                    let removed = ls.remove_quick_unlock()?;
                    if failed {
                        ls.note_failed_hardware_attempt();
                    }
                    let alias = removed
                        .and_then(alias_of)
                        .or_else(|| inner.expired_alias.take());
                    inner.expired_alias = None;
                    return Ok(alias);
                }
                Phase::Unlocked(u) => u.owned.clone(),
                Phase::NoAccount(_) => return Ok(None),
                Phase::Closed => return Err(MobileError::StoreUnavailable),
                Phase::Unlocking | Phase::Busy => return Err(MobileError::Busy),
            }
        };
        let removed = owned
            .with_session_mut(|s| s.remove_quick_unlock().map_err(MobileError::from))
            .ok_or(MobileError::Locked)??;
        self.update_info(|i| i.quick_unlock_enrolled = false);
        let mut guard = self.guard()?;
        let stale = guard.expired_alias.take();
        Ok(removed.and_then(alias_of).or(stale))
    }
}
