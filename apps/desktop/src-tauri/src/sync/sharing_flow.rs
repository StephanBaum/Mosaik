//! Sharing and emergency access in the desktop app (M6b, CRYPTO_SPEC §8,
//! `docs/DESKTOP_IPC.md` "Sharing"): the commands run the sync engine's user
//! operations (`SyncEngine::lookup_contact`, `invite_member`, ...) over a
//! transport of their own, next to the sync thread (the token is the same).
//!
//! - The session is reached one closure at a time through the sync service's
//!   [`BackendSession`]; it is never held across a network call.
//! - Every command records the lock epoch first and drops its result if the
//!   app locked meanwhile.
//! - A fresh login (granting emergency access, approving a request) goes
//!   through the same path as removing a device: the master password is
//!   checked and counted, the sync thread restarts afterwards.
//! - Only safety codes, roles, addresses and IDs reach the webview. The
//!   questions that matter (removing a member, leaving a vault) are asked in
//!   native dialogs with texts Rust owns ([`super::strings`]).

use mosaic_core::model::{ContactState, Role};
use mosaic_core::{AccountId, Fingerprint, VaultId};
use mosaic_sync::engine::{EngineConfig, SessionAccess as _, SyncEngine, SyncError};
use mosaic_sync::http::{HttpTransport, StoredToken};
use mosaic_sync::sharing::{GrantScope, GrantState, InviteState, MemberOrigin};
use mosaic_sync::transport::TransportKind;
use mosaic_vault::session::{Error as SessionError, SharingFinding, UnlockedSession};

use super::access::BackendSession;
use super::errors::sharing_error;
use super::hosted_flow::HostedCtx;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::ids::WireVaultId;
use crate::contract::sharing::{
    ContactCodeRequest, ContactList, ContactStatus, ContactView, EmergencyClaimRequest,
    EmergencyClaimed, EmergencyDecision, EmergencyGrantRequest, EmergencyOverview,
    EmergencyRequestRequest, EmergencyRequested, EmergencyRespondRequest, EmergencyRevokeRequest,
    EmergencyScopeView, EmergencyState, GrantedView, IncomingInviteView, InviteAcceptRequest,
    InviteDeclineRequest, InviteList, InvitePreviewView, InviteRequest, InviteSent, LookupRequest,
    LookupResult, MAX_GRANT_VAULTS, MemberOriginKind, MemberRequest, MemberRow, MemberRowState,
    OutgoingInviteView, ReceivedView, RemoveOutcome, SetRoleRequest, ShareRole, ShareVaultRequest,
    SharingFindingView, SharingIdentity, VaultMembersView, WAIT_DAYS,
};
use crate::contract::vaults::VaultRequest;
use crate::state::Backend;

/// The longest safety code the commands accept (26 characters and groups).
const MAX_CODE_LEN: usize = 64;

