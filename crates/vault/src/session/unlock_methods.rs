//! Quick unlock and hardware-key unlock (CRYPTO_SPEC §6.2, §6.3, ADR-0028).
//!
//! The vault never talks to a platform keystore or a security key. The app
//! hands it the secret the platform released (`W` for quick unlock, the PRF
//! output of a security key) and the vault does the rest: derive, wrap,
//! store, unwrap, expire and count failures.
//!
//! - **Quick unlock** needs the master password once since the store was
//!   opened ([`LockedSession::password_required`]), expires after the
//!   policy's maximum age, and is deleted (wrapping and WAL frames) on
//!   expiry, on a wrapping that does not open, and on a master password
//!   change. Its `method_id` is a UUIDv7 drawn at enrollment; the timestamp
//!   inside is the enrollment time, authenticated through the AAD.
//! - **Hardware keys** are registered per account (at most
//!   [`MAX_HARDWARE_KEYS`]); five rejected unlocks in a row require the
//!   master password.

use core::time::Duration;

use mosaic_core::unlock::{
    MAX_CREDENTIAL_ID_LEN, MIN_CREDENTIAL_ID_LEN, PrfOutput, credential_id_hash, prf_unlock_key,
};
use mosaic_core::{DeviceId, Kek, MasterPassword, SecretKey, UnlockMethod};

use super::{Error, LockedSession, Result, UnlockError, UnlockedSession, unlock_account_key};
use crate::storage::{Error as StorageError, StoredUnlockMethod};

/// How long a quick-unlock enrollment is used (CRYPTO_SPEC §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuickUnlockPolicy {
    /// The maximum age of an enrollment.
    pub max_age: Duration,
}

impl QuickUnlockPolicy {
    /// Default maximum age: 14 days.
    pub const DEFAULT_MAX_AGE: Duration = Duration::from_hours(14 * 24);
    /// Shortest configurable maximum age: 1 day.
    pub const MIN_MAX_AGE: Duration = Duration::from_hours(24);
    /// Longest accepted maximum age: 30 days. Never exceeded (§6.2).
    pub const MAX_MAX_AGE: Duration = Duration::from_hours(30 * 24);

    /// A policy with `max_age` clamped to [`Self::MIN_MAX_AGE`] ..=
    /// [`Self::MAX_MAX_AGE`].
    #[must_use]
    pub fn new(max_age: Duration) -> Self {
        Self {
            max_age: max_age.clamp(Self::MIN_MAX_AGE, Self::MAX_MAX_AGE),
        }
    }
}

impl Default for QuickUnlockPolicy {
    fn default() -> Self {
        Self {
            max_age: Self::DEFAULT_MAX_AGE,
        }
    }
}

/// Grace for an enrollment time in the future (clock skew), §6.2.
pub const QUICK_UNLOCK_FUTURE_SKEW: Duration = Duration::from_secs(300);
/// Rejected quick or hardware-key unlocks in a row before the password is
/// required.
pub const MAX_FAILED_QUICK_ATTEMPTS: u8 = 5;
/// Hardware keys per account (§6.3) and per hardware-protected vault (§7.4).
pub const MAX_HARDWARE_KEYS: usize = 8;
/// The RP ID of desktop enrollments (§6.3, ADR-0028).
pub const DEFAULT_RP_ID: &str = "mosaic.invalid";

const MAX_RP_ID_LEN: usize = 253;
const HW_PARAMS_VERSION: u8 = 0x01;

fn duration_ms(d: Duration) -> i64 {
    i64::try_from(d.as_millis()).unwrap_or(i64::MAX)
}

/// A UUIDv7's version nibble and 48-bit millisecond timestamp.
fn uuid_v7_millis(id: &DeviceId) -> Option<i64> {
    let [t0, t1, t2, t3, t4, t5, version, ..] = *id.as_bytes();
    if version >> 4 != 7 {
        return None;
    }
    let ms = u64::from_be_bytes([0, 0, t0, t1, t2, t3, t4, t5]);
    i64::try_from(ms).ok()
}

/// A quick-unlock enrollment as stored (public data only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickUnlockEnrollment {
    /// The enrollment ID (`method_id`, UUIDv7).
    pub enrollment_id: DeviceId,
    /// Its timestamp, ms since the Unix epoch (from the UUIDv7,
    /// authenticated by the AAD).
    pub enrolled_at_ms: i64,
    /// The app's platform parameters (opaque to the vault, at most
    /// `MAX_UNLOCK_PARAMS_LEN`).
    pub platform_params: Vec<u8>,
}

