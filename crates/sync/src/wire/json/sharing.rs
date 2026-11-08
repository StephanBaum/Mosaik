//! The JSON bodies of the sharing and emergency-access routes (M6b spec
//! §5.2). They mirror `crates/server/src/api/wire.rs`: camelCase keys, every
//! key present, unknown keys refused, IDs as lowercase hyphenated UUIDs,
//! bytes as base64url without padding, roles and states as small numbers.

use core::fmt;

use mosaic_core::{AccountId, InstallId, KeyGeneration, VaultId, WrappedKey};
use serde::{Deserialize, Serialize};

use super::{B64, B64Fixed, UuidId, generation, profile, required, uuid_str, wrapped_key};

/// A 16-byte ID that is not an account, vault, install or item ID (an invite
/// ID): a non-nil UUID.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WireId(pub [u8; 16]);

impl fmt::Debug for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WireId(..)")
    }
}

impl fmt::Display for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The same hyphenated text as the core ID types.
        match InstallId::from_bytes(self.0) {
            Ok(id) => fmt::Display::fmt(&id, f),
            Err(_) => f.write_str("00000000-0000-0000-0000-000000000000"),
        }
    }
}

impl UuidId for WireId {
    fn from_raw(bytes: [u8; 16]) -> Option<Self> {
        (bytes != [0u8; 16]).then_some(Self(bytes))
    }
}

/// A sealed key (82 bytes).
pub type SealBytes = B64Fixed<{ mosaic_core::ShareSeal::LEN }>;
/// A vault roster ciphertext in a body.
pub type RosterBytes = B64<65_600>;
/// The contact book ciphertext in a body.
pub type ContactsBytes = B64<262_200>;

/// `PUT /v1/account/identity`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityBody {
    pub ipk: B64Fixed<32>,
    pub wrapped_isk: B64Fixed<74>,
}

/// `GET /v1/account?identity=1`: the account plus its identity pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountWithIdentity {
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub email: String,
    pub salt: B64Fixed<16>,
    #[serde(with = "profile")]
    pub profile: mosaic_core::Argon2Profile,
    #[serde(with = "wrapped_key")]
    pub wrapped_ak: WrappedKey,
    pub account_version: u64,
    pub totp_enabled: bool,
    #[serde(deserialize_with = "required")]
    pub identity: Option<IdentityBody>,
}

/// Detail of `identity_exists`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityExistsDetail {
    pub ipk: B64Fixed<32>,
    pub wrapped_isk: B64Fixed<74>,
}

/// `POST /v1/directory/lookup` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LookupRequest {
    pub email: String,
}

/// `POST /v1/directory/lookup` and `GET /v1/directory/{id}` answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectoryEntry {
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub ipk: B64Fixed<32>,
}

/// `PUT /v1/account/contacts` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutContactsRequest {
    pub ciphertext: ContactsBytes,
    pub base_seq: u64,
}

/// `GET /v1/account/contacts` answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContactsResponse {
    #[serde(deserialize_with = "required")]
    pub ciphertext: Option<ContactsBytes>,
    pub seq: u64,
}

/// An answer that is a sequence number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SeqResponse {
    pub seq: u64,
}

/// Detail of `conflict`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConflictDetail {
    pub current_seq: u64,
}

/// `POST /v1/vaults/{v}/invites` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteRequest {
    #[serde(with = "uuid_str")]
    pub invite_id: WireId,
    #[serde(with = "uuid_str")]
    pub recipient: AccountId,
    pub role: u8,
    pub generation: u32,
    pub seal: SealBytes,
}

/// `POST /v1/vaults/{v}/invites` answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteSent {
    pub expires_at: u64,
}

/// An invite addressed to the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IncomingInviteEntry {
    #[serde(with = "uuid_str")]
    pub invite_id: WireId,
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    #[serde(with = "uuid_str")]
    pub sender: AccountId,
    pub sender_email: String,
    pub role: u8,
    pub generation: u32,
    pub seal: SealBytes,
    pub expires_at: u64,
}

/// An invite the caller sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutgoingInviteEntry {
    #[serde(with = "uuid_str")]
    pub invite_id: WireId,
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    #[serde(with = "uuid_str")]
    pub recipient: AccountId,
    pub role: u8,
    pub state: u8,
    pub delivered: bool,
    pub expires_at: u64,
}

/// `GET /v1/invites`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvitesResponse {
    pub incoming: Vec<IncomingInviteEntry>,
    pub outgoing: Vec<OutgoingInviteEntry>,
}

/// `GET /v1/invites/{id}/vault`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteVaultResponse {
    pub meta: B64<{ super::MAX_META_LEN }>,
    #[serde(deserialize_with = "required")]
    pub roster: Option<RosterBytes>,
}

/// `POST /v1/invites/{id}/accept` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptRequest {
    #[serde(with = "wrapped_key")]
    pub wrapped: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device: InstallId,
    /// The key generation of the wrapped key (M6b review M1): the server
    /// refuses the accept unless it is the invite's and the vault's current
    /// one.
    pub generation: u32,
}

/// `POST /v1/invites/{id}/accept` answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptResponse {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    pub generation: u32,
    pub role: u8,
}

/// Detail of `invite_stale`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteStaleDetail {
    pub current_generation: u32,
}

/// `POST /v1/invites/{id}/decline` request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclineRequest {
    pub block: bool,
}

/// A member in `GET /v1/vaults/{v}/members`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberEntry {
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub email: String,
    pub role: u8,
    pub origin: u8,
    pub added_at: u64,
}

