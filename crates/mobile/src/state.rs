//! The state a `Core` reports and the state it keeps (CRYPTO_SPEC §6.9, ADR-0094).

use mosaic_core::AccountId;
use mosaic_sync::engine::OwnedSession;
use mosaic_vault::session::LockedSession;

use crate::autofill::FillSessions;
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

/// How the session in the unlocked phase was opened. Both ways verify the
/// user (the master password, or the platform's biometric key through `W`);
/// the iOS passkey calls require it (G1-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnlockKind {
    /// The master password and the Secret Key (also a join or a new account).
    Password,
    /// `W`, which the platform released after a biometric check.
    Quick,
}

/// What the unlocked phase caches, so `state()` never waits for the session.
#[derive(Clone, Debug)]
pub(crate) struct UnlockedInfo {
    pub account: AccountId,
    pub email: Option<String>,
    pub quick_unlock_enrolled: bool,
    pub unlock: UnlockKind,
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

/// Pending slots. Every slot is dropped on lock, the pending join included: a
/// join holds the Secret Key and the derived keys, and must not outlive a lock
/// (G1-11). The one exception is `close_store()`, which iOS calls when the app
/// goes to the background while the user is in Safari for the security-key
/// handoff: an unexpired join survives that (ADR-0091); the apps must not call
/// `lock()` during the handoff. An expired join is dropped whenever the state
/// is read.
#[derive(Default)]
pub(crate) struct Slots {
    pub quick: Option<PendingEnroll>,
    pub otp: PendingOtps,
    pub join: Option<PendingJoin>,
    pub passkeys: PasskeySessions,
    pub fill: FillSessions,
}

impl Slots {
    /// Drops every slot (the pending join too, unless `keep_join`).
    pub(crate) fn clear_on_lock(&mut self, keep_join: bool) {
        self.quick = None;
        self.otp.clear();
        self.passkeys.clear();
        self.fill.clear();
        if !keep_join {
            self.join = None;
        }
    }

    /// Drops a pending join that ran out; `true` when one was dropped.
    pub(crate) fn drop_expired_join(&mut self, now_ms: u64) -> bool {
        let expired = self.join.as_ref().is_some_and(|j| j.expired(now_ms));
        if expired {
            self.join = None;
        }
        expired
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
    /// The Secret Key of the account is in secure storage. Cached: asked of the
    /// platform (`contains`, never `get`) outside the mutex at open and set
    /// where the core stores or deletes it, so `state()` never calls the
    /// platform (G1-2).
    pub secret_key_stored: bool,
    /// A session token is in secure storage (cached like `secret_key_stored`).
    pub signed_in: bool,
    /// Requests that came while a phase was held out. Read and changed only
    /// under the core's mutex, in the same step that decides the phase.
    pub pending: Pending,
}

/// Requests that arrived while an unlock, a long call or a reopen held the
/// phase out. Whoever settles the phase takes them (`Core::finish_lock`,
/// `Inner::close_if_requested`).
#[derive(Debug, Default)]
pub(crate) struct Pending {
    /// `lock()` or `close_store()` came: the settled session ends locked.
    pub lock: bool,
    /// The pending lock must also require the master password (the
    /// re-authentication limit was hit while a long call held the session).
    pub strict: bool,
    /// `close_store()` came while an unlock, a long call or a reopen was in
    /// flight (G3i-1, G3i-2): after the lock, the store is closed.
    pub close: bool,
    /// A `reopen_store()` is opening the store.
    pub reopening: bool,
    /// A join is between its pending slot and its end (the network fetch,
    /// the unlock): a `lock()` or `close_store()` that comes meanwhile is
    /// armed for it, never settled away, and `join_cancel` and the expiry
    /// leave the armed requests alone. Set and cleared by `JoinFlight`.
    pub join_in_flight: bool,
}

impl Inner {
    /// Closes a settled locked phase when `close_store()` came while it was
    /// out. The dropped store releases its file lock. Called with the mutex
    /// held, right after the phase is set.
    pub fn close_if_requested(&mut self) {
        if matches!(self.phase, Phase::Locked(_) | Phase::NoAccount(_))
            && std::mem::take(&mut self.pending.close)
        {
            self.phase = Phase::Closed;
            self.store_busy = false;
        }
    }

    /// Settles a phase installed fresh from the store file (a failed join, a
    /// reset): a lock request that came meanwhile is met by the fresh locked
    /// state (a strict one still sets the password limit, as in `put_back`),
    /// and a pending close is honoured.
    pub fn settle_fresh_phase(&mut self) {
        self.pending.lock = false;
        if std::mem::take(&mut self.pending.strict)
            && let Phase::Locked(locked) = &mut self.phase
        {
            locked.require_password();
        }
        self.close_if_requested();
    }

    /// Settles the requests a join start kept (`put_back_keeping_lock`) when
    /// the join ends without `finish_join` (cancelled, expired, refused): on
    /// a settled phase no request stays armed. A phase that is held out, or
    /// a closed one a reopen is opening, keeps them for whoever settles it.
    pub fn settle_abandoned_join(&mut self) {
        if matches!(self.phase, Phase::Locked(_) | Phase::NoAccount(_)) {
            self.settle_fresh_phase();
        }
    }

    /// Drops a pending join that ran out and settles what it kept.
    pub fn drop_expired_join(&mut self, now_ms: u64) {
        if self.slots.drop_expired_join(now_ms) && !self.pending.join_in_flight {
            self.settle_abandoned_join();
        }
    }
}

impl Pending {
    /// Takes the lock request that settles a held-out session: `Some(strict)`
    /// when a lock or a close is pending. A close is also a lock (lock, then
    /// close); `close` stays set for `Inner::close_if_requested`.
    pub fn take_lock(&mut self) -> Option<bool> {
        if !(self.lock || self.close) {
            return None;
        }
        self.lock = false;
        Some(std::mem::take(&mut self.strict))
    }
}