impl QuickUnlockEnrollment {
    fn from_id(enrollment_id: DeviceId, platform_params: Vec<u8>) -> Self {
        Self {
            enrolled_at_ms: uuid_v7_millis(&enrollment_id).unwrap_or(0),
            enrollment_id,
            platform_params,
        }
    }

    pub(super) fn from_row(row: StoredUnlockMethod) -> Option<Self> {
        match row.method {
            UnlockMethod::QuickUnlock(id) => Some(Self::from_id(id, row.params)),
            _ => None,
        }
    }

    /// `enrolled_at + max_age`, in ms.
    #[must_use]
    pub fn expires_at_ms(&self, policy: &QuickUnlockPolicy) -> i64 {
        self.enrolled_at_ms
            .saturating_add(duration_ms(policy.max_age))
    }

    /// Whether the enrollment is expired at `now_ms`: past its maximum age,
    /// enrolled more than [`QUICK_UNLOCK_FUTURE_SKEW`] in the future, or its
    /// ID is not a UUIDv7 (so it carries no enrollment time).
    #[must_use]
    pub fn is_expired(&self, policy: &QuickUnlockPolicy, now_ms: i64) -> bool {
        if uuid_v7_millis(&self.enrollment_id).is_none() {
            return true;
        }
        if self.enrolled_at_ms > now_ms.saturating_add(duration_ms(QUICK_UNLOCK_FUTURE_SKEW)) {
            return true;
        }
        now_ms > self.expires_at_ms(policy)
    }
}

/// A security key to register (public data from the platform).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewHardwareKey {
    /// The WebAuthn credential ID (16 to 1023 bytes).
    pub credential_id: Vec<u8>,
    /// The RP ID the credential was created for.
    pub rp_id: String,
    /// `prf_salt` (§6.3) or `prf_salt_hw` (§7.4).
    pub prf_salt: [u8; 32],
}

fn valid_rp_id(rp_id: &str) -> bool {
    (1..=MAX_RP_ID_LEN).contains(&rp_id.len()) && rp_id.bytes().all(|b| (0x21..=0x7E).contains(&b))
}

fn valid_credential_id(credential_id: &[u8]) -> bool {
    (MIN_CREDENTIAL_ID_LEN..=MAX_CREDENTIAL_ID_LEN).contains(&credential_id.len())
}

impl NewHardwareKey {
    /// Validates and builds the registration data.
    ///
    /// # Errors
    /// [`Error::InvalidHardwareKey`] for a credential ID outside 16 to 1023
    /// bytes or an RP ID outside 1 to 253 bytes of printable ASCII without
    /// spaces.
    pub fn new(credential_id: Vec<u8>, rp_id: String, prf_salt: [u8; 32]) -> Result<Self> {
        let key = Self {
            credential_id,
            rp_id,
            prf_salt,
        };
        key.check()?;
        Ok(key)
    }

    /// Re-checks the fields (they are public, so a caller may have edited
    /// them after construction).
    pub(super) fn check(&self) -> Result<()> {
        if valid_credential_id(&self.credential_id) && valid_rp_id(&self.rp_id) {
            Ok(())
        } else {
            Err(Error::InvalidHardwareKey)
        }
    }

    /// `method_id = SHA-256(credential_id)[..16]` as an unlock method.
    pub(super) fn method(&self) -> Result<UnlockMethod> {
        credential_id_hash(&self.credential_id).map_err(|_| Error::InvalidHardwareKey)?;
        Ok(UnlockMethod::hardware_key(&self.credential_id))
    }
}

/// A registered security key (for unlock, or for one hardware-protected
/// vault).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareKeyEnrollment {
    /// `SHA-256(credential_id)[..16]`.
    pub method_id: [u8; 16],
    /// The WebAuthn credential ID.
    pub credential_id: Vec<u8>,
    /// The RP ID the credential was created for.
    pub rp_id: String,
    /// `prf_salt` (§6.3) or `prf_salt_hw` (§7.4).
    pub prf_salt: [u8; 32],
    /// The row's `updated_at` (display only; not security-relevant).
    pub added_at_ms: i64,
}