fn ms(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn role_view(role: Role) -> ShareRole {
    match role {
        Role::Owner => ShareRole::Owner,
        Role::Manage => ShareRole::Manage,
        Role::Write => ShareRole::Write,
        // `Removed` never reaches a row that is shown.
        Role::Read | Role::Removed => ShareRole::Read,
    }
}

/// The role to offer: any but owner.
fn role_in(role: ShareRole) -> IpcResult<Role> {
    match role {
        ShareRole::Owner => Err(IpcError::new(ErrorCode::InvalidInput)),
        ShareRole::Manage => Ok(Role::Manage),
        ShareRole::Write => Ok(Role::Write),
        ShareRole::Read => Ok(Role::Read),
    }
}

fn status_view(state: ContactState) -> ContactStatus {
    match state {
        ContactState::Pinned { verified: true } => ContactStatus::Verified,
        ContactState::Pinned { verified: false } => ContactStatus::Pinned,
        ContactState::Changed => ContactStatus::Changed,
        ContactState::Conflicting => ContactStatus::Conflicting,
        // `Unknown` and a state a later version adds.
        _ => ContactStatus::New,
    }
}

fn emergency_state(state: GrantState) -> EmergencyState {
    match state {
        GrantState::Waiting => EmergencyState::Waiting,
        GrantState::Approved => EmergencyState::Approved,
        GrantState::Released => EmergencyState::Released,
        GrantState::Idle => EmergencyState::Idle,
    }
}

fn some_text(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

fn code_text(code: &str) -> IpcResult<&str> {
    if code.len() > MAX_CODE_LEN {
        Err(IpcError::new(ErrorCode::InvalidInput))
    } else {
        Ok(code)
    }
}

/// An address as the server and the contact book keep it.
fn address(email: &str) -> IpcResult<String> {
    mosaic_sync::wire::email::normalize(email.trim())
        .map_err(|_| IpcError::new(ErrorCode::InvalidInput))
}

// ---- what the session shows ---------------------------------------------------

/// Whether a shared vault this device can still reach exists (a backend
/// change is refused then, CRYPTO_SPEC §10.2).
pub(super) fn has_live_shared_vaults(session: &UnlockedSession) -> bool {
    session.vaults().iter().any(|v| {
        session.vault_shared(v.vault_id).unwrap_or(false)
            && !session.vault_access_lost(v.vault_id).unwrap_or(false)
    })
}

/// The address of `account` in a vault: the contact book first, then the
/// roster.
fn email_in(
    session: &UnlockedSession,
    vault: Option<VaultId>,
    account: AccountId,
) -> Option<String> {
    session
        .contacts()
        .ok()
        .and_then(|contacts| {
            contacts
                .into_iter()
                .find(|c| c.account_id == account)
                .map(|c| c.email)
        })
        .or_else(|| {
            let roster = session.vault_roster(vault?).ok().flatten()?;
            roster
                .into_iter()
                .find(|m| m.account_id == account)
                .map(|m| m.email)
        })
        .and_then(some_text)
}

fn vault_name(session: &UnlockedSession, vault: VaultId) -> Option<String> {
    session
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .and_then(|v| v.name.map(str::to_owned))
}

/// The findings of sharing, as the settings and the vault screen show them.
/// A finding about a member that was resolved since (verified, or gone from
/// the roster) is not shown, though the store keeps the row until the user
/// acknowledges.
pub(super) fn findings_view(session: &UnlockedSession) -> Vec<SharingFindingView> {
    let contacts = session.contacts().unwrap_or_default();
    let verified = |a: &AccountId| {
        contacts
            .iter()
            .any(|c| c.account_id == *a && c.verified && !c.conflicting)
    };
    let mut out = Vec::new();
    for finding in session.sharing_findings().unwrap_or_default() {
        match finding {
            SharingFinding::IdentityMismatch => out.push(SharingFindingView::IdentityMismatch),
            SharingFinding::RosterInvalid { vault } => {
                out.push(SharingFindingView::RosterInvalid {
                    vault_name: vault_name(session, vault),
                });
            }
            SharingFinding::ShareUntrusted { vault, sender, .. } => {
                out.push(SharingFindingView::ShareUntrusted {
                    vault_name: vault_name(session, vault),
                    sender_email: email_in(session, Some(vault), sender),
                });
            }
            SharingFinding::MemberUnverified { vault, account } if !verified(&account) => {
                out.push(SharingFindingView::MemberUnverified {
                    vault_id: vault.into(),
                    vault_name: vault_name(session, vault),
                    account_id: account.into(),
                    email: email_in(session, Some(vault), account),
                });
            }
            SharingFinding::MemberDropped { vault, account } => {
                let listed = session.vault_roster(vault).ok().flatten().is_some_and(|r| {
                    r.iter()
                        .any(|m| m.account_id == account && m.role != Role::Removed)
                });
                if listed {
                    out.push(SharingFindingView::MemberDropped {
                        vault_id: vault.into(),
                        vault_name: vault_name(session, vault),
                        account_id: account.into(),
                        email: email_in(session, Some(vault), account),
                    });
                }
            }
            // A member who was verified since.
            SharingFinding::MemberUnverified { .. } => {}
        }
    }
    for summary in session.vaults() {
        if session.vault_access_lost(summary.vault_id).unwrap_or(false) {
            out.push(SharingFindingView::AccessLost {
                vault_id: summary.vault_id.into(),
                vault_name: summary.name.map(str::to_owned),
            });
        }
    }
    out
}

// ---- the commands ---------------------------------------------------------------

impl Backend {
    /// The hosted choice, for a sharing command.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted` (no sync, or a folder).
    fn sharing_ctx(&self) -> IpcResult<HostedCtx> {
        match self.hosted_ctx() {
            Err(e) if e.code == ErrorCode::SyncNotConfigured => {
                Err(IpcError::new(ErrorCode::SharingNotHosted))
            }
            other => other,
        }
    }

    /// Runs `op` on an engine over a transport of its own (the stored
    /// session token), with the session of the current sync generation. The
    /// result is dropped if the app locked meanwhile.
    fn with_sharing<R>(
        &self,
        not_found: ErrorCode,
        op: impl FnOnce(&mut SyncEngine<HttpTransport>, &mut BackendSession) -> Result<R, SyncError>,
    ) -> IpcResult<R> {
        let ctx = self.sharing_ctx()?;
        let epoch = self.sync.epoch();
        let token = self.stored_token(ctx.account)?;
        let result = self.run_engine(&ctx.url, &token, ctx.install, not_found, op);
        self.sync.check_epoch(epoch)?;
        result
    }

    fn run_engine<R>(
        &self,
        url: &str,
        token: &StoredToken,
        install: mosaic_core::InstallId,
        not_found: ErrorCode,
        op: impl FnOnce(&mut SyncEngine<HttpTransport>, &mut BackendSession) -> Result<R, SyncError>,
    ) -> IpcResult<R> {
        let transport = HttpTransport::connect(url, token, install)?;
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
        let mut access = self.sync.session()?;
        op(&mut engine, &mut access).map_err(|e| sharing_error(e, not_found))
    }

    /// The address a verified or pinned contact was looked up by.
    fn contact_email(&self, account: AccountId) -> IpcResult<String> {
        self.with_unlocked(|s| {
            Ok(s.contacts()?
                .into_iter()
                .find(|c| c.account_id == account)
                .map(|c| c.email))
        })?
        .ok_or(IpcError::new(ErrorCode::SharingContactUnverified))
    }

    /// A change of sharing state is a reason to sync soon (roster uploads,
    /// the new vault).
    fn sharing_done(&self) {
        self.sync_poke();
    }

    /// `sharing_identity`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`.
    pub fn sharing_identity(&self) -> IpcResult<SharingIdentity> {
        self.sharing_ctx()?;
        self.with_unlocked(|s| {
            Ok(SharingIdentity {
                safety_code: s.own_fingerprint()?.map(|f| f.safety_code()),
                published: s.identity()?.is_some_and(|(_, _, published)| published),
            })
        })
    }

    /// `sharing_lookup`: the key the server serves for an address, pinned
    /// (unverified) when it is new. Nothing is sealed to it until the user
    /// compared the safety code.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput`, `syncRateLimited`, and
    /// the transport's `sync*` codes.
    pub fn sharing_lookup(&self, request: &LookupRequest) -> IpcResult<LookupResult> {
        let email = address(&request.email)?;
        let found = self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.lookup_contact(access, &email)
        })?;
        Ok(LookupResult {
            account_id: found.account_id.into(),
            email: found.email,
            safety_code: found.safety_code,
            status: status_view(found.state),
        })
    }

    /// `sharing_verify_contact`: the user compared the code with the person.
    /// It must be the code of the key pinned now.
    ///
    /// # Errors
    /// `locked`, `sharingContactUnverified` (no such contact),
    /// `sharingKeyChanged` (the code is not the pinned key's).
    pub fn sharing_verify_contact(&self, request: &ContactCodeRequest) -> IpcResult<()> {
        let account = request.account_id.get();
        let typed = code_text(&request.safety_code)?;
        self.with_unlocked(|s| verify_pinned(s, account, typed).map_err(IpcError::from))
    }

    /// `sharing_accept_key_change`: the server serves another key for a
    /// contact. The new key is looked up again, its code must be the one the
    /// user compared, and it replaces the pin (unverified: the next step is
    /// `sharing_verify_contact` with the same code).
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput`, `sharingKeyChanged`.
    pub fn sharing_accept_key_change(&self, request: &ContactCodeRequest) -> IpcResult<()> {
        let account = request.account_id.get();
        let typed = code_text(&request.safety_code)?;
        let email = self.contact_email(account)?;
        let served = self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.lookup_contact(access, &email)
        })?;
        let fingerprint = Fingerprint::of(&served.account_id, &served.ipk);
        if served.account_id != account || !fingerprint.matches_code(typed) {
            return Err(IpcError::new(ErrorCode::SharingKeyChanged));
        }
        self.with_unlocked(|s| Ok(s.accept_key_change(&account, &served.ipk, &email)?))
    }

    /// `sharing_contacts`.
    ///
    /// # Errors
    /// `locked`.
    pub fn sharing_contacts(&self) -> IpcResult<ContactList> {
        self.with_unlocked(|s| {
            Ok(ContactList {
                contacts: s
                    .contacts()?
                    .into_iter()
                    .map(|c| ContactView {
                        account_id: c.account_id.into(),
                        email: c.email,
                        safety_code: c.fingerprint.safety_code(),
                        verified: c.verified && !c.conflicting,
                        changed: c.conflicting,
                    })
                    .collect(),
            })
        })
    }

    /// `vault_share`: invites a verified contact.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput` (role owner),
    /// `vaultUnavailable` (a hardware vault), `sharingContactUnverified`,
    /// `sharingKeyChanged`, `sharingRoleForbidden`, `sharingInviteExists`,
    /// `sharingAlreadyMember`, `sharingReinviteBlocked`, `sharingInviteLimit`.
    pub fn vault_share(&self, request: &ShareVaultRequest) -> IpcResult<InviteSent> {
        let (vault, account) = (request.vault_id.get(), request.account_id.get());
        let role = role_in(request.role)?;
        self.sharing_ctx()?;
        let email = self.contact_email(account)?;
        self.with_unlocked(|s| {
            if s.synced_vaults().contains(&vault) {
                Ok(())
            } else {
                Err(IpcError::new(ErrorCode::VaultUnavailable))
            }
        })?;
        let sent = self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.invite_member(access, vault, &account, &email, role)
        })?;
        self.sharing_done();
        Ok(InviteSent {
            invite_id: sent.invite_id.into(),
            expires_at: ms(sent.expires_at),
        })
    }

    /// `vault_members`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `vaultNotFound`, `sharingRoleForbidden`
    /// (the server refuses this account the vault).
    pub fn vault_members(&self, request: &VaultRequest) -> IpcResult<VaultMembersView> {
        let vault = request.vault_id.get();
        let view = self.with_sharing(ErrorCode::VaultNotFound, |engine, access| {
            engine.vault_members(access, vault)
        })?;
        Ok(VaultMembersView {
            own_role: role_view(view.own_role),
            members: view
                .members
                .into_iter()
                .map(|m| MemberRow {
                    account_id: m.account_id.into(),
                    email: some_text(m.email),
                    role: role_view(m.role),
                    verified: m.verified,
                    state: match m.state {
                        mosaic_sync::engine::MemberState::Invited => MemberRowState::Invited,
                        mosaic_sync::engine::MemberState::Unconfirmed => {
                            MemberRowState::Unconfirmed
                        }
                        mosaic_sync::engine::MemberState::Dropped => MemberRowState::Dropped,
                        mosaic_sync::engine::MemberState::Removing => MemberRowState::Removing,
                        mosaic_sync::engine::MemberState::Member => MemberRowState::Member,
                    },
                    origin: m.origin.map(|o| match o {
                        MemberOrigin::Owner => MemberOriginKind::Owner,
                        MemberOrigin::Emergency => MemberOriginKind::Emergency,
                        MemberOrigin::Invite => MemberOriginKind::Invite,
                    }),
                })
                .collect(),
            pending_rotation: view.pending_rotation,
            unprotected_attachments: self.sync.unprotected(vault),
        })
    }

    /// `vault_member_set_role`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput` (role owner),
    /// `sharingRoleForbidden`.
    pub fn vault_member_set_role(&self, request: &SetRoleRequest) -> IpcResult<()> {
        let (vault, account) = (request.vault_id.get(), request.account_id.get());
        let role = role_in(request.role)?;
        self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.set_member_role(access, vault, &account, role)
        })?;
        self.sharing_done();
        Ok(())
    }

    /// `vault_member_remove`: asks in a **native** dialog first (the sentence
    /// of CRYPTO_SPEC §8.3: what the person keeps, and that the vault key
    /// changes), then removes. The owner's device changes the vault key at
    /// once; a manager's removal waits for the owner's next sync.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput` (not a member),
    /// `cancelled` (the user said no), `sharingRoleForbidden`.
    pub fn vault_member_remove(&self, request: &MemberRequest) -> IpcResult<RemoveOutcome> {
        let (vault, account) = (request.vault_id.get(), request.account_id.get());
        let view = self.with_sharing(ErrorCode::VaultNotFound, |engine, access| {
            engine.vault_members(access, vault)
        })?;
        let row = view
            .members
            .iter()
            .find(|m| m.account_id == account)
            .ok_or(IpcError::new(ErrorCode::InvalidInput))?;
        let shown = if row.email.is_empty() {
            self.with_unlocked(|s| Ok(email_in(s, Some(vault), account)))?
                .unwrap_or_default()
        } else {
            row.email.clone()
        };
        let texts = super::strings::share(self.settings_snapshot().language);
        let (title, message) = texts.remove(&shown, view.own_role == Role::Owner);
        // The slot is not held while the dialog is open.
        if !self.dialogs.confirm(&title, &message)? {
            return Err(IpcError::new(ErrorCode::Cancelled));
        }
        let unreachable = self.with_sharing(ErrorCode::InvalidInput, |engine, access| {
            engine.remove_member(access, vault, &account)
        })?;
        let count = u32::try_from(unreachable.len()).unwrap_or(u32::MAX);
        if view.own_role == Role::Owner {
            self.sync.set_unprotected(vault, count);
        }
        self.sharing_done();
        Ok(RemoveOutcome {
            unreachable_attachments: count,
        })
    }

    /// `vault_invite_cancel`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `sharingInviteExpired` (not pending).
    pub fn vault_invite_cancel(&self, request: &InviteRequest) -> IpcResult<()> {
        let invite = request.invite_id.get();
        self.with_sharing(ErrorCode::SharingInviteExpired, |engine, access| {
            engine.cancel_invite(access, &invite)
        })?;
        self.sharing_done();
        Ok(())
    }

    /// `vault_leave`: asks in a native dialog, then leaves.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `cancelled`, `sharingOwnerCannotLeave`.
    pub fn vault_leave(&self, request: &VaultRequest) -> IpcResult<()> {
        let vault = request.vault_id.get();
        self.sharing_ctx()?;
        let texts = super::strings::share(self.settings_snapshot().language);
        if !self.dialogs.confirm(texts.leave_title, texts.leave_body)? {
            return Err(IpcError::new(ErrorCode::Cancelled));
        }
        self.with_sharing(ErrorCode::VaultNotFound, |engine, access| {
            engine.leave_vault(access, vault)
        })?;
        self.sharing_done();
        Ok(())
    }

    /// `invites_list`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, and the transport's `sync*` codes.
    pub fn invites_list(&self) -> IpcResult<InviteList> {
        let (incoming, outgoing) = self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.list_invites(access)
        })?;
        self.with_unlocked(|s| {
            Ok(InviteList {
                incoming: incoming
                    .into_iter()
                    .map(|l| IncomingInviteView {
                        invite_id: l.invite.invite_id.into(),
                        sender_email: l.invite.sender_email,
                        role: role_view(l.invite.role),
                        expires_at: ms(l.expires_at),
                    })
                    .collect(),
                outgoing: outgoing
                    .into_iter()
                    .filter(|o| o.state == InviteState::Pending)
                    .map(|o| OutgoingInviteView {
                        invite_id: o.invite_id.into(),
                        vault_id: o.vault_id.into(),
                        vault_name: vault_name(s, o.vault_id),
                        recipient_email: email_in(s, None, o.recipient),
                        role: role_view(o.role),
                        expires_at: ms(o.expires_at),
                    })
                    .collect(),
            })
        })
    }

    /// `invite_preview`: the invite after every check of CRYPTO_SPEC §8.2.
    /// The key waits in the session for five minutes; nothing is stored
    /// until `invite_accept`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `sharingInviteExpired`,
    /// `sharingInviteUnreadable`, `sharingInviteNotInRoster`,
    /// `sharingKeyChanged`, `sharingVaultKnown`.
    pub fn invite_preview(&self, request: &InviteRequest) -> IpcResult<InvitePreviewView> {
        let invite = request.invite_id.get();
        let preview = self.with_sharing(ErrorCode::SharingInviteExpired, |engine, access| {
            engine.preview_invite(access, &invite)
        })?;
        let owner_email = if preview.owner == preview.sender {
            preview.sender_email.clone()
        } else {
            self.with_unlocked(|s| Ok(email_in(s, None, preview.owner)))?
                .unwrap_or_default()
        };
        Ok(InvitePreviewView {
            vault_name: preview.vault_name,
            role: role_view(preview.role),
            sender_account_id: preview.sender.into(),
            sender_email: preview.sender_email,
            safety_code: preview.sender_code,
            sender_status: status_view(preview.sender_state),
            owner_account_id: preview.owner.into(),
            owner_email,
            owner_code: preview.owner_code,
            owner_status: status_view(preview.owner_state),
        })
    }

    /// `invite_accept`: the sender's code, when the user just compared it
    /// (it must be the code of the pinned key), then the accept. A vault
    /// whose owner is not the sender needs the owner verified first.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `sharingContactUnverified`,
    /// `sharingKeyChanged`, `sharingInviteStale` (open the invite again),
    /// `sharingInviteExpired`, `sharingInviteNotInRoster`.
    pub fn invite_accept(&self, request: &InviteAcceptRequest) -> IpcResult<()> {
        let invite = request.invite_id.get();
        let typed = request
            .safety_code
            .as_deref()
            .map(code_text)
            .transpose()?
            .map(str::to_owned);
        let vault = self.with_sharing(ErrorCode::SharingInviteExpired, |engine, access| {
            if let Some(code) = &typed {
                let (incoming, _) = engine.list_invites(access)?;
                let sender = incoming
                    .iter()
                    .find(|l| l.invite.invite_id == invite)
                    .map(|l| l.invite.sender)
                    .ok_or(SyncError::Transport(
                        mosaic_sync::transport::TransportError::NotFound,
                    ))?;
                access.with(|s| verify_pinned(s, sender, code))??;
            }
            engine.accept_invite(access, &invite)
        })?;
        self.sharing_done();
        self.emit(&AppEvent::SyncDataChanged {
            vault_ids: vec![WireVaultId::from(vault)],
        });
        Ok(())
    }

    /// `invite_decline`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `sharingInviteExpired`.
    pub fn invite_decline(&self, request: &InviteDeclineRequest) -> IpcResult<()> {
        let invite = request.invite_id.get();
        self.with_sharing(ErrorCode::SharingInviteExpired, |engine, access| {
            engine.decline_invite(access, &invite, request.block)
        })
    }

    /// `emergency_overview`.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, and the transport's `sync*` codes.
    pub fn emergency_overview(&self) -> IpcResult<EmergencyOverview> {
        let (given, received) = self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.emergency_overview(access)
        })?;
        Ok(EmergencyOverview {
            granted: given
                .into_iter()
                .map(|g| GrantedView {
                    grantee_account_id: g.grantee.into(),
                    email: g.email,
                    wait_days: g.wait_days,
                    scope: match g.scope {
                        GrantScope::Selected => EmergencyScopeView::Vaults {
                            vault_ids: g
                                .vaults
                                .iter()
                                .map(|(v, _)| WireVaultId::from(*v))
                                .collect(),
                        },
                        GrantScope::All => EmergencyScopeView::All,
                    },
                    state: emergency_state(g.state),
                    requested_at: g.requested_at.map(ms),
                    release_at: g.release_at.map(ms),
                })
                .collect(),
            received: received
                .into_iter()
                .map(|r| ReceivedView {
                    grantor_account_id: r.grantor.into(),
                    email: r.email,
                    wait_days: r.wait_days,
                    state: emergency_state(r.state),
                    requested_at: r.requested_at.map(ms),
                    release_at: r.release_at.map(ms),
                    cooldown_until: r.cooldown_until.map(ms),
                })
                .collect(),
        })
    }

    /// `emergency_grant`: the grantee must be a verified contact; needs the
    /// master password (a fresh login). Vault keys only (CRYPTO_SPEC §8.4).
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `invalidInput` (wait or scope),
    /// `wrongCredentials`, `syncTwoFactorRequired`, `sharingContactUnverified`,
    /// `sharingKeyChanged`, `sharingRoleForbidden`, `emergencyWaiting`.
    pub fn emergency_grant(&self, request: &EmergencyGrantRequest) -> IpcResult<()> {
        if !WAIT_DAYS.contains(&request.wait_days) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let (scope, vaults) = match &request.scope {
            EmergencyScopeView::All => (GrantScope::All, Vec::new()),
            EmergencyScopeView::Vaults { vault_ids } => {
                if vault_ids.is_empty() || vault_ids.len() > MAX_GRANT_VAULTS {
                    return Err(IpcError::new(ErrorCode::InvalidInput));
                }
                (
                    GrantScope::Selected,
                    vault_ids.iter().map(|v| v.get()).collect(),
                )
            }
        };
        let grantee = request.account_id.get();
        let ctx = self.sharing_ctx()?;
        let email = self.contact_email(grantee)?;
        self.with_fresh_hosted_ipc(&request.password, request.code.as_ref(), false, |fresh| {
            self.run_engine(
                &fresh.url,
                &fresh.token,
                ctx.install,
                ErrorCode::Internal,
                |engine, access| {
                    engine.grant_emergency(
                        access,
                        &grantee,
                        &email,
                        scope,
                        request.wait_days,
                        &vaults,
                    )
                },
            )
        })?;
        self.sharing_done();
        Ok(())
    }

    /// `emergency_revoke`: deletes a grant in any state; a grant that was
    /// released also changes the key of the vaults it opened.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, and the transport's `sync*` codes.
    pub fn emergency_revoke(&self, request: &EmergencyRevokeRequest) -> IpcResult<()> {
        let grantee = request.account_id.get();
        self.with_sharing(ErrorCode::Internal, |engine, access| {
            engine.revoke_emergency(access, &grantee)
        })?;
        self.sharing_done();
        Ok(())
    }

    /// `emergency_request`: asks a grantor for access; the server keeps the
    /// wait.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `emergencyCooldown`,
    /// `emergencyNotGranted`.
    pub fn emergency_request(
        &self,
        request: &EmergencyRequestRequest,
    ) -> IpcResult<EmergencyRequested> {
        let grantor = request.grantor_account_id.get();
        let release = self.with_sharing(ErrorCode::EmergencyNotGranted, |engine, access| {
            engine.request_emergency(access, &grantor)
        })?;
        Ok(EmergencyRequested {
            release_at: ms(release),
        })
    }

    /// `emergency_respond`: refuse, or approve at once (a fresh login with
    /// the master password).
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `emptyPassword` (approving without the
    /// password), `wrongCredentials`, `syncTwoFactorRequired`,
    /// `emergencyNotGranted`.
    pub fn emergency_respond(&self, request: &EmergencyRespondRequest) -> IpcResult<()> {
        let grantee = request.grantee_account_id.get();
        match request.decision {
            EmergencyDecision::Decline => {
                self.with_sharing(ErrorCode::EmergencyNotGranted, |engine, access| {
                    engine.respond_emergency(access, &grantee, false)
                })?;
            }
            EmergencyDecision::Approve => {
                let ctx = self.sharing_ctx()?;
                let password = request
                    .password
                    .as_ref()
                    .ok_or(IpcError::new(ErrorCode::EmptyPassword))?;
                self.with_fresh_hosted_ipc(password, request.code.as_ref(), false, |fresh| {
                    self.run_engine(
                        &fresh.url,
                        &fresh.token,
                        ctx.install,
                        ErrorCode::EmergencyNotGranted,
                        |engine, access| engine.respond_emergency(access, &grantee, true),
                    )
                })?;
            }
        }
        self.sharing_done();
        Ok(())
    }

    /// `emergency_claim`: once the wait is over the vaults arrive read-only.
    ///
    /// # Errors
    /// `locked`, `sharingNotHosted`, `emergencyWaiting`,
    /// `emergencyNotGranted`, `sharingContactUnverified` (verify the grantor
    /// first), `sharingKeyChanged`.
    pub fn emergency_claim(&self, request: &EmergencyClaimRequest) -> IpcResult<EmergencyClaimed> {
        let grantor = request.grantor_account_id.get();
        let claimed = self.with_sharing(ErrorCode::EmergencyNotGranted, |engine, access| {
            engine.claim_emergency(access, &grantor)
        })?;
        self.sharing_done();
        self.emit(&AppEvent::SyncDataChanged {
            vault_ids: Vec::new(),
        });
        Ok(EmergencyClaimed {
            vaults: u32::try_from(claimed).unwrap_or(u32::MAX),
        })
    }
}

/// Verifies the pinned contact if `typed` is the code of its pinned key.
fn verify_pinned(
    session: &mut UnlockedSession,
    account: AccountId,
    typed: &str,
) -> Result<(), SessionError> {
    let contact = session
        .contacts()?
        .into_iter()
        .find(|c| c.account_id == account)
        .ok_or(SessionError::ContactUnverified)?;
    if contact.conflicting || !contact.fingerprint.matches_code(typed) {
        return Err(SessionError::ContactKeyChanged);
    }
    session.verify_contact(&account, &contact.fingerprint)
}
