//! The JSON bodies of the account re-key routes (M7 spec §4.2). They mirror
//! `crates/server/src/api/wire.rs`: camelCase keys, every key present,
//! unknown keys refused, IDs as lowercase hyphenated UUIDs, bytes as
//! base64url without padding.

use mosaic_core::{AccountId, VaultId, WrappedKey};
use serde::{Deserialize, Serialize};

use super::sharing::{IdentityBody, InviteSealEntry, RecipientSealEntry, RosterBytes, SealBytes};
use super::{B64, B64Fixed, profile, required, uuid_str, wrapped_key};

/// `POST /v1/account/rekey/start` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyStartRequest {
    pub registration_request: B64<4096>,
    pub ak_epoch: u32,
}

/// One owned vault of a re-key: its new key at the next generation and, for
/// a shared vault, the roster and the seals from the new identity key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyVaultEntry {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    pub generation: u32,
    #[serde(with = "wrapped_key")]
    pub wrapped: WrappedKey,
    pub expected_head: u64,
    #[serde(deserialize_with = "required")]
    pub roster: Option<RosterBytes>,
    pub seals: Vec<RecipientSealEntry>,
    pub invite_seals: Vec<InviteSealEntry>,
}

/// The current key of a vault someone else owns, re-wrapped under the new
/// account key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyMemberRewrap {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    pub generation: u32,
    #[serde(with = "wrapped_key")]
    pub wrapped: WrappedKey,
}

/// A fresh emergency seal from the new identity key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyEmergencySeal {
    #[serde(with = "uuid_str")]
    pub grantee: AccountId,
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    pub generation: u32,
    pub seal: SealBytes,
}

/// `POST /v1/account/rekey/finish` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyFinishRequest {
    pub registration_upload: B64<4096>,
    #[serde(with = "wrapped_key")]
    pub wrapped_ak: WrappedKey,
    pub salt: B64Fixed<16>,
    /// The profile number (1 to 4), as the server's body takes it.
    pub profile: u8,
    pub ak_epoch: u32,
    #[serde(deserialize_with = "required")]
    pub identity: Option<IdentityBody>,
    pub vaults: Vec<RekeyVaultEntry>,
    pub member_rewraps: Vec<RekeyMemberRewrap>,
    pub emergency_seals: Vec<RekeyEmergencySeal>,
    #[serde(with = "uuid_str")]
    pub device: mosaic_core::InstallId,
}

/// `POST /v1/account/rekey/finish` answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyFinishResponse {
    pub account_version: u64,
    pub ak_epoch: u32,
}

/// `GET /v1/account?epoch=1` (with or without `identity=1`): the account plus
/// its key epoch and, when asked for, its identity pair. The epoch is public
/// metadata the server states; it is not authenticated (THREAT_MODEL R84).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountWithEpoch {
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
    pub ak_epoch: u32,
    #[serde(default, deserialize_with = "required")]
    pub identity: Option<IdentityBody>,
}

/// Detail of `epoch_mismatch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EpochMismatchDetail {
    pub current_epoch: u32,
}

/// Detail of `rekey_mismatch`: which list of the request did not match what
/// the server holds (`vaults`, `memberRewraps` or `emergencySeals`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyMismatchDetail {
    pub set: String,
}