impl HardwareKeyEnrollment {
    pub(super) fn from_params(
        method_id: [u8; 16],
        params: &[u8],
        added_at_ms: i64,
    ) -> Result<Self> {
        let key = decode_hw_params(&method_id, params)?;
        Ok(Self {
            method_id,
            credential_id: key.credential_id,
            rp_id: key.rp_id,
            prf_salt: key.prf_salt,
            added_at_ms,
        })
    }
}

/// How a vault's keys are protected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VaultProtection {
    /// Wrapped under the account key.
    AccountKey,
    /// §7.4; `open` while this session holds TK and the vault keys.
    HardwareKey {
        /// Whether the vault is open in this session.
        open: bool,
    },
}

/// The quick methods [`LockedSession::password_required`] asks about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickMethod {
    /// Quick unlock with Windows Hello or Touch ID (§6.2).
    Biometric,
    /// A security key (§6.3).
    HardwareKey,
}

/// What the locked session remembers about the password and the failed
/// quick attempts.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct QuickState {
    /// Whether the master password unlocked since the store was opened.
    pub(super) password_unlocked: bool,
    /// Rejected quick or hardware-key unlocks in a row.
    pub(super) failed_quick: u8,
}

impl QuickState {
    /// After a successful password unlock.
    pub(super) const PASSWORD: Self = Self {
        password_unlocked: true,
        failed_quick: 0,
    };
}

// ---------------------------------------------------------------------------
// Hardware-key params codec (§6.3)
// ---------------------------------------------------------------------------

/// `0x01 || prf_salt[32] || rp_id_len:u8 || rp_id || cred_len:u16 || credential_id`.
pub(super) fn encode_hw_params(key: &NewHardwareKey) -> Result<Vec<u8>> {
    key.check()?;
    let rp_len = u8::try_from(key.rp_id.len()).map_err(|_| Error::InvalidHardwareKey)?;
    let cred_len = u16::try_from(key.credential_id.len()).map_err(|_| Error::InvalidHardwareKey)?;
    let mut out = Vec::with_capacity(1 + 32 + 1 + key.rp_id.len() + 2 + key.credential_id.len());
    out.push(HW_PARAMS_VERSION);
    out.extend_from_slice(&key.prf_salt);
    out.push(rp_len);
    out.extend_from_slice(key.rp_id.as_bytes());
    out.extend_from_slice(&cred_len.to_be_bytes());
    out.extend_from_slice(&key.credential_id);
    Ok(out)
}

/// Strict inverse of [`encode_hw_params`]: exact length, version `0x01`,
/// and `method_id == SHA-256(credential_id)[..16]`. Any failure is
/// [`StorageError::Corrupt`].
pub(super) fn decode_hw_params(method_id: &[u8; 16], bytes: &[u8]) -> Result<NewHardwareKey> {
    decode_hw_params_inner(method_id, bytes).ok_or(Error::Store(StorageError::Corrupt))
}

fn decode_hw_params_inner(method_id: &[u8; 16], bytes: &[u8]) -> Option<NewHardwareKey> {
    let (&version, rest) = bytes.split_first()?;
    if version != HW_PARAMS_VERSION {
        return None;
    }
    let (salt, rest) = rest.split_first_chunk::<32>()?;
    let (&rp_len, rest) = rest.split_first()?;
    let (rp_id, rest) = rest.split_at_checked(usize::from(rp_len))?;
    let (cred_len, credential_id) = rest.split_first_chunk::<2>()?;
    if credential_id.len() != usize::from(u16::from_be_bytes(*cred_len)) {
        return None;
    }
    let rp_id = core::str::from_utf8(rp_id).ok()?;
    if !valid_rp_id(rp_id) || !valid_credential_id(credential_id) {
        return None;
    }
    let UnlockMethod::HardwareKey(expected) = UnlockMethod::hardware_key(credential_id) else {
        return None;
    };
    if &expected != method_id {
        return None;
    }
    Some(NewHardwareKey {
        credential_id: credential_id.to_vec(),
        rp_id: rp_id.to_owned(),
        prf_salt: *salt,
    })
}

// ---------------------------------------------------------------------------
// Store helpers
// ---------------------------------------------------------------------------

