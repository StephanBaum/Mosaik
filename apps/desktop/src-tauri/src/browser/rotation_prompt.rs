//! The rotation dialog of the browser endpoint (M7b, ADR-0068): the single prompt behind
//! `rotationBegin`, `rotationResult` and `rotationAddOrigin`. Same rules as the save prompt
//! (`prompts`): one at a time (`busy`), an Allow within a second of it appearing is refused
//! (`browserAllowTooSoon`, the dialog stays), an unanswered request ends with the prompt
//! timeout, a lock, or the browser hanging up. The vault is written only by the user's answer,
//! never by the extension's request; the generated password stays in Rust (the dialog fetches
//! it with `browser_rotation_reveal`, and it is dropped when the prompt ends in any way).

use mosaic_core::model::{RotationAttemptId, RotationStage, RulesSource};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_native_messaging::bodies::{
    AddOriginOutcome, BeginOutcome, ResultOutcome, RotationAddOriginResult, RotationBeginResult,
    RotationResultResult,
};
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use serde::Serialize;
use tokio::sync::oneshot;

use super::prompts::{expires_millis, new_request_id, refusal_for, too_soon};
use super::rotation::{regenerate, valid_decision};
use super::{Shared, unix_millis};
use crate::contract::browser::{
    BrowserCloseReason, BrowserRotationAttempt, BrowserRotationDecision, BrowserRotationKind,
    BrowserRotationRequest, BrowserRotationRespondRequest, BrowserRotationRevealRequest,
    BrowserRotationRevealed,
};
use crate::contract::credential_rotation::{AttemptStage, RotationOutcomeKey};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::items::RevealedSecret;
use crate::contract::secret::SecretOut;
use crate::state::Backend;

/// How long the dialog shows a revealed password.
const REVEAL_SECONDS: u32 = 15;

/// What a rotation dialog decides.
pub(crate) enum RotationPlanKind {
    /// Start a change: the password the desktop generated for the page.
    Begin {
        generated: SecretString,
        /// The page's `passwordrules`, kept for "generate another".
        rules: Option<String>,
        rules_source: Option<RulesSource>,
    },
    /// Close an attempt as the user reported.
    Result {
        attempt: RotationAttemptId,
        reported: RotationOutcomeKey,
        /// Which attempt this closes, for the dialog (M7b review L4).
        info: AttemptInfo,
    },
    /// Add the page's origin to the login.
    AddOrigin { attempt: RotationAttemptId },
}

/// What the result dialog says about the attempt it closes: when it started, how far it got and
/// how many other attempts of the login are open. No value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct AttemptInfo {
    pub(crate) started_at_ms: i64,
    pub(crate) stage: RotationStage,
    pub(crate) other_open: u32,
}

/// What a rotation request shows the user and what its answer writes, computed before it opens.
pub(crate) struct RotationPlan {
    pub(crate) kind: RotationPlanKind,
    pub(crate) vault: VaultId,
    pub(crate) item: ItemId,
    /// The page's canonical origin (re-checked when the answer is applied).
    pub(crate) origin: String,
    pub(crate) title: Option<String>,
    pub(crate) host: String,
    pub(crate) unicode_host: Option<String>,
}

/// What a rotation request ends with, as the extension's response body.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum RotationAnswer {
    Begin(RotationBeginResult),
    Result(RotationResultResult),
    AddOrigin(RotationAddOriginResult),
}

pub(crate) type RotationReply = Result<RotationAnswer, WireError>;

impl RotationPlan {
    const fn kind_key(&self) -> BrowserRotationKind {
        match self.kind {
            RotationPlanKind::Begin { .. } => BrowserRotationKind::Begin,
            RotationPlanKind::Result { .. } => BrowserRotationKind::Result,
            RotationPlanKind::AddOrigin { .. } => BrowserRotationKind::AddOrigin,
        }
    }

    /// The answer of a request nobody decided.
    pub(crate) const fn cancelled(&self) -> RotationAnswer {
        match self.kind {
            RotationPlanKind::Begin { .. } => RotationAnswer::Begin(RotationBeginResult {
                outcome: BeginOutcome::Cancelled,
                attempt_id: None,
            }),
            RotationPlanKind::Result { .. } => RotationAnswer::Result(RotationResultResult {
                outcome: ResultOutcome::Cancelled,
            }),
            RotationPlanKind::AddOrigin { .. } => {
                RotationAnswer::AddOrigin(RotationAddOriginResult {
                    outcome: AddOriginOutcome::Cancelled,
                })
            }
        }
    }
}

/// A rotation request waiting for the user.
pub(crate) struct PendingRotation {
    request_id: String,
    /// The pairing that asked; the prompt ends if it is removed.
    pub(crate) pairing: [u8; 16],
    /// The session that asked; the prompt ends if it closes.
    pub(crate) conn: u64,
    /// When the dialog last showed something new (Allow is refused for a second after).
    shown_at: std::time::Instant,
    pub(crate) plan: RotationPlan,
    reply: oneshot::Sender<RotationReply>,
}