/// A pending invite in `GET /v1/vaults/{v}/members`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberInviteEntry {
    #[serde(with = "uuid_str")]
    pub invite_id: WireId,
    #[serde(with = "uuid_str")]
    pub recipient: AccountId,
    pub role: u8,
    pub delivered: bool,
}

/// `GET /v1/vaults/{v}/members`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MembersResponse {
    pub members: Vec<MemberEntry>,
    pub invites: Vec<MemberInviteEntry>,
}

/// `PUT /v1/vaults/{v}/members/{a}` request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoleRequest {
    pub role: u8,
}

/// `PUT /v1/vaults/{v}/roster` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutRosterRequest {
    pub generation: u32,
    pub ciphertext: RosterBytes,
    #[serde(deserialize_with = "required")]
    pub base_seq: Option<u64>,
}

/// A share for one member in a rotation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipientSealEntry {
    #[serde(with = "uuid_str")]
    pub recipient: AccountId,
    pub seal: SealBytes,
}

/// A refreshed seal for one pending invite in a rotation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteSealEntry {
    #[serde(with = "uuid_str")]
    pub invite_id: WireId,
    pub seal: SealBytes,
}

/// `POST /v1/vaults/{v}/rotate` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotateBody {
    pub generation: u32,
    #[serde(with = "wrapped_key")]
    pub wrapped: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device: InstallId,
    pub expected_head: u64,
    pub roster: RosterBytes,
    pub seals: Vec<RecipientSealEntry>,
    pub invite_seals: Vec<InviteSealEntry>,
    pub revoke: Vec<AccountIdWire>,
}

/// An account ID in a list (the `with` attribute cannot apply to elements).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountIdWire(pub AccountId);

impl Serialize for AccountIdWire {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        uuid_str::serialize(&self.0, s)
    }
}

impl<'de> Deserialize<'de> for AccountIdWire {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        uuid_str::deserialize(d).map(Self)
    }
}

/// Detail of `members_mismatch`: account and invite IDs as UUID text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MembersMismatchDetail {
    pub members: Vec<AccountIdWire>,
    pub invites: Vec<WireIdItem>,
}

/// An invite ID in a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireIdItem(pub WireId);

impl Serialize for WireIdItem {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        uuid_str::serialize(&self.0, s)
    }
}

impl<'de> Deserialize<'de> for WireIdItem {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        uuid_str::deserialize(d).map(Self)
    }
}

/// `PUT /v1/vaults/{v}/keys/{g}/claim` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaimRequest {
    #[serde(with = "wrapped_key")]
    pub wrapped: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device: InstallId,
}

/// A seal of one vault at one generation (emergency grants and releases).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultSealEntry {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    pub generation: u32,
    pub seal: SealBytes,
}

/// `PUT /v1/emergency/grants/{grantee}` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantRequest {
    pub wait_days: u8,
    pub scope: u8,
    pub seals: Vec<VaultSealEntry>,
}

/// `PUT /v1/emergency/grants/{grantee}/seals` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SealsRequest {
    pub seals: Vec<VaultSealEntry>,
}

/// `POST /v1/emergency/grants/{grantor}/release` answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseResponse {
    pub seals: Vec<VaultSealEntry>,
}

/// `POST /v1/emergency/grants/{grantor}/request` answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseAtResponse {
    pub release_at: u64,
}

/// Detail of `emergency_waiting` on a release.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaitingDetail {
    pub release_at: u64,
}

/// Detail of `emergency_cooldown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CooldownDetail {
    pub until: u64,
}

/// A vault and its seal generation in a grant listing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultGenerationEntry {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
}

/// A grant the caller gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantedEntry {
    #[serde(with = "uuid_str")]
    pub grantee: AccountId,
    pub email: String,
    pub wait_days: u8,
    pub scope: u8,
    pub vaults: Vec<VaultGenerationEntry>,
    pub state: u8,
    #[serde(deserialize_with = "required")]
    pub requested_at: Option<u64>,
    #[serde(deserialize_with = "required")]
    pub release_at: Option<u64>,
}

/// A grant the caller received.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceivedEntry {
    #[serde(with = "uuid_str")]
    pub grantor: AccountId,
    pub email: String,
    pub wait_days: u8,
    pub state: u8,
    #[serde(deserialize_with = "required")]
    pub requested_at: Option<u64>,
    #[serde(deserialize_with = "required")]
    pub release_at: Option<u64>,
    #[serde(deserialize_with = "required")]
    pub cooldown_until: Option<u64>,
}

/// `GET /v1/emergency`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyResponse {
    pub granted: Vec<GrantedEntry>,
    pub received: Vec<ReceivedEntry>,
}

/// A key share in a sharing-aware changes page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeShare {
    pub generation: u32,
    #[serde(with = "uuid_str")]
    pub sender: AccountId,
    pub seal: SealBytes,
    pub seq: u64,
}

/// A roster in a sharing-aware changes page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeRoster {
    pub generation: u32,
    pub ciphertext: RosterBytes,
    pub seq: u64,
}

/// `GET /v1/vaults/{v}/changes?sharing=1`: the M6 page plus the caller's
/// role, the unclaimed shares addressed to them and the rosters newer than
/// `since`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SharingChangesResponse {
    pub head_seq: u64,
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    pub keys: Vec<super::ChangeKey>,
    #[serde(deserialize_with = "required")]
    pub meta: Option<super::ChangeMeta>,
    pub records: Vec<super::ChangeRecord>,
    pub more: bool,
    pub next_since: u64,
    pub role: u8,
    pub shares: Vec<ChangeShare>,
    pub rosters: Vec<ChangeRoster>,
}
