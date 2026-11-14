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
use crate::transport::{Rejection, TransportError};

/// What the rotate call of a [`RefusedSharing`] answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotateStub {
    /// Every call is refused with the error (the default).
    Refuse,
    /// The member list is served (owner only, no invites) and the rotation
    /// answers `GenerationTaken`: another install of the account rotated
    /// first. Benign, never a refusal.
    GenerationTaken,
    /// The member list is served and the rotation is rejected with
    /// `role_forbidden`, which an honest server never answers the owner.
    RoleForbidden,
}

/// See the [module documentation](self).
#[derive(Clone, Copy, Debug)]
pub struct RefusedSharing {
    /// The refusal every refused call answers.
    pub error: TransportError,
    /// What the member list and the rotation answer.
    pub rotate: RotateStub,
}

impl RefusedSharing {
    /// Refuses everything with `error`.
    #[must_use]
    pub const fn new(error: TransportError) -> Self {
        Self {
            error,
            rotate: RotateStub::Refuse,
        }
    }

    /// Serves the member list and answers the rotation as `rotate` says.
    #[must_use]
    pub const fn with_rotate(mut self, rotate: RotateStub) -> Self {
        self.rotate = rotate;
        self
    }
}

impl SharingTransport for RefusedSharing {
    fn account_identity(
        &mut self,
    ) -> Result<Option<(IdentityPublicKey, WrappedKey)>, TransportError> {
        Err(self.error)
    }

    fn put_identity(
        &mut self,
        _ipk: &IdentityPublicKey,
        _wrapped: &WrappedKey,
    ) -> Result<IdentityPut, TransportError> {
        Err(self.error)
    }

    fn lookup(&mut self, _email: &str) -> Result<DirectoryEntry, TransportError> {
        Err(self.error)
    }

    fn directory(
        &mut self,
        _account: &AccountId,
    ) -> Result<Option<IdentityPublicKey>, TransportError> {
        Err(self.error)
    }

    fn get_contacts(&mut self) -> Result<(Option<ContactsCiphertext>, u64), TransportError> {
        Err(self.error)
    }

    fn put_contacts(
        &mut self,
        _ciphertext: &ContactsCiphertext,
        _base_seq: u64,
    ) -> Result<CasResult, TransportError> {
        Err(self.error)
    }

    fn create_invite(
        &mut self,
        _vault: VaultId,
        _invite: &NewInvite<'_>,
    ) -> Result<u64, TransportError> {
        Err(self.error)
    }

    fn list_invites(&mut self) -> Result<(Vec<ListedInvite>, Vec<OutgoingInvite>), TransportError> {
        Ok((Vec::new(), Vec::new()))
    }

    fn invite_vault(
        &mut self,
        _invite: &InviteId,
    ) -> Result<(VaultMetaCiphertext, Option<VaultRosterCiphertext>), TransportError> {
        Err(self.error)
    }

    fn accept_invite(
        &mut self,
        _invite: &InviteId,
        _wrapped: &WrappedKey,
        _generation: KeyGeneration,
        _device: InstallId,
    ) -> Result<AcceptResult, TransportError> {
        Err(self.error)
    }

    fn decline_invite(&mut self, _invite: &InviteId, _block: bool) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn cancel_invite(&mut self, _invite: &InviteId) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn members(&mut self, _vault: VaultId) -> Result<MembersView, TransportError> {
        match self.rotate {
            RotateStub::Refuse => Err(self.error),
            RotateStub::GenerationTaken | RotateStub::RoleForbidden => Ok(MembersView::default()),
        }
    }

    fn set_role(
        &mut self,
        _vault: VaultId,
        _account: &AccountId,
        _role: Role,
    ) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn remove_member(
        &mut self,
        _vault: VaultId,
        _account: &AccountId,
    ) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn put_roster(
        &mut self,
        _vault: VaultId,
        _generation: KeyGeneration,
        _roster: &VaultRosterCiphertext,
        _base_seq: Option<u64>,
    ) -> Result<CasResult, TransportError> {
        Err(self.error)
    }

    fn rotate(
        &mut self,
        _vault: VaultId,
        _request: &RotateRequest<'_>,
    ) -> Result<RotateResult, TransportError> {
        match self.rotate {
            RotateStub::Refuse => Err(self.error),
            RotateStub::GenerationTaken => Ok(RotateResult::GenerationTaken {
                current: KeyGeneration::FIRST,
            }),
            RotateStub::RoleForbidden => Err(TransportError::Rejected(Rejection::RoleForbidden)),
        }
    }

    fn claim_key(
        &mut self,
        _vault: VaultId,
        _generation: KeyGeneration,
        _wrapped: &WrappedKey,
        _device: InstallId,
    ) -> Result<u64, TransportError> {
        Err(self.error)
    }

    fn leave(&mut self, _vault: VaultId) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn block(&mut self, _account: &AccountId) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn unblock(&mut self, _account: &AccountId) -> Result<(), TransportError> {
        Err(self.error)
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
        Err(self.error)
    }

    fn put_grant_seals(
        &mut self,
        _grantee: &AccountId,
        _seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn delete_grant(&mut self, _grantee: &AccountId) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn emergency_request(&mut self, _grantor: &AccountId) -> Result<u64, TransportError> {
        Err(self.error)
    }

    fn emergency_decide(
        &mut self,
        _grantee: &AccountId,
        _approve: bool,
    ) -> Result<(), TransportError> {
        Err(self.error)
    }

    fn emergency_release(
        &mut self,
        _grantor: &AccountId,
    ) -> Result<Vec<(VaultId, KeyGeneration, ShareSeal)>, TransportError> {
        Err(self.error)
    }
}
