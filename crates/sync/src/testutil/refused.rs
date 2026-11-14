//! A sharing backend that refuses every call, the way a hostile hosted
//! server can answer the owner of a vault (403 or 404). An honest server
//! never refuses the owner: the owner cannot be removed or leave.

use mosaic_core::model::Role;
use mosaic_core::{
    AccountId, ContactsCiphertext, IdentityPublicKey, InstallId, KeyGeneration, ShareSeal, VaultId,
    VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use mosaic_vault::session::InviteId;

use crate::sharing::{
    AcceptResult, CasResult, DirectoryEntry, GrantInfo, GrantScope, IdentityPut, ListedInvite,
    MembersView, NewInvite, OutgoingInvite, ReceivedGrant, RotateRequest, RotateResult,
    SharingTransport,
};
use crate::transport::TransportError;

/// See the [module documentation](self).
#[derive(Clone, Copy, Debug)]
pub struct RefusedSharing(pub TransportError);

impl SharingTransport for RefusedSharing {
    fn account_identity(
        &mut self,
    ) -> Result<Option<(IdentityPublicKey, WrappedKey)>, TransportError> {
        Err(self.0)
    }

    fn put_identity(
        &mut self,
        _ipk: &IdentityPublicKey,
        _wrapped: &WrappedKey,
    ) -> Result<IdentityPut, TransportError> {
        Err(self.0)
    }

    fn lookup(&mut self, _email: &str) -> Result<DirectoryEntry, TransportError> {
        Err(self.0)
    }

    fn directory(
        &mut self,
        _account: &AccountId,
    ) -> Result<Option<IdentityPublicKey>, TransportError> {
        Err(self.0)
    }

    fn get_contacts(&mut self) -> Result<(Option<ContactsCiphertext>, u64), TransportError> {
        Err(self.0)
    }

    fn put_contacts(
        &mut self,
        _ciphertext: &ContactsCiphertext,
        _base_seq: u64,
    ) -> Result<CasResult, TransportError> {
        Err(self.0)
    }

    fn create_invite(
        &mut self,
        _vault: VaultId,
        _invite: &NewInvite<'_>,
    ) -> Result<u64, TransportError> {
        Err(self.0)
    }

    fn list_invites(&mut self) -> Result<(Vec<ListedInvite>, Vec<OutgoingInvite>), TransportError> {
        Ok((Vec::new(), Vec::new()))
    }

    fn invite_vault(
        &mut self,
        _invite: &InviteId,
    ) -> Result<(VaultMetaCiphertext, Option<VaultRosterCiphertext>), TransportError> {
        Err(self.0)
    }

    fn accept_invite(
        &mut self,
        _invite: &InviteId,
        _wrapped: &WrappedKey,
        _generation: KeyGeneration,
        _device: InstallId,
    ) -> Result<AcceptResult, TransportError> {
        Err(self.0)
    }

    fn decline_invite(&mut self, _invite: &InviteId, _block: bool) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn cancel_invite(&mut self, _invite: &InviteId) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn members(&mut self, _vault: VaultId) -> Result<MembersView, TransportError> {
        Err(self.0)
    }

    fn set_role(
        &mut self,
        _vault: VaultId,
        _account: &AccountId,
        _role: Role,
    ) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn remove_member(
        &mut self,
        _vault: VaultId,
        _account: &AccountId,
    ) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn put_roster(
        &mut self,
        _vault: VaultId,
        _generation: KeyGeneration,
        _roster: &VaultRosterCiphertext,
        _base_seq: Option<u64>,
    ) -> Result<CasResult, TransportError> {
        Err(self.0)
    }

    fn rotate(
        &mut self,
        _vault: VaultId,
        _request: &RotateRequest<'_>,
    ) -> Result<RotateResult, TransportError> {
        Err(self.0)
    }

    fn claim_key(
        &mut self,
        _vault: VaultId,
        _generation: KeyGeneration,
        _wrapped: &WrappedKey,
        _device: InstallId,
    ) -> Result<u64, TransportError> {
        Err(self.0)
    }

    fn leave(&mut self, _vault: VaultId) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn block(&mut self, _account: &AccountId) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn unblock(&mut self, _account: &AccountId) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn emergency(&mut self) -> Result<(Vec<GrantInfo>, Vec<ReceivedGrant>), TransportError> {
        Ok((Vec::new(), Vec::new()))
    }

    fn put_grant(
        &mut self,
        _grantee: &AccountId,
        _wait_days: u8,
        _scope: GrantScope,
        _seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn put_grant_seals(
        &mut self,
        _grantee: &AccountId,
        _seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn delete_grant(&mut self, _grantee: &AccountId) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn emergency_request(&mut self, _grantor: &AccountId) -> Result<u64, TransportError> {
        Err(self.0)
    }

    fn emergency_decide(
        &mut self,
        _grantee: &AccountId,
        _approve: bool,
    ) -> Result<(), TransportError> {
        Err(self.0)
    }

    fn emergency_release(
        &mut self,
        _grantor: &AccountId,
    ) -> Result<Vec<(VaultId, KeyGeneration, ShareSeal)>, TransportError> {
        Err(self.0)
    }
}
