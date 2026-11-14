//! The boot marker API (CRYPTO_SPEC §6.9.2, ADR-0090): on a phone, quick
//! unlock is allowed once the master password unlocked **since the device
//! booted**, not since the app started.
//!
//! The platform passes the current boot ID ([`BootId`]). After a master-password
//! unlock the app records it ([`UnlockedSession::record_boot_marker`]); a later
//! process of the app adopts it ([`LockedSession::adopt_boot_marker`]) and then
//! may quick-unlock. The desktop and the CLI never write a marker and keep the
//! app-start rule. Deletion paths are in the storage layer and in
//! [`UnlockedSession::lock_requiring_password`].

use subtle::ConstantTimeEq;

use super::unlock_methods::MAX_FAILED_QUICK_ATTEMPTS;
use super::{Error, LockedSession, Result, UnlockedSession};
use crate::storage::MAX_BOOT_ID_LEN;

/// The boot ID of the current device boot: 1 to 32 bytes (Android: the boot
/// count as a big-endian `u32`; iOS: `kern.boottime` as big-endian
/// `u64 sec || u64 usec`). Not secret.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BootId(Vec<u8>);

impl BootId {
    /// A boot ID.
    ///
    /// # Errors
    /// [`Error::InvalidInput`] unless `bytes` is 1 to 32 bytes (the platform
    /// passes an empty value when it could not read the boot ID, so no marker
    /// can match and the password is required).
    pub fn new(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_BOOT_ID_LEN {
            return Err(Error::InvalidInput);
        }
        Ok(Self(bytes.to_vec()))
    }

    /// The bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl UnlockedSession {
    /// Records that the master password unlocked in this device boot,
    /// replacing any earlier marker.
    ///
    /// Only right after a master-password unlock of **this** session; a
    /// session from a quick unlock, a security key or a CLI session never
    /// writes or refreshes a marker.
    ///
    /// # Errors
    /// [`Error::ReauthenticationRequired`], [`Error::Store`].
    pub fn record_boot_marker(&mut self, boot: &BootId, now_ms: i64) -> Result<()> {
        if !self.password_fresh {
            return Err(Error::ReauthenticationRequired);
        }
        self.store.put_boot_marker(boot.as_bytes(), now_ms)?;
        Ok(())
    }

    /// Counts one wrong master password at a re-authentication and returns
    /// the count. It is stored with the vault, so a restart of the app does
    /// not start it again; only a right master password ends it
    /// ([`Self::reset_failed_reauth`]). The count is written before the caller
    /// answers the wrong password.
    ///
    /// # Errors
    /// [`Error::Store`]; the caller then treats the limit as reached.
    pub fn note_failed_reauth(&mut self) -> Result<u8> {
        Ok(self.store.note_reauth_failure()?)
    }

    /// Ends the count of wrong re-authentications (a right master password,
    /// at a re-authentication or an unlock).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn reset_failed_reauth(&mut self) -> Result<()> {
        Ok(self.store.reset_reauth_failures()?)
    }
}

impl LockedSession {
    /// Sets the "password unlocked since start" flag of CRYPTO_SPEC §6.2 if and
    /// only if the stored marker equals `boot` (constant time) **and** the
    /// store accepts a write (the marker is rewritten unchanged in a write
    /// transaction), so a store that cannot be written never lets a marker be
    /// adopted: the failed-attempt limit could not be enforced on it. It also
    /// refuses when [`MAX_FAILED_QUICK_ATTEMPTS`] wrong re-authentications are
    /// stored (the password is then required). Returns whether it matched. It
    /// never resets the counter of failed quick unlocks.
    ///
    /// # Errors
    /// [`Error::Store`] (the marker is then not adopted).
    pub fn adopt_boot_marker(&mut self, boot: &BootId) -> Result<bool> {
        if self.store.reauth_failures()? >= MAX_FAILED_QUICK_ATTEMPTS {
            self.quick.failed_quick = MAX_FAILED_QUICK_ATTEMPTS;
            let _ = self.store.clear_boot_marker();
            return Ok(false);
        }
        let matched = self.store.boot_marker()?.is_some_and(|marker| {
            marker.boot_id.len() == boot.0.len() && bool::from(marker.boot_id.ct_eq(&boot.0))
        });
        if matched {
            self.store.touch_boot_marker()?;
            self.quick.password_unlocked = true;
        }
        Ok(matched)
    }

    /// Requires the master password from now on: the failed-attempt counter
    /// is set to its limit and the boot marker is deleted (or overwritten), as
    /// [`UnlockedSession::lock_requiring_password`] does. For a strict lock
    /// that finds the session locked already. The delete cannot fail loudly;
    /// the counter still refuses quick unlock in this session, and
    /// [`Self::enforce_boot_marker_limit`] tries again.
    pub fn require_password(&mut self) {
        self.quick.failed_quick = MAX_FAILED_QUICK_ATTEMPTS;
        let _ = self.store.clear_boot_marker();
    }
}
