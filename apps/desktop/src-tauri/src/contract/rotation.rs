//! Account keys (M7, `docs/DESKTOP_IPC.md` §3 "Account keys"): the Secret Key
//! rotation, the KDF profile upgrade and the full account re-key, with their
//! journal. The master password change keeps its M6 command
//! (`change_master_password`). Typed secrets travel as [`SecretIn`]. The new
//! Secret Key never reaches the webview: Rust generates it, keeps it in the
//! pending keystore entry and prints it only into the Emergency Kit PDF.

use serde::{Deserialize, Serialize};

use super::ids::{WireAccountId, WireVaultId};
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// What a rotation job renews.
    pub enum RotationKind {
        /// A new Secret Key, same account key.
        SecretKey,
        /// A full re-key: new account key, vault keys and identity.
        Rekey,
        /// A stronger key derivation, same keys.
        Profile,
        /// One vault's key (reserved: the job exists in the journal).
        Vault,
    }
);

wire_enum!(
    /// Where a rotation job stands (CRYPTO_SPEC §6.6, phases 1 to 7).
    pub enum RotationPhaseName {
        /// The kit is saved and verified; nothing was sent.
        Prepared,
        /// The change was sent to the server.
        BackendStarted,
        /// The server accepted the change.
        BackendAccepted,
        /// The new account file is in the folder.
        FolderWritten,
        /// This device holds the new keys.
        LocalCommitted,
        /// Attachments are being protected again.
        Attachments,
        /// The re-protected data is being uploaded.
        Uploading,
    }
);

wire_enum!(
    /// What the user must type to continue a job.
    pub enum RotationNeeds {
        /// Nothing: the job continues by itself.
        None,
        /// The master password (the wizard starts again at that step).
        Password,
        /// The new master password and the last group of the new Secret Key
        /// from the new Emergency Kit.
        KitGroup,
    }
);

wire_enum!(
    /// The protection profiles `profile_upgrade` offers (never weaker than
    /// the account's own).
    pub enum RotationProfile {
        /// 256 MiB.
        Standard,
        /// 512 MiB.
        Strong,
        /// 1 GiB.
        Max,
    }
);

wire_enum!(
    /// Where the account's data is synced, for the review.
    pub enum RekeyBackend {
        /// Only on this device.
        Local,
        /// The Mosaic server.
        Hosted,
        /// A folder.
        Folder,
    }
);

/// An open rotation job.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationJobView {
    /// What is renewed.
    pub kind: RotationKind,
    /// Where it stands.
    pub phase: RotationPhaseName,
    /// Steps done (attachments protected again).
    pub done: u32,
    /// Steps in all.
    pub total: u32,
    /// What continuing needs from the user.
    pub needs: RotationNeeds,
}

/// The `rotation_status` response, and what `rotation_resume` returns.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationStatus {
    /// The open job (`null` if none).
    pub job: Option<RotationJobView>,
}

/// What a finished (or paused) rotation did, for the last wizard step.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationDone {
    /// Quick unlock was turned off (set it up again).
    pub quick_unlock_removed: bool,
    /// How many security-key unlocks were turned off (register them again).
    pub hardware_keys_removed: u32,
    /// Attachments that could not be protected again.
    pub unreachable_attachments: u32,
    /// Everything is done. `false`: the job continues on the next sync or
    /// from "Continue key rotation".
    pub finished: bool,
}

/// `secret_key_rotate_prepare`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotatePrepareRequest {
    /// The master password.
    pub password: SecretIn,
}

/// `secret_key_rotate_commit`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotateCommitRequest {
    /// The master password.
    pub password: SecretIn,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `profile_upgrade`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileUpgradeRequest {
    /// The master password.
    pub password: SecretIn,
    /// The new profile.
    pub profile: RotationProfile,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
    /// Go on although this device needs more than two seconds per unlock
    /// with the new profile (`profileTooSlow` otherwise).
    pub force: bool,
}

/// An owned shared vault in the review.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedOwnedView {
    /// The vault.
    pub vault_id: WireVaultId,
    /// Its name (`null` if unknown).
    pub name: Option<String>,
    /// How many people besides you can read it.
    pub members: u32,
}

/// A vault someone else owns, in the review.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberVaultView {
    /// The vault.
    pub vault_id: WireVaultId,
    /// Its name (`null` if unknown).
    pub name: Option<String>,
}

