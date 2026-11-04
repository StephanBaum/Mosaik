//! One-time-code setup (ADR-0031): scan or parse, preview, live code, discard.
//! The parsed secret stays in Rust behind `pending_id`; no type here carries it.

use serde::{Deserialize, Serialize};

use super::items::OtpScheme;
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// Where `otp_scan` looks.
    pub enum OtpScanSource {
        /// Every display, once.
        Screen,
        /// An image on the clipboard.
        Clipboard,
        /// A PNG or JPEG the user picks in the native file dialog.
        File,
    }
);

wire_enum!(
    /// HMAC hash of a one-time code.
    pub enum OtpAlgorithm {
        /// SHA-1 (the default of nearly every site).
        Sha1,
        /// SHA-256.
        Sha256,
        /// SHA-512.
        Sha512,
    }
);

wire_enum!(
    /// Secret length class (`mosaic_totp::SecretStrength`).
    pub enum OtpSecretStrength {
        /// Shorter than 10 bytes.
        Weak,
        /// 10 to 15 bytes: common, below the RFC 4226 minimum.
        BelowRfc4226,
        /// 16 bytes or more.
        Rfc4226,
    }
);

/// `otp_scan`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtpScanRequest {
    /// Where to look.
    pub source: OtpScanSource,
}

/// A parsed setup waiting for the user's confirmation, without its secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpPending {
    /// Opaque ID of the slot Rust holds the secret in (`otp-<n>`).
    pub pending_id: String,
    /// The scheme.
    pub scheme: OtpScheme,
    /// The issuer (`null` if none).
    pub issuer: Option<String>,
    /// The account name (`null` when empty).
    pub account_name: Option<String>,
    /// The hash.
    pub algorithm: OtpAlgorithm,
    /// Code length (5 for Steam).
    pub digits: u8,
    /// Time step in seconds (`null` for HOTP).
    pub period: Option<u32>,
    /// Secret length class.
    pub strength: OtpSecretStrength,
    /// Seconds the slot lives (600 at creation).
    pub expires_in_seconds: u32,
}

/// The `otp_scan` response.
#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OtpScanResult {
    /// A setup code was found.
    Found {
        /// The preview.
        pending: OtpPending,
        /// Rust emptied the clipboard after reading the screenshot.
        clipboard_cleared: bool,
    },
    /// The file dialog was dismissed.
    Cancelled,
}

/// `otp_parse`: manual entry.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OtpParseRequest {
    /// An `otpauth://` link.
    Uri {
        /// The link (it contains the shared secret).
        uri: SecretIn,
    },
    /// A bare base32 key; always makes a TOTP.
    Key {
        /// The key (spaces and hyphens are ignored).
        secret: SecretIn,
        /// The service.
        issuer: Option<String>,
        /// The account.
        account_name: Option<String>,
        /// The hash.
        algorithm: OtpAlgorithm,
        /// Code length, 6 to 8.
        digits: u8,
        /// Time step in seconds, 1 to 300.
        period: u32,
    },
}

/// `otp_pending_code` and `otp_pending_discard`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OtpPendingRequest {
    /// The slot.
    pub pending_id: String,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn scan_requests() -> Vec<OtpScanRequest> {
        OtpScanSource::ALL
            .iter()
            .map(|&source| OtpScanRequest { source })
            .collect()
    }

    pub(crate) fn parse_requests() -> Vec<OtpParseRequest> {
        vec![
            OtpParseRequest::Uri {
                uri: SecretIn::test(
                    "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP&issuer=GitHub",
                ),
            },
            OtpParseRequest::Key {
                secret: SecretIn::test("JBSW Y3DP EHPK 3PXP"),
                issuer: Some("GitHub".into()),
                account_name: None,
                algorithm: OtpAlgorithm::Sha1,
                digits: 6,
                period: 30,
            },
        ]
    }

    pub(crate) fn pending_requests() -> Vec<OtpPendingRequest> {
        vec![OtpPendingRequest {
            pending_id: "otp-1".into(),
        }]
    }

    pub(crate) fn previews() -> Vec<OtpPending> {
        vec![
            OtpPending {
                pending_id: "otp-1".into(),
                scheme: OtpScheme::Totp,
                issuer: Some("GitHub".into()),
                account_name: Some("alex@example.com".into()),
                algorithm: OtpAlgorithm::Sha1,
                digits: 6,
                period: Some(30),
                strength: OtpSecretStrength::BelowRfc4226,
                expires_in_seconds: 600,
            },
            OtpPending {
                pending_id: "otp-2".into(),
                scheme: OtpScheme::Hotp,
                issuer: None,
                account_name: None,
                algorithm: OtpAlgorithm::Sha256,
                digits: 8,
                period: None,
                strength: OtpSecretStrength::Rfc4226,
                expires_in_seconds: 600,
            },
            OtpPending {
                pending_id: "otp-3".into(),
                scheme: OtpScheme::Steam,
                issuer: Some("Steam".into()),
                account_name: Some("gabe".into()),
                algorithm: OtpAlgorithm::Sha1,
                digits: 5,
                period: Some(30),
                strength: OtpSecretStrength::Weak,
                expires_in_seconds: 590,
            },
        ]
    }

    pub(crate) fn scan_results() -> Vec<OtpScanResult> {
        let mut found: Vec<OtpScanResult> = previews()
            .into_iter()
            .take(1)
            .map(|pending| OtpScanResult::Found {
                pending,
                clipboard_cleared: true,
            })
            .collect();
        found.push(OtpScanResult::Cancelled);
        found
    }
}