/// Every quick-unlock row, newest first (UUIDv7 sorts by time).
fn quick_rows(rows: Vec<StoredUnlockMethod>) -> Vec<StoredUnlockMethod> {
    let mut quick: Vec<_> = rows
        .into_iter()
        .filter(|r| matches!(r.method, UnlockMethod::QuickUnlock(_)))
        .collect();
    quick.sort_by_key(|r| match r.method {
        UnlockMethod::QuickUnlock(id) => core::cmp::Reverse(*id.as_bytes()),
        _ => core::cmp::Reverse([0; 16]),
    });
    quick
}

fn hw_rows(rows: Vec<StoredUnlockMethod>) -> Vec<StoredUnlockMethod> {
    rows.into_iter()
        .filter(|r| matches!(r.method, UnlockMethod::HardwareKey(_)))
        .collect()
}

fn hw_enrollments(rows: Vec<StoredUnlockMethod>) -> Result<Vec<HardwareKeyEnrollment>> {
    hw_rows(rows)
        .into_iter()
        .map(|row| match row.method {
            UnlockMethod::HardwareKey(id) => {
                HardwareKeyEnrollment::from_params(id, &row.params, row.updated_at_ms)
            }
            _ => Err(Error::Store(StorageError::Corrupt)),
        })
        .collect()
}

/// Deletes every quick-unlock row (wrapping and WAL frames) and returns the
/// newest one.
fn delete_quick(store: &mut crate::storage::Store) -> Result<Option<QuickUnlockEnrollment>> {
    let rows = quick_rows(store.unlock_methods()?);
    for row in &rows {
        store.remove_unlock_method(&row.method)?;
    }
    Ok(rows
        .into_iter()
        .next()
        .and_then(QuickUnlockEnrollment::from_row))
}

// ---------------------------------------------------------------------------
// LockedSession
// ---------------------------------------------------------------------------

impl LockedSession {
    /// Whether the quick method is refused now: no password unlock since the
    /// store was opened (quick unlock only), or
    /// [`MAX_FAILED_QUICK_ATTEMPTS`] rejected attempts in a row (both).
    #[must_use]
    pub fn password_required(&self, method: QuickMethod) -> bool {
        self.quick.failed_quick >= MAX_FAILED_QUICK_ATTEMPTS
            || (method == QuickMethod::Biometric && !self.quick.password_unlocked)
    }

