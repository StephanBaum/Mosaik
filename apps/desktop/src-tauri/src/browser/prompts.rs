//! The prompts of the browser endpoint: pairing, saving a login and (in
//! `passkey_prompt`) passkeys. One of each at a time, following the SSH approval pattern (ADR-0025):
//! a request is shown in the webview, an Allow within a second of it
//! appearing is refused (`browserAllowTooSoon`, the request stays open), and
//! an unanswered request ends with its timeout, a lock, or the browser
//! hanging up. The captured password stays here, in a `SecretString`, and is
//! dropped when the prompt ends in any way.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mosaic_core::browser_channel::PairingKey;
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_native_messaging::bodies::{ProposeSaveResult, SaveOutcome};
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use tokio::sync::oneshot;

use super::passkey_prompt::PendingPasskey;
use super::registry::{RegistryError, label_is_valid};
use super::{Bucket, Shared, hex, unix_millis};
use crate::contract::browser::{
    BrowserCloseReason, BrowserFamily, BrowserHint, BrowserPairingRequest,
    BrowserPairingRespondRequest, BrowserSaveKind, BrowserSaveRequest, BrowserSaveRespondRequest,
    PairingDecision,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::state::Backend;

/// The earliest an Allow is taken after the prompt was shown (ADR-0025,
/// ADR-0037). A deny is always taken.
pub const MIN_ALLOW_DELAY: Duration = Duration::from_secs(1);

/// Denials within this long count towards a block.
const DENIAL_WINDOW: Duration = Duration::from_mins(10);
/// Denials within the window that block pairing requests.
const DENIALS_TO_BLOCK: usize = 3;
/// How long pairing requests stay blocked.
const BLOCK_FOR: Duration = Duration::from_mins(10);

/// What a pairing request ends with: the new pairing's ID and key, or why
/// the browser is refused.
pub type PairOutcome = Result<([u8; 16], PairingKey), WireError>;

/// What a save request ends with.
pub type SaveReply = Result<ProposeSaveResult, WireError>;

struct PendingPair {
    request_id: String,
    browser: BrowserFamily,
    hint: BrowserHint,
    shown_at: Instant,
    reply: oneshot::Sender<PairOutcome>,
}

/// A save request waiting for the user.
pub(crate) struct PendingSave {
    request_id: String,
    /// The pairing that asked; the prompt ends if it is removed.
    pub(crate) pairing: [u8; 16],
    /// The session that asked; the prompt ends if it closes.
    pub(crate) conn: u64,
    shown_at: Instant,
    /// The page's origin, which becomes the saved URL.
    pub(crate) origin: String,
    /// The captured user name.
    pub(crate) username: Option<String>,
    /// The captured password. Never leaves Rust.
    pub(crate) password: SecretString,
    /// The logins an update may change.
    pub(crate) candidates: Vec<(VaultId, ItemId)>,
    /// The page is plain HTTP and not loopback.
    pub(crate) insecure: bool,
    reply: oneshot::Sender<SaveReply>,
}

/// What a save request shows the user, computed before it is opened.
pub(crate) struct SavePlan {
    pub(crate) kind: BrowserSaveKind,
    pub(crate) origin: String,
    pub(crate) host: String,
    pub(crate) suggested_title: String,
    pub(crate) insecure: bool,
    pub(crate) candidates: Vec<crate::contract::browser::BrowserSaveCandidate>,
}

/// The open prompts and the pairing denials.
#[derive(Default)]
pub struct Prompts {
    pair: Option<PendingPair>,
    save: Option<PendingSave>,
    /// The open passkey request (`passkey_prompt`).
    pub(super) passkey: Option<PendingPasskey>,
    /// A passkey answer is being applied: its entry is out of `passkey` and
    /// no other passkey request may open.
    pub(super) passkey_applying: bool,
    /// Why the prompts were dropped while `passkey_applying`.
    pub(super) passkey_dropped_while_applying: Option<BrowserCloseReason>,
    /// A save is being written to the vault: its entry is out of `save`
    /// (the vault is not written under the prompts lock) and no other save
    /// may open.
    applying: bool,
    /// Why the prompts were dropped while `applying`.
    dropped_while_applying: Option<BrowserCloseReason>,
    denials: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

impl Prompts {
    /// The open pairing request's ID (tests).
    #[cfg(test)]
    pub(crate) fn pair_id(&self) -> Option<&str> {
        self.pair.as_ref().map(|p| p.request_id.as_str())
    }

    /// The open passkey request's ID (tests).
    #[cfg(test)]
    pub(crate) fn passkey_id(&self) -> Option<&str> {
        self.passkey.as_ref().map(PendingPasskey::request_id)
    }

    /// The open save request's ID (tests).
    #[cfg(test)]
    pub(crate) fn save_id(&self) -> Option<&str> {
        self.save.as_ref().map(|p| p.request_id.as_str())
    }

    fn note_denial(&mut self, now: Instant) {
        self.denials.push_back(now);
        while self
            .denials
            .front()
            .is_some_and(|&t| now.saturating_duration_since(t) >= DENIAL_WINDOW)
        {
            self.denials.pop_front();
        }
        if self.denials.len() >= DENIALS_TO_BLOCK {
            self.denials.clear();
            self.blocked_until = Some(now + BLOCK_FOR);
        }
    }

    fn is_blocked(&mut self, now: Instant) -> bool {
        match self.blocked_until {
            Some(until) if now < until => true,
            Some(_) => {
                self.blocked_until = None;
                false
            }
            None => false,
        }
    }
}

fn new_request_id() -> Result<String, WireError> {
    mosaic_core::browser_channel::new_endpoint_token()
        .map(|t| hex(&t))
        .map_err(|_| WireError::Internal)
}

pub(super) fn too_soon(shown_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(shown_at) < MIN_ALLOW_DELAY
}

fn default_label(hint: BrowserHint) -> &'static str {
    match hint {
        BrowserHint::Chrome => "Chrome",
        BrowserHint::Edge => "Edge",
        BrowserHint::Brave => "Brave",
        BrowserHint::Opera => "Opera",
        BrowserHint::Vivaldi => "Vivaldi",
        BrowserHint::Chromium => "Chromium",
        BrowserHint::Firefox => "Firefox",
        BrowserHint::Other => "Browser",
    }
}

pub(super) fn expires_millis(limit: Duration) -> i64 {
    unix_millis().saturating_add(i64::try_from(limit.as_millis()).unwrap_or(i64::MAX))
}

impl Shared {
    /// Opens the pairing prompt for a `pairRequest`.
    ///
    /// # Errors
    /// `pairingBlocked` (three denials lately), `rateLimited`, `locked`,
    /// `pairingLimit`, and `pairingConflict` when another request was open:
    /// both close (the open one is told too).
    pub(crate) fn open_pair(
        &self,
        browser: BrowserFamily,
        hint: BrowserHint,
        code: &str,
    ) -> Result<(String, oneshot::Receiver<PairOutcome>), WireError> {
        let now = self.now();
        let mut prompts = super::lock(&self.prompts)?;
        if prompts.is_blocked(now) {
            return Err(WireError::PairingBlocked);
        }
        if !super::lock(&self.limiter)?.allow(Bucket::PairRequest, None, now) {
            return Err(WireError::RateLimited);
        }
        if !self.is_unlocked() {
            return Err(WireError::Locked);
        }
        {
            let registry = super::lock(&self.registry)?;
            if registry.is_unreadable() {
                // Nothing could be saved: do not ask the user.
                return Err(WireError::Internal);
            }
            if registry.list().len() >= mosaic_native_messaging::protocol::MAX_PAIRINGS {
                return Err(WireError::PairingLimit);
            }
        }
        if let Some(old) = prompts.pair.take() {
            let _ = old.reply.send(Err(WireError::PairingConflict));
            self.emit(AppEvent::BrowserPairingClosed {
                request_id: old.request_id,
                reason: BrowserCloseReason::Conflict,
            });
            return Err(WireError::PairingConflict);
        }
        let request_id = new_request_id()?;
        let (reply, rx) = oneshot::channel();
        prompts.pair = Some(PendingPair {
            request_id: request_id.clone(),
            browser,
            hint,
            shown_at: now,
            reply,
        });
        drop(prompts);
        self.emit(AppEvent::BrowserPairingRequested {
            request: BrowserPairingRequest {
                request_id: request_id.clone(),
                browser,
                hint,
                code: code.to_owned(),
                requested_at: unix_millis(),
                expires_at: expires_millis(self.limits.pair_timeout),
            },
        });
        Ok((request_id, rx))
    }

    /// Waits for the user's answer, the timeout, or the prompt closing.
    pub(crate) async fn await_pair(
        &self,
        request_id: &str,
        mut rx: oneshot::Receiver<PairOutcome>,
    ) -> PairOutcome {
        match tokio::time::timeout(self.limits.pair_timeout, &mut rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(WireError::Internal),
            Err(_) => {
                if self.close_pair(request_id, BrowserCloseReason::Expired) {
                    Err(WireError::PairingExpired)
                } else {
                    // Answered in the same instant.
                    rx.try_recv().unwrap_or(Err(WireError::PairingExpired))
                }
            }
        }
    }

    /// Ends the pairing prompt `request_id` without an answer. `false`: it
    /// was not open.
    pub(crate) fn close_pair(&self, request_id: &str, reason: BrowserCloseReason) -> bool {
        let Ok(mut prompts) = self.prompts.lock() else {
            return false;
        };
        if prompts
            .pair
            .as_ref()
            .is_none_or(|p| p.request_id != request_id)
        {
            return false;
        }
        let Some(pending) = prompts.pair.take() else {
            return false;
        };
        drop(prompts);
        drop(pending.reply);
        self.emit(AppEvent::BrowserPairingClosed {
            request_id: request_id.to_owned(),
            reason,
        });
        true
    }

    /// `browser_pairing_respond`.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon` (the request stays
    /// open), `invalidInput` (a bad label; the request stays open).
    pub(crate) fn respond_pair(&self, request: &BrowserPairingRespondRequest) -> IpcResult<()> {
        let now = self.now();
        let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
        let open = prompts
            .pair
            .as_ref()
            .filter(|p| p.request_id == request.request_id)
            .ok_or(IpcError::new(ErrorCode::BrowserRequestExpired))?;
        if request.decision == PairingDecision::Deny {
            let Some(pending) = prompts.pair.take() else {
                return Err(IpcError::INTERNAL);
            };
            prompts.note_denial(now);
            drop(prompts);
            let _ = pending.reply.send(Err(WireError::PairingDenied));
            self.close_event(&request.request_id);
            return Ok(());
        }
        let label = match request.label.as_deref() {
            Some(label) if label_is_valid(label) => label.to_owned(),
            Some(_) => return Err(IpcError::new(ErrorCode::InvalidInput)),
            None => default_label(open.hint).to_owned(),
        };
        if too_soon(open.shown_at, now) {
            return Err(IpcError::new(ErrorCode::BrowserAllowTooSoon));
        }
        let Some(pending) = prompts.pair.take() else {
            return Err(IpcError::INTERNAL);
        };
        let added = super::lock(&self.registry)
            .map_err(|_| IpcError::INTERNAL)?
            .add(label, pending.browser, unix_millis());
        drop(prompts);
        let outcome = match added {
            Ok(pairing) => Ok(pairing),
            Err(RegistryError::Full) => Err(WireError::PairingLimit),
            Err(RegistryError::Storage | RegistryError::Unreadable) => Err(WireError::Internal),
        };
        if let Err(Ok((id, _key))) = pending.reply.send(outcome) {
            // The browser hung up in the same instant: nobody got the key.
            let _ = super::lock(&self.registry).map(|mut registry| registry.remove(&id));
            self.emit(AppEvent::BrowserPairingClosed {
                request_id: request.request_id.clone(),
                reason: BrowserCloseReason::Disconnected,
            });
            self.pairings_changed();
            return Err(IpcError::new(ErrorCode::BrowserRequestExpired));
        }
        self.close_event(&request.request_id);
        self.pairings_changed();
        Ok(())
    }

    fn close_event(&self, request_id: &str) {
        self.emit(AppEvent::BrowserPairingClosed {
            request_id: request_id.to_owned(),
            reason: BrowserCloseReason::Answered,
        });
    }

    /// Opens the save prompt for a `proposeSave`.
    ///
    /// # Errors
    /// `busy` while another save request is open, `notPaired` when the
    /// pairing or the session is gone.
    pub(crate) fn open_save(
        &self,
        pairing: &[u8; 16],
        conn: u64,
        plan: SavePlan,
        username: Option<String>,
        password: SecretString,
    ) -> Result<(String, oneshot::Receiver<SaveReply>), WireError> {
        let now = self.now();
        let mut prompts = super::lock(&self.prompts)?;
        if !self.is_unlocked() {
            // The lock hook sets the flag before it drains the prompts, so a
            // prompt opened here is either refused or drained.
            return Err(WireError::Locked);
        }
        if prompts.save.is_some() || prompts.applying {
            return Err(WireError::Busy);
        }
        // Both checks happen under the prompts lock, and the owner's removal
        // or disconnect sweeps the prompts afterwards, so a prompt is never
        // left behind by an owner that is already gone.
        if !self.conn_is_open(conn) {
            return Err(WireError::NotPaired);
        }
        let label = super::lock(&self.registry)?
            .get(pairing)
            .map(|p| p.label.clone())
            .ok_or(WireError::NotPaired)?;
        let request_id = new_request_id()?;
        let (reply, rx) = oneshot::channel();
        let request = BrowserSaveRequest {
            request_id: request_id.clone(),
            pairing_label: label,
            origin: plan.origin.clone(),
            host: plan.host,
            username: username.clone(),
            kind: plan.kind,
            candidates: plan.candidates.clone(),
            suggested_title: plan.suggested_title,
            insecure: plan.insecure,
            requested_at: unix_millis(),
            expires_at: expires_millis(self.limits.save_timeout),
        };
        prompts.save = Some(PendingSave {
            request_id: request_id.clone(),
            pairing: *pairing,
            conn,
            shown_at: now,
            origin: plan.origin,
            username,
            password,
            candidates: plan
                .candidates
                .iter()
                .map(|c| (c.vault_id.get(), c.item_id.get()))
                .collect(),
            insecure: plan.insecure,
            reply,
        });
        drop(prompts);
        self.emit(AppEvent::BrowserSaveRequested { request });
        Ok((request_id, rx))
    }

    /// Waits for the user's answer, the timeout, or the prompt closing.
    pub(crate) async fn await_save(
        &self,
        request_id: &str,
        mut rx: oneshot::Receiver<SaveReply>,
    ) -> SaveReply {
        match tokio::time::timeout(self.limits.save_timeout, &mut rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => Err(WireError::Internal),
            Err(_) => {
                if self.close_save(request_id, BrowserCloseReason::Expired) {
                    Ok(cancelled())
                } else {
                    rx.try_recv().unwrap_or_else(|_| Ok(cancelled()))
                }
            }
        }
    }

    /// Ends the save prompt `request_id` without an answer (the password is
    /// dropped). `false`: it was not open.
    pub(crate) fn close_save(&self, request_id: &str, reason: BrowserCloseReason) -> bool {
        let Ok(mut prompts) = self.prompts.lock() else {
            return false;
        };
        if prompts
            .save
            .as_ref()
            .is_none_or(|p| p.request_id != request_id)
        {
            return false;
        }
        let pending = prompts.save.take();
        drop(prompts);
        drop(pending);
        self.emit(AppEvent::BrowserSaveClosed {
            request_id: request_id.to_owned(),
            reason,
        });
        true
    }

    /// Whether a save request is open (or being written).
    pub(crate) fn save_is_open(&self) -> bool {
        self.prompts
            .lock()
            .is_ok_and(|prompts| prompts.save.is_some() || prompts.applying)
    }

    /// Ends the save prompt if `owned` says it is the caller's: its
    /// session closed or its pairing was removed. The password is dropped
    /// and the waiting `proposeSave` ends (its reply channel closes).
    pub(crate) fn close_saves_where(
        &self,
        reason: BrowserCloseReason,
        owned: impl Fn(&PendingSave) -> bool,
    ) {
        let Ok(mut prompts) = self.prompts.lock() else {
            return;
        };
        if !prompts.save.as_ref().is_some_and(owned) {
            return;
        }
        let pending = prompts.save.take();
        drop(prompts);
        if let Some(pending) = pending {
            let request_id = pending.request_id.clone();
            drop(pending);
            self.emit(AppEvent::BrowserSaveClosed { request_id, reason });
        }
    }

    /// `browser_save_respond`.
    ///
    /// The vault is written without the prompts lock (the lock hooks take it
    /// while the vault slot is held): the entry is taken out first, and put
    /// back if the write fails and the prompts were not dropped meanwhile.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon`, `invalidInput`,
    /// `vaultUnavailable`, `vaultNotFound`, `itemNotFound`: the request
    /// stays open after all of them except an expired one.
    pub(crate) fn respond_save(
        &self,
        backend: &Backend,
        request: &BrowserSaveRespondRequest,
    ) -> IpcResult<()> {
        use crate::contract::browser::BrowserSaveDecision as Decision;
        let now = self.now();
        let cancel = matches!(request.decision, Decision::Cancel);
        let pending = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            let open = prompts
                .save
                .as_ref()
                .filter(|p| p.request_id == request.request_id)
                .ok_or(IpcError::new(ErrorCode::BrowserRequestExpired))?;
            if !cancel && too_soon(open.shown_at, now) {
                return Err(IpcError::new(ErrorCode::BrowserAllowTooSoon));
            }
            prompts.applying = !cancel;
            prompts.dropped_while_applying = None;
            prompts.save.take()
        };
        let Some(pending) = pending else {
            return Err(IpcError::INTERNAL);
        };
        let applied = if cancel {
            Ok(cancelled())
        } else {
            backend.browser_apply_save(&pending, &request.decision)
        };
        let dropped = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            prompts.applying = false;
            let dropped = prompts.dropped_while_applying.take();
            if applied.is_err() && dropped.is_none() {
                // Nothing was written: the prompt stays open.
                prompts.save = Some(pending);
                return applied.map(|_| ());
            }
            dropped
        };
        match applied {
            Ok(result) => {
                let _ = pending.reply.send(Ok(result));
                self.emit(AppEvent::BrowserSaveClosed {
                    request_id: request.request_id.clone(),
                    reason: BrowserCloseReason::Answered,
                });
                Ok(())
            }
            Err(error) => {
                // The vault locked (or the service stopped) during the write.
                let reason = dropped.unwrap_or(BrowserCloseReason::Locked);
                let _ = pending.reply.send(Err(refusal_for(reason)));
                self.emit(AppEvent::BrowserSaveClosed {
                    request_id: request.request_id.clone(),
                    reason,
                });
                Err(error)
            }
        }
    }

    /// Closes every prompt: the vault locked or the service stopped. The
    /// held password is dropped, and a waiting passkey request is told
    /// `cancelled`.
    pub(crate) fn drop_prompts(&self, reason: BrowserCloseReason) {
        let Ok(mut prompts) = self.prompts.lock() else {
            return;
        };
        let pair = prompts.pair.take();
        let save = prompts.save.take();
        let passkey = prompts.passkey.take();
        if prompts.applying {
            // A save is being written: its owner finishes it off.
            prompts.dropped_while_applying = Some(reason);
        }
        if prompts.passkey_applying {
            prompts.passkey_dropped_while_applying = Some(reason);
        }
        drop(prompts);
        if let Some(passkey) = passkey {
            let request_id = passkey.request_id().to_owned();
            passkey.refuse(WireError::Cancelled);
            self.emit(AppEvent::BrowserPasskeyClosed { request_id, reason });
        }
        let refusal = refusal_for(reason);
        if let Some(pair) = pair {
            let _ = pair.reply.send(Err(refusal));
            self.emit(AppEvent::BrowserPairingClosed {
                request_id: pair.request_id,
                reason,
            });
        }
        if let Some(save) = save {
            let request_id = save.request_id.clone();
            let _ = save.reply.send(Err(refusal));
            self.emit(AppEvent::BrowserSaveClosed { request_id, reason });
        }
    }
}

/// What a prompt that closed for `reason` tells the browser.
const fn refusal_for(reason: BrowserCloseReason) -> WireError {
    match reason {
        BrowserCloseReason::Locked => WireError::Locked,
        _ => WireError::DesktopNotRunning,
    }
}

/// The reply for a save that was not made.
pub(crate) const fn cancelled() -> ProposeSaveResult {
    ProposeSaveResult {
        outcome: SaveOutcome::Cancelled,
        vault_id: None,
        item_id: None,
    }
}
