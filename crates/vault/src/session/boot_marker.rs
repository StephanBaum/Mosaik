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
}

impl LockedSession {
    /// Sets the "password unlocked since start" flag of CRYPTO_SPEC §6.2 if and
    /// only if the stored marker equals `boot` (constant time). Returns whether
    /// it matched. It never resets the counter of failed quick unlocks.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn adopt_boot_marker(&mut self, boot: &BootId) -> Result<bool> {
        let matched = self.store.boot_marker()?.is_some_and(|marker| {
            marker.boot_id.len() == boot.0.len() && bool::from(marker.boot_id.ct_eq(&boot.0))
        });
        if matched {
            self.quick.password_unlocked = true;
        }
        Ok(matched)
    }
}