    /// The stored quick-unlock enrollment, if any (no side effects).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn quick_unlock_enrollment(&self) -> Result<Option<QuickUnlockEnrollment>> {
        Ok(quick_rows(self.store.unlock_methods()?)
            .into_iter()
            .next()
            .and_then(QuickUnlockEnrollment::from_row))
    }

    /// Deletes the quick-unlock enrollment (wrapping, WAL scrubbed) and
    /// returns it so the app can delete the platform key. Used for expiry
    /// found while locked and for reconciling a missing platform key.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn remove_quick_unlock(&mut self) -> Result<Option<QuickUnlockEnrollment>> {
        delete_quick(&mut self.store)
    }

    fn fail(self, error: Error) -> UnlockError {
        UnlockError {
            session: self,
            error,
        }
    }

    /// Unlocks with the quick-unlock `W` (§6.2).
    ///
    /// Order: [`Error::NoAccount`], [`Error::PasswordRequired`], then
    /// [`Error::NoQuickUnlock`]; an expired enrollment is deleted and answers
    /// [`Error::QuickUnlockExpired`]; a `W` that does not open the wrapping
    /// deletes the enrollment, counts as a failed attempt and answers
    /// [`Error::QuickUnlockInvalidated`].
    ///
    /// # Errors
    /// As above, plus [`Error::Store`] and [`Error::Core`].
    pub fn unlock_quick(
        mut self,
        w: &Kek,
        policy: &QuickUnlockPolicy,
        now_ms: i64,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        let account = match self.store.account() {
            Ok(Some(a)) => a,
            Ok(None) => return Err(self.fail(Error::NoAccount)),
            Err(e) => return Err(self.fail(e.into())),
        };
        if self.password_required(QuickMethod::Biometric) {
            return Err(self.fail(Error::PasswordRequired));
        }
        let rows = match self.store.unlock_methods() {
            Ok(rows) => quick_rows(rows),
            Err(e) => return Err(self.fail(e.into())),
        };
        let Some(row) = rows.into_iter().next() else {
            return Err(self.fail(Error::NoQuickUnlock));
        };
        let wrapped = row.wrapped_account_key.clone();
        let method = row.method;
        let Some(enrollment) = QuickUnlockEnrollment::from_row(row) else {
            return Err(self.fail(Error::NoQuickUnlock));
        };
        if enrollment.is_expired(policy, now_ms) {
            return Err(match delete_quick(&mut self.store) {
                Ok(_) => self.fail(Error::QuickUnlockExpired),
                Err(e) => self.fail(e),
            });
        }
        match wrapped.unwrap_account_key(w, &account.account_id, &method) {
            Ok(ak) => {
                let state = QuickState {
                    password_unlocked: self.quick.password_unlocked,
                    failed_quick: 0,
                };
                UnlockedSession::load(self.store, account.account_id, ak, state)
            }
            Err(mosaic_core::Error::Decryption) => {
                self.quick.failed_quick = self.quick.failed_quick.saturating_add(1);
                Err(match delete_quick(&mut self.store) {
                    Ok(_) => self.fail(Error::QuickUnlockInvalidated),
                    Err(e) => self.fail(e),
                })
            }
            Err(e) => Err(self.fail(e.into())),
        }
    }

    /// The registered unlock security keys, ordered by `method_id`.
    ///
    /// # Errors
    /// [`Error::Store`] (`Corrupt` for a row whose parameters do not decode).
    pub fn hardware_keys(&self) -> Result<Vec<HardwareKeyEnrollment>> {
        hw_enrollments(self.store.unlock_methods()?)
    }

    /// Unlocks with a security key (§6.3).
    ///
    /// Order: [`Error::NoAccount`], [`Error::PasswordRequired`] (only the
    /// failed-attempt limit applies, not the app-start rule),
    /// [`Error::InvalidHardwareKey`], [`Error::UnknownHardwareKey`]; a PRF
    /// output that does not open the wrapping counts as a failed attempt and
    /// answers [`Error::HardwareKeyRejected`]. The wrapping is kept.
    ///
    /// # Errors
    /// As above, plus [`Error::Store`] and [`Error::Core`].
    pub fn unlock_with_hardware_key(
        mut self,
        credential_id: &[u8],
        prf: &PrfOutput,
    ) -> core::result::Result<UnlockedSession, UnlockError> {
        let account = match self.store.account() {
            Ok(Some(a)) => a,
            Ok(None) => return Err(self.fail(Error::NoAccount)),
            Err(e) => return Err(self.fail(e.into())),
        };
        if self.password_required(QuickMethod::HardwareKey) {
            return Err(self.fail(Error::PasswordRequired));
        }
        if credential_id_hash(credential_id).is_err() {
            return Err(self.fail(Error::InvalidHardwareKey));
        }
        let method = UnlockMethod::hardware_key(credential_id);
        let row = match self.store.unlock_method(&method) {
            Ok(Some(row)) => row,
            Ok(None) => return Err(self.fail(Error::UnknownHardwareKey)),
            Err(e) => return Err(self.fail(e.into())),
        };
        let Ok(w) = prf_unlock_key(prf, credential_id) else {
            return Err(self.fail(Error::InvalidHardwareKey));
        };
        match row
            .wrapped_account_key
            .unwrap_account_key(&w, &account.account_id, &method)
        {
            Ok(ak) => {
                let state = QuickState {
                    password_unlocked: self.quick.password_unlocked,
                    failed_quick: 0,
                };
                UnlockedSession::load(self.store, account.account_id, ak, state)
            }
            Err(mosaic_core::Error::Decryption) => {
                self.quick.failed_quick = self.quick.failed_quick.saturating_add(1);
                Err(self.fail(Error::HardwareKeyRejected))
            }
            Err(e) => Err(self.fail(e.into())),
        }
    }
}

// ---------------------------------------------------------------------------
// UnlockedSession
// ---------------------------------------------------------------------------

impl UnlockedSession {
    /// Checks the master password and Secret Key against the stored password
    /// wrapping; the account key it opens must equal the one this session
    /// holds (constant time). Nothing changes, so a wrong password does not
    /// lock (M3-R8); the app counts failures and locks after five.
    ///
    /// # Errors
    /// [`Error::UnlockFailed`], [`Error::NoAccount`], [`Error::Core`]
    /// (empty password, allocation), [`Error::Store`].
    pub fn verify_master_password(
        &self,
        password: &MasterPassword,
        secret_key: &SecretKey,
    ) -> Result<()> {
        let (_, check) = unlock_account_key(&self.store, password, secret_key)?;
        if check == self.account_key {
            Ok(())
        } else {
            Err(Error::UnlockFailed)
        }
    }

