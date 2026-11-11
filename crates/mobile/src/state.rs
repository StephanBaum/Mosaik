//! The state a `Core` reports and the state it keeps (CRYPTO_SPEC §6.9, ADR-0094).

use mosaic_core::AccountId;
use mosaic_sync::engine::OwnedSession;
use mosaic_vault::session::LockedSession;

use crate::error::MobileError;
use crate::join::PendingJoin;
use crate::otp::PendingOtps;
use crate::passkeys::PasskeySessions;
use crate::quick_unlock::PendingEnroll;
use crate::settings::SettingsGuard;

/// The platform row of a quick-unlock enrolment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickUnlockPlatform {
    /// iOS Secure Enclave (platform byte `0x03`).
    IosSecureEnclave,
    /// Android Keystore (platform byte `0x04`); `strongbox` is bit 0 of the flags (`StrongBox`).
    AndroidKeystore {
        /// The key lives in `StrongBox`.
        strongbox: bool,
    },
}

/// Whether and how the phone can quick-unlock now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuickUnlockAvailability {
    /// No enrolment.
    NotEnrolled,
    /// An enrolment exists but the master password is needed first: no boot
    /// marker for this boot, or five failed attempts.
    PasswordRequired,
    /// The platform decrypts `body` with the key `key_alias` (and, on
    /// Android, the GCM AAD `aad`) and passes `W` to `unlock_quick`.
    Available {
        /// The platform row.
        platform: QuickUnlockPlatform,
        /// The alias of the platform key.
        key_alias: String,
        /// The public parameters the platform decrypts.
        body: Vec<u8>,
        /// The GCM AAD (Android only; empty on iOS).
        aad: Vec<u8>,
    },
    /// The enrolment ran out or its key stopped working. Rust has deleted the
    /// wrapping; the platform deletes the key. Reported until a new
    /// enrolment replaces it.
    Expired {
        /// The alias of the platform key to delete.
        key_alias: String,
    },
}

/// What the core is doing, for the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// No account on this phone: onboarding.
    NoAccount,
    /// An account, locked.
    Locked {
        /// The account email, if known.
        email: Option<String>,
        /// Whether the Secret Key is in secure storage (else the user types it).
        secret_key_stored: bool,
        /// Quick unlock.
        quick_unlock: QuickUnlockAvailability,
    },
    /// Unlocked.
    Unlocked {
        /// The account ID (canonical UUID).
        account_id: String,
        /// The account email, if known.
        email: Option<String>,
        /// Whether a quick-unlock enrolment exists.
        quick_unlock_enrolled: bool,
    },
    /// The store is closed (`close_store`); call `reopen_store`.
    StoreClosed,
    /// Another process holds the store (the app, or the extension).
    StoreBusy,
    /// A long call (unlock, join, reset) is running. Deviation from the
    /// five states of the plan: `state()` must be total while one runs.
    Busy,
}

/// What the unlocked phase caches, so `state()` never waits for the session.
#[derive(Clone, Debug)]
pub(crate) struct UnlockedInfo {
    pub account: AccountId,
    pub email: Option<String>,
    pub quick_unlock_enrolled: bool,
}

/// The unlocked session and what `state()` shows of it.
pub(crate) struct UnlockedPhase {
    pub owned: OwnedSession,
    pub info: UnlockedInfo,
}

/// The phase of the core (the plan's `Closed`, `NoAccount`, `Locked`,
/// `Unlocking`, `Unlocked`, `Busy`; the pending join is a slot, so it holds no
/// store).
pub(crate) enum Phase {
    Closed,
    NoAccount(LockedSession),
    Locked(LockedSession),
    Unlocking,
    Unlocked(Box<UnlockedPhase>),
    Busy,
}

/// Pending slots. Every slot except the join is dropped on lock; the join
/// survives backgrounding for its five minutes (CRYPTO_SPEC §6.9, ADR-0091).
#[derive(Default)]
pub(crate) struct Slots {
    pub quick: Option<PendingEnroll>,
    pub otp: PendingOtps,
    pub join: Option<PendingJoin>,
    pub passkeys: PasskeySessions,
}

impl Slots {
    /// Drops every slot but the join.
    pub(crate) fn clear_on_lock(&mut self) {
        self.quick = None;
        self.otp.clear();
        self.passkeys.clear();
    }
}

/// The mutable state of a `Core`, behind its mutex.
pub(crate) struct Inner {
    pub phase: Phase,
    /// `Closed` because another process holds the store.
    pub store_busy: bool,
    /// The platform key to delete after an enrolment expired or was
    /// invalidated (reported by `state()` until a new enrolment).
    pub expired_alias: Option<String>,
    pub slots: Slots,
    pub settings: SettingsGuard,
    /// When the last sync run succeeded (Unix ms).
    pub last_sync_at_ms: Option<u64>,
    /// How the last sync run failed, until one succeeds.
    pub last_sync_error: Option<MobileError>,
}