impl PendingRotation {
    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Ends the request with an error (the vault locked or the service stopped).
    pub(crate) fn refuse(self, error: WireError) {
        let _ = self.reply.send(Err(error));
    }
}

/// An opened request: its handle, the answer channel and what a timeout answers.
pub(crate) struct RotationTicket {
    request_id: String,
    rx: oneshot::Receiver<RotationReply>,
    cancelled: RotationAnswer,
}

impl Shared {
    /// Opens the rotation dialog.
    ///
    /// # Errors
    /// `busy` while another rotation request is open, `locked`, `notPaired` when the pairing or
    /// the session is gone.
    pub(crate) fn open_rotation(
        &self,
        pairing: &[u8; 16],
        conn: u64,
        plan: RotationPlan,
    ) -> Result<RotationTicket, WireError> {
        let now = self.now();
        let mut prompts = super::lock(&self.prompts)?;
        if !self.is_unlocked() {
            return Err(WireError::Locked);
        }
        if prompts.rotation.is_some() || prompts.rotation_applying {
            return Err(WireError::Busy);
        }
        if !self.conn_is_open(conn) {
            return Err(WireError::NotPaired);
        }
        let label = super::lock(&self.registry)?
            .get(pairing)
            .map(|p| p.label.clone())
            .ok_or(WireError::NotPaired)?;
        let request_id = new_request_id()?;
        let (reply, rx) = oneshot::channel();
        let (vault_id, item_id) = (plan.vault.into(), plan.item.into());
        let request = BrowserRotationRequest {
            request_id: request_id.clone(),
            pairing_label: label,
            kind: plan.kind_key(),
            vault_id,
            item_id,
            title: plan.title.clone(),
            host: plan.host.clone(),
            unicode_host: plan.unicode_host.clone(),
            outcome: match &plan.kind {
                RotationPlanKind::Result { reported, .. } => Some(*reported),
                _ => None,
            },
            attempt: match &plan.kind {
                RotationPlanKind::Result { info, .. } => Some(BrowserRotationAttempt {
                    started_at: info.started_at_ms,
                    stage: match info.stage {
                        RotationStage::Prepared => AttemptStage::Prepared,
                        RotationStage::Delivered => AttemptStage::Delivered,
                    },
                    other_open: info.other_open,
                }),
                _ => None,
            },
            new_origin: match plan.kind {
                RotationPlanKind::AddOrigin { .. } => Some(plan.origin.clone()),
                _ => None,
            },
            requested_at: unix_millis(),
            expires_at: expires_millis(self.limits.save_timeout),
        };
        let cancelled = plan.cancelled();
        prompts.rotation = Some(PendingRotation {
            request_id: request_id.clone(),
            pairing: *pairing,
            conn,
            shown_at: now,
            plan,
            reply,
        });
        drop(prompts);
        self.emit(AppEvent::BrowserRotationRequested { request });
        Ok(RotationTicket {
            request_id,
            rx,
            cancelled,
        })
    }

    /// Waits for the user's answer, the timeout, or the prompt closing.
    pub(crate) async fn await_rotation(&self, ticket: RotationTicket) -> RotationReply {
        let RotationTicket {
            request_id,
            mut rx,
            cancelled,
        } = ticket;
        match tokio::time::timeout(self.limits.save_timeout, &mut rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => Err(WireError::Internal),
            Err(_) => {
                if self.close_rotation(&request_id, BrowserCloseReason::Expired) {
                    Ok(cancelled)
                } else {
                    // Answered in the same instant.
                    rx.try_recv().unwrap_or(Ok(cancelled))
                }
            }
        }
    }

    /// Whether a rotation request is open (or being written).
    pub(crate) fn rotation_is_open(&self) -> bool {
        self.prompts
            .lock()
            .is_ok_and(|prompts| prompts.rotation.is_some() || prompts.rotation_applying)
    }

    /// Ends the rotation prompt `request_id` without an answer (the generated password is
    /// dropped). `false`: it was not open.
    pub(crate) fn close_rotation(&self, request_id: &str, reason: BrowserCloseReason) -> bool {
        let Ok(mut prompts) = self.prompts.lock() else {
            return false;
        };
        if prompts
            .rotation
            .as_ref()
            .is_none_or(|p| p.request_id != request_id)
        {
            return false;
        }
        let pending = prompts.rotation.take();
        drop(prompts);
        drop(pending);
        self.emit(AppEvent::BrowserRotationClosed {
            request_id: request_id.to_owned(),
            reason,
        });
        true
    }

