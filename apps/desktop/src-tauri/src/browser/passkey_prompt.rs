//! The passkey prompt of the browser endpoint (M5, ADR-0040): one at a time,
//! like the save prompt, with the same 1 s Allow guard and expiry.
//!
//! The prompt belongs to the **pairing and the session** that asked. It ends
//! when the vault locks (the site is told `cancelled`), when the session
//! closes or the pairing is removed, when the page aborts (`passkeyCancel`,
//! same pairing only), or on expiry. The user's answer is applied in
//! [`Shared::respond_passkey`], so an error (a wrong password, a closed
//! vault) leaves the dialog open and nothing is signed or written.

use std::time::Instant;

use mosaic_core::MasterPassword;
use mosaic_native_messaging::bodies::{
    CreateOutcome, GetOutcome, PasskeyCreateResult, PasskeyGetResult, UserVerification,
};
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use tokio::sync::oneshot;

use super::passkey::PasskeyPlan;
use super::prompts::{expires_millis, too_soon};
use super::{Shared, unix_millis};
use crate::contract::browser::{
    BrowserCloseReason, BrowserPasskeyDecision as Decision, BrowserPasskeyRespondRequest,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::secret::SecretIn;
use crate::state::Backend;

/// What a passkey request ends with, as the site's answer.
pub(crate) enum PasskeyReply {
    /// The answer to a `passkeyCreate`.
    Create(PasskeyCreateResult),
    /// The answer to a `passkeyGet`.
    Get(PasskeyGetResult),
}

impl PasskeyReply {
    /// Whether a credential was made or an assertion signed.
    pub(crate) const fn completed(&self) -> bool {
        matches!(
            self,
            Self::Create(PasskeyCreateResult {
                outcome: CreateOutcome::Created,
                ..
            }) | Self::Get(PasskeyGetResult {
                outcome: GetOutcome::Asserted,
                ..
            })
        )
    }
}

/// The result a waiting `passkeyCreate` or `passkeyGet` receives.
pub(crate) type PasskeyResult = Result<PasskeyReply, WireError>;

/// A passkey request waiting for the user.
pub(crate) struct PendingPasskey {
    pub(super) request_id: String,
    /// The pairing that asked; the prompt ends if it is removed.
    pub(crate) pairing: [u8; 16],
    /// The session that asked; the prompt ends if it closes.
    pub(crate) conn: u64,
    shown_at: Instant,
    plan: PasskeyPlan,
    reply: oneshot::Sender<PasskeyResult>,
}

impl PendingPasskey {
    /// The answer for a request that was not completed.
    pub(super) fn refuse(self, error: WireError) {
        let _ = self.reply.send(Err(error));
    }

    /// The request's ID.
    pub(super) fn request_id(&self) -> &str {
        &self.request_id
    }
}

/// Whether `decision` is one the plan allows.
const fn allowed(plan: &PasskeyPlan, decision: &Decision) -> bool {
    match (plan, decision) {
        (PasskeyPlan::Create(p), Decision::Acknowledge) => p.excluded,
        (PasskeyPlan::Create(p), Decision::Create { .. } | Decision::UseAnotherDevice) => {
            !p.excluded
        }
        (_, Decision::Cancel)
        | (PasskeyPlan::Get(_), Decision::Assert { .. } | Decision::UseAnotherDevice) => true,
        _ => false,
    }
}

/// Whether the decision signs or writes (and so needs the Allow guard).
const fn commits(decision: &Decision) -> bool {
    matches!(decision, Decision::Create { .. } | Decision::Assert { .. })
}

impl Shared {
    /// The label of a pairing.
    ///
    /// # Errors
    /// `notPaired` when the pairing is gone (or the list is unreadable).
    pub(crate) fn pairing_label(&self, pairing: &[u8; 16]) -> Result<String, WireError> {
        super::lock(&self.registry)?
            .get(pairing)
            .map(|p| p.label.clone())
            .ok_or(WireError::NotPaired)
    }

    /// Whether a passkey request is open (or being applied).
    pub(crate) fn passkey_is_open(&self) -> bool {
        self.prompts
            .lock()
            .is_ok_and(|prompts| prompts.passkey.is_some() || prompts.passkey_applying)
    }

    /// Opens the passkey prompt for a planned request.
    ///
    /// # Errors
    /// `locked`, `busy` while another passkey request is open,
    /// `notPaired` when the pairing or the session is gone.
    pub(crate) fn open_passkey(
        &self,
        pairing: &[u8; 16],
        conn: u64,
        plan: PasskeyPlan,
    ) -> Result<oneshot::Receiver<PasskeyResult>, WireError> {
        let now = self.now();
        let backend = self.backend().ok_or(WireError::Internal)?;
        let needs_password = plan.needs_password(backend.uv_recent());
        let mut prompts = super::lock(&self.prompts)?;
        if !self.is_unlocked() {
            // The lock hook sets the flag before it drains the prompts, so a
            // prompt opened here is either refused or drained.
            return Err(WireError::Locked);
        }
        if prompts.passkey.is_some() || prompts.passkey_applying {
            return Err(WireError::Busy);
        }
        // Checked under the prompts lock, and the owner's removal or
        // disconnect sweeps the prompts afterwards: never a prompt left by an
        // owner that is already gone.
        if !self.conn_is_open(conn) || !self.is_paired(pairing) {
            return Err(WireError::NotPaired);
        }
        let request = plan.to_request(
            needs_password,
            unix_millis(),
            expires_millis(self.limits.passkey_timeout),
        );
        let (reply, rx) = oneshot::channel();
        prompts.passkey = Some(PendingPasskey {
            request_id: request.request_id.clone(),
            pairing: *pairing,
            conn,
            shown_at: now,
            plan,
            reply,
        });
        drop(prompts);
        self.emit(AppEvent::BrowserPasskeyRequested { request });
        Ok(rx)
    }

    /// Waits for the user's answer, the timeout, or the prompt closing. All
    /// that is not an answer is `cancelled`.
    pub(crate) async fn await_passkey(
        &self,
        request_id: &str,
        mut rx: oneshot::Receiver<PasskeyResult>,
    ) -> PasskeyResult {
        match tokio::time::timeout(self.limits.passkey_timeout, &mut rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(WireError::Cancelled),
            Err(_) => {
                if self.close_passkey(request_id, BrowserCloseReason::Expired) {
                    Err(WireError::Cancelled)
                } else {
                    // Answered in the same instant.
                    rx.try_recv().unwrap_or(Err(WireError::Cancelled))
                }
            }
        }
    }

    /// Ends the passkey prompt `request_id` without an answer; the waiting
    /// request is told `cancelled`. `false`: it was not open.
    pub(crate) fn close_passkey(&self, request_id: &str, reason: BrowserCloseReason) -> bool {
        self.close_passkeys_matching(reason, |p| p.request_id == request_id)
    }

    /// `passkeyCancel`: the page gave up. Only the pairing that asked can end
    /// its own prompt; anything else is ignored without a trace.
    pub(crate) fn close_passkey_by_request_id(&self, pairing: &[u8; 16], request_id: &str) {
        self.close_passkeys_matching(BrowserCloseReason::AbortedByPage, |p| {
            p.pairing == *pairing && p.request_id == request_id
        });
    }

    /// Ends the passkey prompt if `owned` says it is the caller's: its
    /// session closed or its pairing was removed.
    pub(crate) fn close_passkeys_where(
        &self,
        reason: BrowserCloseReason,
        owned: impl Fn(&PendingPasskey) -> bool,
    ) {
        self.close_passkeys_matching(reason, owned);
    }

    fn close_passkeys_matching(
        &self,
        reason: BrowserCloseReason,
        matching: impl Fn(&PendingPasskey) -> bool,
    ) -> bool {
        let Ok(mut prompts) = self.prompts.lock() else {
            return false;
        };
        if !prompts.passkey.as_ref().is_some_and(matching) {
            return false;
        }
        let pending = prompts.passkey.take();
        drop(prompts);
        let Some(pending) = pending else {
            return false;
        };
        let request_id = pending.request_id.clone();
        pending.refuse(WireError::Cancelled);
        self.emit(AppEvent::BrowserPasskeyClosed { request_id, reason });
        true
    }

    /// Ends the dialog for an answer that signs nothing.
    fn finish_unsigned(&self, pending: PendingPasskey, decision: &Decision) {
        let request_id = pending.request_id.clone();
        let result = match (decision, &pending.plan) {
            (Decision::UseAnotherDevice, PasskeyPlan::Create(_)) => {
                Ok(PasskeyReply::Create(PasskeyCreateResult {
                    outcome: CreateOutcome::UseAnotherDevice,
                    credential: None,
                }))
            }
            (Decision::UseAnotherDevice, PasskeyPlan::Get(_)) => {
                Ok(PasskeyReply::Get(PasskeyGetResult {
                    outcome: GetOutcome::UseAnotherDevice,
                    assertion: None,
                }))
            }
            (Decision::Acknowledge, _) => Err(WireError::PasskeyExcluded),
            _ => Err(WireError::Cancelled),
        };
        let _ = pending.reply.send(result);
        self.emit(AppEvent::BrowserPasskeyClosed {
            request_id,
            reason: BrowserCloseReason::Answered,
        });
    }

    /// `browser_passkey_respond`.
    ///
    /// Order: the request is open and the answer fits it; a create or a
    /// sign-in waits out the Allow guard; the user is verified (unlocked
    /// within five minutes, or the master password, whose failures count
    /// towards the five-failure lock); then the vault is written without the
    /// prompts lock (the entry is taken out first, and put back if the write
    /// fails and the prompts were not dropped meanwhile).
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon`, `invalidInput` (the
    /// answer does not fit the request), `reauthRequired`,
    /// `wrongCredentials`, `vaultNotFound`, `vaultUnavailable`,
    /// `itemNotFound`, `locked`: the request stays open after all of them
    /// except an expired one.
    pub(crate) fn respond_passkey(
        &self,
        backend: &Backend,
        request: &BrowserPasskeyRespondRequest,
    ) -> IpcResult<()> {
        let decision = &request.decision;
        let verification = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            let open = prompts
                .passkey
                .as_ref()
                .filter(|p| p.request_id == request.request_id)
                .ok_or(IpcError::new(ErrorCode::BrowserRequestExpired))?;
            if !allowed(&open.plan, decision) {
                return Err(IpcError::new(ErrorCode::InvalidInput));
            }
            if commits(decision) {
                if too_soon(open.shown_at, self.now()) {
                    return Err(IpcError::new(ErrorCode::BrowserAllowTooSoon));
                }
                open.plan.verification()
            } else {
                let pending = prompts.passkey.take();
                drop(prompts);
                if let Some(pending) = pending {
                    self.finish_unsigned(pending, decision);
                }
                return Ok(());
            }
        };
        let user_verified = confirm_user(backend, verification, request.password.as_ref())?;
        self.apply_passkey(backend, request, user_verified)
    }

    /// Takes the prompt out, applies the answer, and answers the site or
    /// puts the prompt back.
    fn apply_passkey(
        &self,
        backend: &Backend,
        request: &BrowserPasskeyRespondRequest,
        user_verified: bool,
    ) -> IpcResult<()> {
        let pending = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            // The vault may have locked while the password was checked.
            if prompts
                .passkey
                .as_ref()
                .is_none_or(|p| p.request_id != request.request_id)
            {
                return Err(IpcError::new(ErrorCode::BrowserRequestExpired));
            }
            prompts.passkey_applying = true;
            prompts.passkey_dropped_while_applying = None;
            prompts.passkey.take()
        };
        let Some(pending) = pending else {
            return Err(IpcError::INTERNAL);
        };
        let applied = apply(backend, &pending.plan, &request.decision, user_verified);
        let dropped = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            prompts.passkey_applying = false;
            let dropped = prompts.passkey_dropped_while_applying.take();
            if applied.is_err() && dropped.is_none() {
                // Nothing was written or signed: the prompt stays open.
                prompts.passkey = Some(pending);
                return applied.map(|_| ());
            }
            dropped
        };
        match applied {
            Ok(reply) => {
                let _ = pending.reply.send(Ok(reply));
                self.emit(AppEvent::BrowserPasskeyClosed {
                    request_id: request.request_id.clone(),
                    reason: BrowserCloseReason::Answered,
                });
                Ok(())
            }
            Err(error) => {
                // The vault locked (or the service stopped) during the write.
                let reason = dropped.unwrap_or(BrowserCloseReason::Locked);
                pending.refuse(WireError::Cancelled);
                self.emit(AppEvent::BrowserPasskeyClosed {
                    request_id: request.request_id.clone(),
                    reason,
                });
                Err(error)
            }
        }
    }
}

/// Whether the user is verified for this operation: not at all for
/// `discouraged`; otherwise within the window of an unlock or a right
/// password, or by the password in the answer (`reauthRequired` without one,
/// `wrongCredentials` for a wrong one).
fn confirm_user(
    backend: &Backend,
    verification: UserVerification,
    password: Option<&SecretIn>,
) -> IpcResult<bool> {
    if verification == UserVerification::Discouraged {
        return Ok(false);
    }
    if !backend.uv_recent() {
        let password = password
            .filter(|p| !p.expose_secret().is_empty())
            .ok_or(IpcError::new(ErrorCode::ReauthRequired))?;
        backend.reauthenticate_master(&MasterPassword::new(password.expose_secret().to_owned()))?;
    }
    Ok(true)
}

/// Writes or signs what the answer chose.
fn apply(
    backend: &Backend,
    plan: &PasskeyPlan,
    decision: &Decision,
    user_verified: bool,
) -> IpcResult<PasskeyReply> {
    match (plan, decision) {
        (PasskeyPlan::Create(plan), Decision::Create { vault_id }) => backend
            .browser_apply_passkey_create(plan, vault_id.get(), user_verified)
            .map(PasskeyReply::Create),
        (PasskeyPlan::Get(plan), Decision::Assert { vault_id, item_id }) => backend
            .browser_apply_passkey_get(plan, vault_id.get(), item_id.get(), user_verified)
            .map(PasskeyReply::Get),
        _ => Err(IpcError::new(ErrorCode::InvalidInput)),
    }
}
