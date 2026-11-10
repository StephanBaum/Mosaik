//! [`SharingTransport`] for the hosted backend (M6b spec §6.4): the
//! sharing and emergency-access routes of `docs/SERVER.md` over the same
//! HTTP layer as [`HttpTransport`]. Responses are parsed strictly; a role,
//! state or ID that is out of range is a protocol error.

use mosaic_core::model::Role;
use mosaic_core::{
    AccountId, ContactsCiphertext, IdentityPublicKey, InstallId, KeyGeneration, ShareSeal, VaultId,
    VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use mosaic_vault::session::{IncomingInvite, InviteId};
use reqwest::Method;

use super::backend::HttpTransport;
use super::client::{REQUEST_TIMEOUT, json_body};
use crate::sharing::{
    AcceptResult, CasResult, DirectoryEntry, GrantInfo, GrantScope, GrantState, IdentityPut,
    InviteState, ListedInvite, MemberInfo, MemberOrigin, MembersView, NewInvite, OutgoingInvite,
    PendingInvite, ReceivedGrant, RotateRequest, RotateResult, SharingTransport,
};
use crate::transport::TransportError;
use crate::wire::json::sharing::{
    AcceptRequest, AcceptResponse, AccountIdWire, AccountWithIdentity, ClaimRequest,
    ConflictDetail, ContactsResponse, DeclineRequest, DirectoryEntry as DirectoryBody,
    EmergencyResponse, GrantRequest, IdentityBody, IdentityExistsDetail, InviteRequest,
    InviteSealEntry, InviteSent, InviteVaultResponse, InvitesResponse, LookupRequest,
    MembersMismatchDetail, MembersResponse, PutContactsRequest, PutRosterRequest,
    RecipientSealEntry, ReleaseAtResponse, ReleaseResponse, RoleRequest, RotateBody, SealsRequest,
    SeqResponse, VaultSealEntry, WireId,
};
use crate::wire::json::{self, B64, B64Fixed, Empty, MAX_RESPONSE_LEN, codes};

fn role_of(n: u8) -> Result<Role, TransportError> {
    Role::from_number(n).map_err(|_| TransportError::Protocol)
}

fn role_number(role: Role) -> Result<u8, TransportError> {
    role.number().ok_or(TransportError::Protocol)
}

fn invite_of(id: WireId) -> Result<InviteId, TransportError> {
    InviteId::from_bytes(id.0).map_err(|_| TransportError::Protocol)
}

fn wire_invite(id: &InviteId) -> WireId {
    WireId(*id.as_bytes())
}

fn generation_of(g: u32) -> Result<KeyGeneration, TransportError> {
    KeyGeneration::new(g).map_err(|_| TransportError::Protocol)
}

fn ipk_of(bytes: &[u8; 32]) -> IdentityPublicKey {
    IdentityPublicKey(*bytes)
}

fn seal_of(bytes: &[u8]) -> Result<ShareSeal, TransportError> {
    ShareSeal::from_bytes(bytes).map_err(|_| TransportError::Protocol)
}

fn state_of(n: u8) -> Result<GrantState, TransportError> {
    Ok(match n {
        0 => GrantState::Idle,
        1 => GrantState::Waiting,
        2 => GrantState::Approved,
        3 => GrantState::Released,
        _ => return Err(TransportError::Protocol),
    })
}

fn seal_entries(seals: &[(VaultId, KeyGeneration, ShareSeal)]) -> Vec<VaultSealEntry> {
    seals
        .iter()
        .map(|(vault, g, seal)| VaultSealEntry {
            vault_id: *vault,
            generation: g.get(),
            seal: B64Fixed(*seal.as_bytes()),
        })
        .collect()
}

impl HttpTransport {
    /// A call that answers 204 or 200 with `{}`, or a bodyless success.
    fn call_done<Req: serde::Serialize>(
        &self,
        method: Method,
        path: &str,
        request: Option<&Req>,
    ) -> Result<(), TransportError> {
        let reply = self.http().send(
            method,
            path,
            Some(self.bearer()),
            json_body(request)?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        match reply.status {
            200 | 201 | 204 => Ok(()),
            _ => Err(reply.failure()),
        }
    }

    /// A compare-and-swap upload: `200 {seq}` or `409 conflict`.
    fn call_cas<Req: serde::Serialize>(
        &self,
        path: &str,
        request: &Req,
    ) -> Result<CasResult, TransportError> {
        let reply = self.http().send(
            Method::PUT,
            path,
            Some(self.bearer()),
            json_body(Some(request))?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        if reply.status == 200 {
            let (_, ok): (u16, SeqResponse) = reply.json(&[200])?;
            return Ok(CasResult::Stored { seq: ok.seq });
        }
        let body = reply.error_body();
        if body.as_ref().map(|b| b.error.as_str()) == Some(codes::CONFLICT) {
            return body
                .and_then(|b| b.detail_as::<ConflictDetail>())
                .map(|d| CasResult::Conflict {
                    current_seq: d.current_seq,
                })
                .ok_or(TransportError::Protocol);
        }
        Err(reply.failure())
    }
}

impl SharingTransport for HttpTransport {
    fn account_identity(
        &mut self,
    ) -> Result<Option<(IdentityPublicKey, WrappedKey)>, TransportError> {
        let a: AccountWithIdentity = self.get("/v1/account?identity=1")?;
        a.identity
            .map(|i| {
                let wrapped = WrappedKey::from_bytes(&i.wrapped_isk.0)
                    .map_err(|_| TransportError::Protocol)?;
                Ok((ipk_of(&i.ipk.0), wrapped))
            })
            .transpose()
    }

    fn put_identity(
        &mut self,
        ipk: &IdentityPublicKey,
        wrapped: &WrappedKey,
    ) -> Result<IdentityPut, TransportError> {
        let body = IdentityBody {
            ipk: B64Fixed(ipk.0),
            wrapped_isk: B64Fixed(*wrapped.as_bytes()),
        };
        let reply = self.http().send(
            Method::PUT,
            "/v1/account/identity",
            Some(self.bearer()),
            json_body(Some(&body))?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        if matches!(reply.status, 200 | 204) {
            return Ok(IdentityPut::Stored);
        }
        let err = reply.error_body();
        if err.as_ref().map(|b| b.error.as_str()) == Some(codes::IDENTITY_EXISTS) {
            let detail = err
                .and_then(|b| b.detail_as::<IdentityExistsDetail>())
                .ok_or(TransportError::Protocol)?;
            return Ok(IdentityPut::Exists {
                ipk: ipk_of(&detail.ipk.0),
                wrapped: WrappedKey::from_bytes(&detail.wrapped_isk.0)
                    .map_err(|_| TransportError::Protocol)?,
            });
        }
        Err(reply.failure())
    }

    fn lookup(&mut self, email: &str) -> Result<DirectoryEntry, TransportError> {
        let (_, r): (u16, DirectoryBody) = self.call(
            Method::POST,
            "/v1/directory/lookup",
            Some(&LookupRequest {
                email: email.to_owned(),
            }),
            &[200],
        )?;
        Ok(DirectoryEntry {
            account_id: r.account_id,
            ipk: ipk_of(&r.ipk.0),
        })
    }

    fn directory(
        &mut self,
        account: &AccountId,
    ) -> Result<Option<IdentityPublicKey>, TransportError> {
        match self.get::<DirectoryBody>(&format!("/v1/directory/{account}")) {
            Ok(r) if r.account_id == *account => Ok(Some(ipk_of(&r.ipk.0))),
            Ok(_) => Err(TransportError::Protocol),
            Err(TransportError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn get_contacts(&mut self) -> Result<(Option<ContactsCiphertext>, u64), TransportError> {
        let r: ContactsResponse = self.get("/v1/account/contacts")?;
        let ct = r
            .ciphertext
            .map(|c| ContactsCiphertext::from_bytes(&c.0).map_err(|_| TransportError::Protocol))
            .transpose()?;
        Ok((ct, r.seq))
    }

    fn put_contacts(
        &mut self,
        ciphertext: &ContactsCiphertext,
        base_seq: u64,
    ) -> Result<CasResult, TransportError> {
        self.call_cas(
            "/v1/account/contacts",
            &PutContactsRequest {
                ciphertext: B64(ciphertext.as_bytes().to_vec()),
                base_seq,
            },
        )
    }

    fn create_invite(
        &mut self,
        vault: VaultId,
        invite: &NewInvite<'_>,
    ) -> Result<u64, TransportError> {
        let (_, sent): (u16, InviteSent) = self.call(
            Method::POST,
            &format!("/v1/vaults/{vault}/invites"),
            Some(&InviteRequest {
                invite_id: wire_invite(&invite.invite_id),
                recipient: invite.recipient,
                role: role_number(invite.role)?,
                generation: invite.generation.get(),
                seal: B64Fixed(*invite.seal.as_bytes()),
            }),
            &[201],
        )?;
        Ok(sent.expires_at)
    }

    fn list_invites(&mut self) -> Result<(Vec<ListedInvite>, Vec<OutgoingInvite>), TransportError> {
        let r: InvitesResponse = self.get("/v1/invites")?;
        let incoming = r
            .incoming
            .into_iter()
            .map(|i| {
                Ok(ListedInvite {
                    invite: IncomingInvite {
                        invite_id: invite_of(i.invite_id)?,
                        vault_id: i.vault_id,
                        sender: i.sender,
                        sender_email: i.sender_email,
                        role: role_of(i.role)?,
                        generation: generation_of(i.generation)?,
                        seal: seal_of(&i.seal.0)?,
                    },
                    expires_at: i.expires_at,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        let outgoing = r
            .outgoing
            .into_iter()
            .map(|o| {
                Ok(OutgoingInvite {
                    invite_id: invite_of(o.invite_id)?,
                    vault_id: o.vault_id,
                    recipient: o.recipient,
                    role: role_of(o.role)?,
                    state: match o.state {
                        0 => InviteState::Pending,
                        1 => InviteState::Accepted,
                        2 => InviteState::Declined,
                        4 => InviteState::Expired,
                        // Cancelled by the recipient's re-key (M7): the
                        // seal addressed to the old identity key is dead,
                        // so the sender's client treats it as cancelled
                        // (review H3).
                        3 | 5 => InviteState::Cancelled,
                        _ => return Err(TransportError::Protocol),
                    },
                    delivered: o.delivered,
                    expires_at: o.expires_at,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        Ok((incoming, outgoing))
    }

    fn invite_vault(
        &mut self,
        invite: &InviteId,
    ) -> Result<(VaultMetaCiphertext, Option<VaultRosterCiphertext>), TransportError> {
        let id = wire_invite(invite);
        let r: InviteVaultResponse = self.get(&format!("/v1/invites/{id}/vault"))?;
        let meta =
            VaultMetaCiphertext::from_bytes(&r.meta.0).map_err(|_| TransportError::Protocol)?;
        let roster = r
            .roster
            .map(|c| VaultRosterCiphertext::from_bytes(&c.0).map_err(|_| TransportError::Protocol))
            .transpose()?;
        Ok((meta, roster))
    }

    fn accept_invite(
        &mut self,
        invite: &InviteId,
        wrapped: &WrappedKey,
        generation: KeyGeneration,
        device: InstallId,
    ) -> Result<AcceptResult, TransportError> {
        let id = wire_invite(invite);
        let (_, r): (u16, AcceptResponse) = self.call(
            Method::POST,
            &format!("/v1/invites/{id}/accept"),
            Some(&AcceptRequest {
                wrapped: wrapped.clone(),
                device,
                generation: generation.get(),
            }),
            &[200],
        )?;
        Ok(AcceptResult {
            vault_id: r.vault_id,
            generation: generation_of(r.generation)?,
            role: role_of(r.role)?,
        })
    }

    fn decline_invite(&mut self, invite: &InviteId, block: bool) -> Result<(), TransportError> {
        let id = wire_invite(invite);
        self.call_done(
            Method::POST,
            &format!("/v1/invites/{id}/decline"),
            Some(&DeclineRequest { block }),
        )
    }

    fn cancel_invite(&mut self, invite: &InviteId) -> Result<(), TransportError> {
        let id = wire_invite(invite);
        self.call_done::<Empty>(Method::DELETE, &format!("/v1/invites/{id}"), None)
    }

    fn members(&mut self, vault: VaultId) -> Result<MembersView, TransportError> {
        let r: MembersResponse = self.get(&format!("/v1/vaults/{vault}/members"))?;
        let members = r
            .members
            .into_iter()
            .map(|m| {
                Ok(MemberInfo {
                    account_id: m.account_id,
                    email: m.email,
                    role: role_of(m.role)?,
                    origin: match m.origin {
                        0 => MemberOrigin::Owner,
                        1 => MemberOrigin::Invite,
                        2 => MemberOrigin::Emergency,
                        _ => return Err(TransportError::Protocol),
                    },
                    added_at: m.added_at,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        let invites = r
            .invites
            .into_iter()
            .map(|i| {
                Ok(PendingInvite {
                    invite_id: invite_of(i.invite_id)?,
                    recipient: i.recipient,
                    role: role_of(i.role)?,
                    delivered: i.delivered,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        Ok(MembersView { members, invites })
    }

    fn set_role(
        &mut self,
        vault: VaultId,
        account: &AccountId,
        role: Role,
    ) -> Result<(), TransportError> {
        self.call_done(
            Method::PUT,
            &format!("/v1/vaults/{vault}/members/{account}"),
            Some(&RoleRequest {
                role: role_number(role)?,
            }),
        )
    }

    fn remove_member(&mut self, vault: VaultId, account: &AccountId) -> Result<(), TransportError> {
        self.call_done::<Empty>(
            Method::DELETE,
            &format!("/v1/vaults/{vault}/members/{account}"),
            None,
        )
    }

    fn put_roster(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        roster: &VaultRosterCiphertext,
        base_seq: Option<u64>,
    ) -> Result<CasResult, TransportError> {
        self.call_cas(
            &format!("/v1/vaults/{vault}/roster"),
            &PutRosterRequest {
                generation: generation.get(),
                ciphertext: B64(roster.as_bytes().to_vec()),
                base_seq,
            },
        )
    }

    fn rotate(
        &mut self,
        vault: VaultId,
        request: &RotateRequest<'_>,
    ) -> Result<RotateResult, TransportError> {
        let body = RotateBody {
            generation: request.generation.get(),
            wrapped: request.wrapped.clone(),
            device: request.device,
            expected_head: request.expected_head,
            roster: B64(request.roster.as_bytes().to_vec()),
            seals: request
                .seals
                .iter()
                .map(|(a, s)| RecipientSealEntry {
                    recipient: *a,
                    seal: B64Fixed(*s.as_bytes()),
                })
                .collect(),
            invite_seals: request
                .invite_seals
                .iter()
                .map(|(i, s)| InviteSealEntry {
                    invite_id: wire_invite(i),
                    seal: B64Fixed(*s.as_bytes()),
                })
                .collect(),
            revoke: request.revoke.iter().map(|a| AccountIdWire(*a)).collect(),
        };
        let reply = self.http().send(
            Method::POST,
            &format!("/v1/vaults/{vault}/rotate"),
            Some(self.bearer()),
            json_body(Some(&body))?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?;
        if reply.status == 200 {
            let (_, ok): (u16, SeqResponse) = reply.json(&[200])?;
            return Ok(RotateResult::Accepted { seq: ok.seq });
        }
        let err = reply.error_body();
        match err.as_ref().map(|b| b.error.as_str()) {
            Some(codes::GENERATION_TAKEN) => err
                .and_then(|b| b.detail_as::<json::GenerationDetail>())
                .and_then(|d| KeyGeneration::new(d.current_generation).ok())
                .map(|current| RotateResult::GenerationTaken { current })
                .ok_or(TransportError::Protocol),
            Some(codes::HEAD_MOVED) => err
                .and_then(|b| b.detail_as::<json::HeadMovedDetail>())
                .map(|d| RotateResult::HeadMoved { head: d.head_seq })
                .ok_or(TransportError::Protocol),
            Some(codes::MEMBERS_MISMATCH) => {
                let d = err
                    .and_then(|b| b.detail_as::<MembersMismatchDetail>())
                    .ok_or(TransportError::Protocol)?;
                Ok(RotateResult::MembersMismatch {
                    members: d.members.into_iter().map(|a| a.0).collect(),
                    invites: d
                        .invites
                        .into_iter()
                        .map(|i| invite_of(i.0))
                        .collect::<Result<_, _>>()?,
                })
            }
            _ => Err(reply.failure()),
        }
    }

    fn claim_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
    ) -> Result<u64, TransportError> {
        let (_, ok): (u16, SeqResponse) = self.call(
            Method::PUT,
            &format!("/v1/vaults/{vault}/keys/{}/claim", generation.get()),
            Some(&ClaimRequest {
                wrapped: wrapped.clone(),
                device,
            }),
            &[200],
        )?;
        Ok(ok.seq)
    }

    fn leave(&mut self, vault: VaultId) -> Result<(), TransportError> {
        self.call_done::<Empty>(Method::POST, &format!("/v1/vaults/{vault}/leave"), None)
    }

    fn block(&mut self, account: &AccountId) -> Result<(), TransportError> {
        self.call_done::<Empty>(Method::POST, &format!("/v1/blocks/{account}"), None)
    }

    fn unblock(&mut self, account: &AccountId) -> Result<(), TransportError> {
        self.call_done::<Empty>(Method::DELETE, &format!("/v1/blocks/{account}"), None)
    }

    fn emergency(&mut self) -> Result<(Vec<GrantInfo>, Vec<ReceivedGrant>), TransportError> {
        let r: EmergencyResponse = self.get("/v1/emergency")?;
        let granted = r
            .granted
            .into_iter()
            .map(|g| {
                Ok(GrantInfo {
                    grantee: g.grantee,
                    email: g.email,
                    wait_days: g.wait_days,
                    scope: match g.scope {
                        1 => GrantScope::All,
                        2 => GrantScope::Selected,
                        _ => return Err(TransportError::Protocol),
                    },
                    vaults: g
                        .vaults
                        .into_iter()
                        .map(|v| (v.vault_id, v.generation))
                        .collect(),
                    state: state_of(g.state)?,
                    requested_at: g.requested_at,
                    release_at: g.release_at,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        let received = r
            .received
            .into_iter()
            .map(|g| {
                Ok(ReceivedGrant {
                    grantor: g.grantor,
                    email: g.email,
                    wait_days: g.wait_days,
                    state: state_of(g.state)?,
                    requested_at: g.requested_at,
                    release_at: g.release_at,
                    cooldown_until: g.cooldown_until,
                })
            })
            .collect::<Result<Vec<_>, TransportError>>()?;
        Ok((granted, received))
    }

    fn put_grant(
        &mut self,
        grantee: &AccountId,
        wait_days: u8,
        scope: GrantScope,
        seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError> {
        self.call_done(
            Method::PUT,
            &format!("/v1/emergency/grants/{grantee}"),
            Some(&GrantRequest {
                wait_days,
                scope: match scope {
                    GrantScope::All => 1,
                    GrantScope::Selected => 2,
                },
                seals: seal_entries(seals),
            }),
        )
    }

    fn put_grant_seals(
        &mut self,
        grantee: &AccountId,
        seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError> {
        self.call_done(
            Method::PUT,
            &format!("/v1/emergency/grants/{grantee}/seals"),
            Some(&SealsRequest {
                seals: seal_entries(seals),
            }),
        )
    }

    fn delete_grant(&mut self, grantee: &AccountId) -> Result<(), TransportError> {
        self.call_done::<Empty>(
            Method::DELETE,
            &format!("/v1/emergency/grants/{grantee}"),
            None,
        )
    }

    fn emergency_request(&mut self, grantor: &AccountId) -> Result<u64, TransportError> {
        let (_, r): (u16, ReleaseAtResponse) = self.call(
            Method::POST,
            &format!("/v1/emergency/grants/{grantor}/request"),
            None::<&Empty>,
            &[200],
        )?;
        Ok(r.release_at)
    }

    fn emergency_decide(
        &mut self,
        grantee: &AccountId,
        approve: bool,
    ) -> Result<(), TransportError> {
        let action = if approve { "approve" } else { "decline" };
        self.call_done::<Empty>(
            Method::POST,
            &format!("/v1/emergency/grants/{grantee}/{action}"),
            None,
        )
    }

    fn emergency_release(
        &mut self,
        grantor: &AccountId,
    ) -> Result<Vec<(VaultId, KeyGeneration, ShareSeal)>, TransportError> {
        let (_, r): (u16, ReleaseResponse) = self.call(
            Method::POST,
            &format!("/v1/emergency/grants/{grantor}/release"),
            None::<&Empty>,
            &[200],
        )?;
        r.seals
            .into_iter()
            .map(|s| {
                Ok((
                    s.vault_id,
                    generation_of(s.generation)?,
                    seal_of(&s.seal.0)?,
                ))
            })
            .collect()
    }
}