    /// Ends the rotation prompt if `owned` says it is the caller's: its session closed or its
    /// pairing was removed. The waiting request ends (its reply channel closes).
    pub(crate) fn close_rotations_where(
        &self,
        reason: BrowserCloseReason,
        owned: impl Fn(&PendingRotation) -> bool,
    ) {
        let Ok(mut prompts) = self.prompts.lock() else {
            return;
        };
        if !prompts.rotation.as_ref().is_some_and(owned) {
            return;
        }
        let pending = prompts.rotation.take();
        drop(prompts);
        if let Some(pending) = pending {
            let request_id = pending.request_id.clone();
            drop(pending);
            self.emit(AppEvent::BrowserRotationClosed { request_id, reason });
        }
    }

    /// `browser_rotation_reveal`: the generated password of the open `begin` request.
    pub(crate) fn reveal_rotation(
        &self,
        request: &BrowserRotationRevealRequest,
    ) -> IpcResult<BrowserRotationRevealed> {
        let prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
        let open = prompts
            .rotation
            .as_ref()
            .filter(|p| p.request_id == request.request_id)
            .ok_or(IpcError::new(ErrorCode::BrowserRequestExpired))?;
        let RotationPlanKind::Begin { generated, .. } = &open.plan.kind else {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        };
        let copy = SecretString::from_str_copy(generated.expose_secret())
            .map_err(|_| IpcError::INTERNAL)?;
        Ok(RevealedSecret {
            value: SecretOut::new(copy),
            hide_after_seconds: REVEAL_SECONDS,
        })
    }

    /// `browser_rotation_respond`.
    ///
    /// The vault is written without the prompts lock (the lock hooks take it while the vault
    /// slot is held): the entry is taken out first, and put back if the write fails and the
    /// prompts were not dropped meanwhile.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon`, `invalidInput`, and the vault's errors:
    /// the request stays open after all of them except an expired one.
    pub(crate) fn respond_rotation(
        &self,
        backend: &Backend,
        request: &BrowserRotationRespondRequest,
    ) -> IpcResult<()> {
        let now = self.now();
        let decision = request.decision;
        let pending = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            let open = prompts
                .rotation
                .as_mut()
                .filter(|p| p.request_id == request.request_id)
                .ok_or(IpcError::new(ErrorCode::BrowserRequestExpired))?;
            if !valid_decision(&open.plan.kind, decision) {
                return Err(IpcError::new(ErrorCode::InvalidInput));
            }
            if matches!(decision, BrowserRotationDecision::Regenerate {}) {
                // A new password, shown from now on: the guard starts again.
                regenerate(&mut open.plan.kind, &open.plan.host).map_err(|_| IpcError::INTERNAL)?;
                open.shown_at = now;
                return Ok(());
            }
            let guarded = !matches!(
                decision,
                BrowserRotationDecision::Cancel {} | BrowserRotationDecision::KeepOld {}
            );
            if guarded && too_soon(open.shown_at, now) {
                return Err(IpcError::new(ErrorCode::BrowserAllowTooSoon));
            }
            prompts.rotation_applying = !matches!(decision, BrowserRotationDecision::Cancel {});
            prompts.rotation_dropped_while_applying = None;
            prompts.rotation.take()
        };
        let Some(pending) = pending else {
            return Err(IpcError::INTERNAL);
        };
        let applied = if matches!(decision, BrowserRotationDecision::Cancel {}) {
            Ok(pending.plan.cancelled())
        } else {
            backend.browser_apply_rotation(&pending.plan, decision)
        };
        let mut orphaned = false;
        let dropped = {
            let mut prompts = super::lock(&self.prompts).map_err(|_| IpcError::INTERNAL)?;
            prompts.rotation_applying = false;
            let dropped = prompts.rotation_dropped_while_applying.take();
            if applied.is_err() && dropped.is_none() {
                if pending.reply.is_closed() {
                    // The request expired (or its session went away) while the answer was
                    // applied: nobody waits for it, so the prompt is not put back.
                    orphaned = true;
                } else {
                    // Nothing was written: the prompt stays open.
                    prompts.rotation = Some(pending);
                    return applied.map(|_| ());
                }
            }
            dropped
        };
        if orphaned {
            self.emit(AppEvent::BrowserRotationClosed {
                request_id: request.request_id.clone(),
                reason: BrowserCloseReason::Expired,
            });
            return applied.map(|_| ());
        }
        match applied {
            Ok(answer) => {
                let _ = pending.reply.send(Ok(answer));
                self.emit(AppEvent::BrowserRotationClosed {
                    request_id: request.request_id.clone(),
                    reason: BrowserCloseReason::Answered,
                });
                Ok(())
            }
            Err(error) => {
                // The vault locked (or the service stopped) during the write.
                let reason = dropped.unwrap_or(BrowserCloseReason::Locked);
                let _ = pending.reply.send(Err(refusal_for(reason)));
                self.emit(AppEvent::BrowserRotationClosed {
                    request_id: request.request_id.clone(),
                    reason,
                });
                Err(error)
            }
        }
    }
}