/// The `rekey_review` response: what a re-key touches.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekeyReviewView {
    /// Vaults only you use: they get new keys.
    pub owned_vaults: u32,
    /// Vaults you share: new keys, and every member must check your new
    /// safety code.
    pub shared_owned: Vec<SharedOwnedView>,
    /// Vaults others share with you: their owners are asked to renew them.
    pub member_vaults: Vec<MemberVaultView>,
    /// Hardware-protected vaults: not affected.
    pub hardware_vaults: u32,
    /// People who hold an emergency access grant from you.
    pub grants: u32,
    /// Registered security keys (their unlock is turned off).
    pub hardware_keys: u32,
    /// Quick unlock is set up (it is turned off).
    pub quick_unlock: bool,
    /// Attachments this device does not hold (they must be downloaded).
    pub unreachable_attachments: u32,
    /// Where the data is synced.
    pub backend: RekeyBackend,
    /// The account's ID, for the new Emergency Kit.
    pub account_id: WireAccountId,
}

/// `rekey_prepare`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyPrepareRequest {
    /// The current master password.
    pub password: SecretIn,
    /// The new master password (it must differ).
    pub new_password: SecretIn,
}

/// `rekey_commit`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RekeyCommitRequest {
    /// The current master password.
    pub password: SecretIn,
    /// The new master password.
    pub new_password: SecretIn,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `rotation_resume`: what the job's `needs` asks for.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationResumeRequest {
    /// The master password the account has after the job (the new one for
    /// a re-key, the unchanged one otherwise); `null` if `needs` is `none`.
    pub password: Option<SecretIn>,
    /// The last group of the new Secret Key, from the new kit (`needs`
    /// `kitGroup`).
    pub kit_group: Option<SecretIn>,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `sync_dismiss_account_file`: an account file in the sync folder that this
/// device cannot verify is not a problem the user wants to see again.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DismissFileRequest {
    /// The file's name, as the warning showed it.
    pub name: String,
    /// The file's SHA-256 (64 hex digits), as the warning showed it.
    pub sha256: String,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn pw() -> SecretIn {
        SecretIn::test("correct horse battery staple")
    }

    pub(crate) fn statuses() -> Vec<RotationStatus> {
        let mut all = vec![RotationStatus { job: None }];
        for (kind, phase, needs) in [
            (
                RotationKind::SecretKey,
                RotationPhaseName::Prepared,
                RotationNeeds::Password,
            ),
            (
                RotationKind::Rekey,
                RotationPhaseName::BackendAccepted,
                RotationNeeds::KitGroup,
            ),
            (
                RotationKind::Rekey,
                RotationPhaseName::Attachments,
                RotationNeeds::None,
            ),
            (
                RotationKind::Profile,
                RotationPhaseName::LocalCommitted,
                RotationNeeds::None,
            ),
        ] {
            all.push(RotationStatus {
                job: Some(RotationJobView {
                    kind,
                    phase,
                    done: 3,
                    total: 12,
                    needs,
                }),
            });
        }
        all
    }

    pub(crate) fn done() -> Vec<RotationDone> {
        vec![
            RotationDone {
                quick_unlock_removed: true,
                hardware_keys_removed: 1,
                unreachable_attachments: 2,
                finished: true,
            },
            RotationDone {
                quick_unlock_removed: false,
                hardware_keys_removed: 0,
                unreachable_attachments: 0,
                finished: false,
            },
        ]
    }

    pub(crate) fn reviews() -> Vec<RekeyReviewView> {
        vec![
            RekeyReviewView {
                owned_vaults: 2,
                shared_owned: vec![SharedOwnedView {
                    vault_id: ids::vault(2),
                    name: Some("Family".into()),
                    members: 2,
                }],
                member_vaults: vec![MemberVaultView {
                    vault_id: ids::vault(4),
                    name: None,
                }],
                hardware_vaults: 1,
                grants: 1,
                hardware_keys: 1,
                quick_unlock: true,
                unreachable_attachments: 3,
                backend: RekeyBackend::Hosted,
                account_id: ids::account(),
            },
            RekeyReviewView {
                owned_vaults: 1,
                shared_owned: vec![],
                member_vaults: vec![],
                hardware_vaults: 0,
                grants: 0,
                hardware_keys: 0,
                quick_unlock: false,
                unreachable_attachments: 0,
                backend: RekeyBackend::Folder,
                account_id: ids::account(),
            },
        ]
    }
}