    /// The stored quick-unlock enrollment, if any.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn quick_unlock_enrollment(&self) -> Result<Option<QuickUnlockEnrollment>> {
        Ok(quick_rows(self.store.unlock_methods()?)
            .into_iter()
            .next()
            .and_then(QuickUnlockEnrollment::from_row))
    }

    /// Enrolls quick unlock (§6.2): a new enrollment ID (UUIDv7), AK wrapped
    /// under `w`, the previous enrollment deleted. The new wrapping is stored
    /// first, then the old rows are removed (each removal scrubs the WAL).
    /// Returns the new enrollment and the one it replaced, whose platform key
    /// the app must delete.
    ///
    /// The caller proves the master password first
    /// ([`Self::verify_master_password`]); this method does not.
    ///
    /// # Errors
    /// [`Error::Store`] (`UnlockParamsTooLarge`), [`Error::Core`].
    pub fn enroll_quick_unlock(
        &mut self,
        w: &Kek,
        platform_params: &[u8],
    ) -> Result<(QuickUnlockEnrollment, Option<QuickUnlockEnrollment>)> {
        let id = DeviceId::new_v7()?;
        let method = UnlockMethod::QuickUnlock(id);
        let wrapped = self.account_key.wrap(w, &self.account_id, &method)?;
        let old = quick_rows(self.store.unlock_methods()?);
        self.store
            .put_unlock_method(&method, &wrapped, platform_params)?;
        for row in &old {
            self.store.remove_unlock_method(&row.method)?;
        }
        let replaced = old
            .into_iter()
            .next()
            .and_then(QuickUnlockEnrollment::from_row);
        Ok((
            QuickUnlockEnrollment::from_id(id, platform_params.to_vec()),
            replaced,
        ))
    }

    /// Deletes the quick-unlock enrollment and returns it, so the app
    /// deletes the platform key.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn remove_quick_unlock(&mut self) -> Result<Option<QuickUnlockEnrollment>> {
        delete_quick(&mut self.store)
    }

    /// Registers a security key for unlock (§6.3): derives `W` from the PRF
    /// output, wraps AK, stores the public parameters. Replacing a
    /// registered credential does not count against [`MAX_HARDWARE_KEYS`].
    ///
    /// # Errors
    /// [`Error::InvalidHardwareKey`], [`Error::HardwareKeyLimit`],
    /// [`Error::Store`], [`Error::Core`].
    pub fn enroll_hardware_key(
        &mut self,
        key: &NewHardwareKey,
        prf: &PrfOutput,
    ) -> Result<HardwareKeyEnrollment> {
        let method = key.method()?;
        let params = encode_hw_params(key)?;
        let rows = hw_rows(self.store.unlock_methods()?);
        let replaces = rows.iter().any(|r| r.method == method);
        if !replaces && rows.len() >= MAX_HARDWARE_KEYS {
            return Err(Error::HardwareKeyLimit);
        }
        let w = prf_unlock_key(prf, &key.credential_id).map_err(|_| Error::InvalidHardwareKey)?;
        let wrapped = self.account_key.wrap(&w, &self.account_id, &method)?;
        self.store.put_unlock_method(&method, &wrapped, &params)?;
        let row = self
            .store
            .unlock_method(&method)?
            .ok_or(Error::Store(StorageError::Corrupt))?;
        let UnlockMethod::HardwareKey(method_id) = method else {
            return Err(Error::InvalidHardwareKey);
        };
        HardwareKeyEnrollment::from_params(method_id, &row.params, row.updated_at_ms)
    }

    /// Removes a registered security key. Returns whether it was registered.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn remove_hardware_key(&mut self, method_id: &[u8; 16]) -> Result<bool> {
        Ok(self
            .store
            .remove_unlock_method(&UnlockMethod::HardwareKey(*method_id))?)
    }

    /// The registered unlock security keys, ordered by `method_id`.
    ///
    /// # Errors
    /// [`Error::Store`] (`Corrupt` for a row whose parameters do not decode).
    pub fn hardware_keys(&self) -> Result<Vec<HardwareKeyEnrollment>> {
        hw_enrollments(self.store.unlock_methods()?)
    }
}
