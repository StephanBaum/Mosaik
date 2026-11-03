//! Quick unlock: the status of the biometric method and of the registered
//! security keys, and the requests that enrol or remove them (CRYPTO_SPEC
//! §6.2, §6.3; ADR-0028). No feature-crate imports.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireVaultId};
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// Which OS facility the biometric method uses.
    pub enum BiometricPlatformKind {
        /// Windows Hello (a TPM-backed `KeyCredentialManager` key).
        WindowsHello,
        /// Touch ID (a Secure Enclave key with a biometry-only access control).
        TouchId,
    }
);

/// The biometric method on this device. Linux has none: `platform` is
/// `null` there and the UI hides the row.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricStatus {
    /// The OS facility, or `null` on a system without one.
    pub platform: Option<BiometricPlatformKind>,
    /// Whether the OS offers it right now (hardware present and set up).
    pub available: bool,
    /// Whether it is set up and not expired.
    pub enrolled: bool,
    /// When it was set up (`null` unless `enrolled`).
    pub enrolled_at: Option<UnixMillis>,
    /// When it expires (`null` unless `enrolled`).
    pub expires_at: Option<UnixMillis>,
}

/// A registered security key.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareKeySummary {
    /// `SHA-256(credential_id)[..16]` as 32 lowercase hex digits.
    pub method_id: String,
    /// When it was registered.
    pub added_at: UnixMillis,
}

/// The security keys registered for unlock.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareKeyStatus {
    /// Whether this build can talk to security keys at all. While `false`
    /// every hardware-key control stays hidden.
    pub supported: bool,
    /// The registered keys.
    pub keys: Vec<HardwareKeySummary>,
    /// The most keys that can be registered.
    pub max_keys: u32,
}

/// The `quick_unlock_status` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickUnlockStatus {
    /// The biometric method.
    pub biometric: BiometricStatus,
    /// The security keys.
    pub hardware_keys: HardwareKeyStatus,
    /// How long a biometric enrollment is used (1 to 30 days).
    pub max_age_days: u32,
}

/// `enroll_biometric`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnrollBiometricRequest {
    /// The master password, or `null` within two minutes of a right one.
    pub password: Option<SecretIn>,
}

/// `enroll_hardware_key`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnrollHardwareKeyRequest {
    /// The master password, or `null` within two minutes of a right one.
    pub password: Option<SecretIn>,
    /// The key's PIN, if it has one set.
    pub pin: Option<SecretIn>,
}

/// `remove_hardware_key`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveHardwareKeyRequest {
    /// The key (32 lowercase hex digits).
    pub method_id: String,
}

/// `create_hardware_vault`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateHardwareVaultRequest {
    /// The vault's name.
    pub name: String,
    /// The key's PIN, if it has one set.
    pub pin: Option<SecretIn>,
}

/// `open_hardware_vault` and `add_hardware_vault_key`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareVaultRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The key's PIN, if it has one set.
    pub pin: Option<SecretIn>,
}

/// `remove_hardware_vault_key`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareVaultKeyRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The key (32 lowercase hex digits).
    pub method_id: String,
}

/// The keys of a hardware-protected vault.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareVaultKeys {
    /// The registered keys.
    pub keys: Vec<HardwareKeySummary>,
}

/// Lowercase hex of a method ID.
pub(crate) fn method_id_hex(method_id: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    method_id
        .iter()
        .fold(String::with_capacity(32), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// A method ID from exactly 32 lowercase hex digits.
pub(crate) fn parse_method_id(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    for (byte, [high, low]) in out.iter_mut().zip(pairs) {
        *byte = (digit(*high)? << 4) | digit(*low)?;
    }
    Some(out)
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn statuses() -> Vec<QuickUnlockStatus> {
        vec![
            QuickUnlockStatus {
                biometric: BiometricStatus {
                    platform: Some(BiometricPlatformKind::WindowsHello),
                    available: true,
                    enrolled: true,
                    enrolled_at: Some(1_761_480_000_000),
                    expires_at: Some(1_762_689_600_000),
                },
                hardware_keys: HardwareKeyStatus {
                    supported: true,
                    keys: vec![HardwareKeySummary {
                        method_id: "0123456789abcdef0123456789abcdef".into(),
                        added_at: 1_761_480_000_000,
                    }],
                    max_keys: 8,
                },
                max_age_days: 14,
            },
            QuickUnlockStatus {
                biometric: BiometricStatus {
                    platform: None,
                    available: false,
                    enrolled: false,
                    enrolled_at: None,
                    expires_at: None,
                },
                hardware_keys: HardwareKeyStatus {
                    supported: false,
                    keys: vec![],
                    max_keys: 8,
                },
                max_age_days: 30,
            },
        ]
    }

    pub(crate) fn enroll_hardware_key() -> Vec<EnrollHardwareKeyRequest> {
        vec![
            EnrollHardwareKeyRequest {
                password: None,
                pin: None,
            },
            EnrollHardwareKeyRequest {
                password: Some(SecretIn::test("correct horse battery staple")),
                pin: Some(SecretIn::test("246810")),
            },
        ]
    }

    pub(crate) fn enroll_biometric() -> Vec<EnrollBiometricRequest> {
        vec![
            EnrollBiometricRequest { password: None },
            EnrollBiometricRequest {
                password: Some(SecretIn::test("correct horse battery staple")),
            },
        ]
    }

    pub(crate) fn key_summaries() -> Vec<HardwareVaultKeys> {
        vec![
            HardwareVaultKeys { keys: vec![] },
            HardwareVaultKeys {
                keys: statuses()[0].hardware_keys.keys.clone(),
            },
        ]
    }

    pub(crate) fn vault_request() -> HardwareVaultRequest {
        HardwareVaultRequest {
            vault_id: ids::vault(1),
            pin: None,
        }
    }
}
