//! Conversions between the item model's OTP configuration
//! ([`mosaic_core::model::OtpConfig`], stored in item plaintext) and
//! `mosaic-totp`'s generator ([`Otp`]) and `otpauth://` entries
//! ([`OtpAuth`]).
//!
//! The model stores the HOTP counter as a separate, grow-only field of the
//! item ([`mosaic_core::Item::otp_counter`]), so the conversions take or
//! return it next to the configuration.

use mosaic_core::SecretVec;
use mosaic_core::model::{OtpAlgorithm, OtpConfig, OtpKind, Text};
use mosaic_totp::{Algorithm, Digits, Otp, OtpAuth, OtpSecret, Period, SecretText};

use crate::session::Result;

const fn algorithm_to_totp(a: OtpAlgorithm) -> Algorithm {
    match a {
        OtpAlgorithm::Sha1 => Algorithm::Sha1,
        OtpAlgorithm::Sha256 => Algorithm::Sha256,
        OtpAlgorithm::Sha512 => Algorithm::Sha512,
    }
}

const fn algorithm_from_totp(a: Algorithm) -> OtpAlgorithm {
    match a {
        Algorithm::Sha1 => OtpAlgorithm::Sha1,
        Algorithm::Sha256 => OtpAlgorithm::Sha256,
        Algorithm::Sha512 => OtpAlgorithm::Sha512,
    }
}

/// The `mosaic-totp` generator for a stored configuration. `hotp_counter`
/// is the item's counter (ignored for TOTP and Steam).
///
/// # Errors
/// [`crate::session::Error::Otp`] if the configuration is outside what
/// `mosaic-totp` accepts: an empty secret or one above
/// [`mosaic_totp::MAX_SECRET_LEN`] bytes, or digits or a period out of
/// range. A configuration that passed [`OtpConfig::validate`] never fails
/// here: the model's limits are the same
/// ([`mosaic_core::model::MAX_OTP_SECRET_LEN`]).
pub fn otp_from_config(config: &OtpConfig, hotp_counter: u64) -> Result<Otp> {
    let secret = OtpSecret::from_bytes(config.secret.expose_secret())?;
    let kind = match config.kind {
        OtpKind::Totp => mosaic_totp::OtpKind::Totp {
            digits: Digits::new(config.digits)?,
            period: Period::new(config.period)?,
        },
        OtpKind::Hotp => mosaic_totp::OtpKind::Hotp {
            digits: Digits::new(config.digits)?,
            counter: hotp_counter,
        },
        OtpKind::Steam => mosaic_totp::OtpKind::Steam {
            period: Period::new(config.period)?,
        },
    };
    Ok(Otp {
        secret,
        algorithm: algorithm_to_totp(config.algorithm),
        kind,
    })
}

/// The stored configuration for a generator, and the HOTP counter to store
/// next to it (0 for TOTP and Steam). `issuer` and `label` are the
/// `otpauth://` issuer and account name.
///
/// # Errors
/// [`crate::session::Error::Core`] if the secret cannot be copied or the
/// result fails [`OtpConfig::validate`].
pub fn config_from_otp(
    otp: &Otp,
    issuer: Option<&str>,
    label: Option<&str>,
) -> Result<(OtpConfig, u64)> {
    let mut config = OtpConfig::totp(SecretVec::from_slice(otp.secret.expose_secret())?);
    config.algorithm = algorithm_from_totp(otp.algorithm);
    let counter = match otp.kind {
        mosaic_totp::OtpKind::Totp { digits, period } => {
            config.kind = OtpKind::Totp;
            config.digits = digits.get();
            config.period = period.get();
            0
        }
        mosaic_totp::OtpKind::Hotp { digits, counter } => {
            config.kind = OtpKind::Hotp;
            config.digits = digits.get();
            counter
        }
        mosaic_totp::OtpKind::Steam { period } => {
            config.kind = OtpKind::Steam;
            config.digits = 5;
            config.period = period.get();
            0
        }
    };
    config.issuer = issuer.filter(|s| !s.is_empty()).map(Text::new);
    config.label = label.filter(|s| !s.is_empty()).map(Text::new);
    config.validate()?;
    Ok((config, counter))
}

/// The stored configuration for an imported `otpauth://` entry
/// ([`OtpAuth::parse`]), and its HOTP counter.
///
/// # Errors
/// As [`config_from_otp`].
pub fn config_from_otp_auth(entry: &OtpAuth) -> Result<(OtpConfig, u64)> {
    config_from_otp(
        &entry.otp,
        entry.issuer.as_ref().map(SecretText::expose_secret),
        Some(entry.account_name.expose_secret()),
    )
}

/// The `otpauth://` entry for a stored configuration, for export
/// ([`OtpAuth::to_uri`]). Issuer and label are copied into the entry's
/// zeroizing [`SecretText`] buffers, never into plain strings.
///
/// # Errors
/// As [`otp_from_config`].
pub fn otp_auth_from_config(config: &OtpConfig, hotp_counter: u64) -> Result<OtpAuth> {
    Ok(OtpAuth {
        otp: otp_from_config(config, hotp_counter)?,
        issuer: config.issuer.as_ref().map(|t| SecretText::new(t.as_str())),
        account_name: SecretText::new(config.label.as_ref().map_or("", Text::as_str)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET: &[u8] = b"12345678901234567890";

    fn config(kind: OtpKind, digits: u8, period: u32) -> OtpConfig {
        let mut c = OtpConfig::totp(SecretVec::from_slice(RFC_SECRET).unwrap());
        c.kind = kind;
        c.digits = digits;
        c.period = period;
        c
    }

    #[test]
    fn totp_matches_rfc6238() {
        // RFC 6238 Appendix B, SHA-1, T = 59 -> 94287082 (8 digits).
        let otp = otp_from_config(&config(OtpKind::Totp, 8, 30), 0).unwrap();
        assert_eq!(otp.code_at(59).unwrap().expose_secret(), "94287082");
        assert_eq!(otp.seconds_remaining(59), Some(1));
    }

    #[test]
    fn hotp_uses_the_counter() {
        // RFC 4226 Appendix D: counter 1 -> 287082.
        let otp = otp_from_config(&config(OtpKind::Hotp, 6, 30), 1).unwrap();
        assert_eq!(otp.code_at(0).unwrap().expose_secret(), "287082");
        assert_eq!(otp.seconds_remaining(0), None);
    }

    #[test]
    fn algorithms_and_steam_round_trip() {
        for (alg, kind, digits) in [
            (OtpAlgorithm::Sha1, OtpKind::Steam, 5),
            (OtpAlgorithm::Sha256, OtpKind::Totp, 7),
            (OtpAlgorithm::Sha512, OtpKind::Hotp, 8),
        ] {
            let mut c = config(kind, digits, 45);
            c.algorithm = alg;
            c.issuer = Some(Text::new("Example"));
            c.label = Some(Text::new("alice"));
            let otp = otp_from_config(&c, 7).unwrap();
            let (back, counter) = config_from_otp(&otp, Some("Example"), Some("alice")).unwrap();
            assert_eq!(back.algorithm, alg);
            assert_eq!(back.kind, kind);
            assert_eq!(back.digits, digits);
            assert_eq!(back.secret.expose_secret(), RFC_SECRET);
            assert_eq!(counter, if kind == OtpKind::Hotp { 7 } else { 0 });
            if kind != OtpKind::Hotp {
                assert_eq!(back.period, 45);
            }
            let entry = otp_auth_from_config(&c, 7).unwrap();
            assert_eq!(
                entry.issuer.as_ref().map(SecretText::expose_secret),
                Some("Example")
            );
            assert_eq!(entry.account_name.expose_secret(), "alice");
            // Item content: never in the entry's Debug output.
            let shown = format!("{entry:?}");
            assert!(!shown.contains("Example") && !shown.contains("alice"));
        }
    }

    #[test]
    fn otpauth_import_and_export() {
        let entry = OtpAuth::parse(
            "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example",
        )
        .unwrap();
        let (c, counter) = config_from_otp_auth(&entry).unwrap();
        assert_eq!(counter, 0);
        assert_eq!(c.issuer.as_ref().map(Text::as_str), Some("Example"));
        assert_eq!(
            c.label.as_ref().map(Text::as_str),
            Some("alice@example.com")
        );
        let uri = otp_auth_from_config(&c, 0).unwrap().to_uri().unwrap();
        let again = OtpAuth::parse(uri.expose_secret()).unwrap();
        assert_eq!(again, entry);
    }

    #[test]
    fn model_and_generator_limits_agree() {
        // Every secret the model stores can produce codes.
        assert_eq!(
            mosaic_core::model::MAX_OTP_SECRET_LEN,
            mosaic_totp::MAX_SECRET_LEN
        );
        let max =
            OtpConfig::totp(SecretVec::from_slice(&[7u8; mosaic_totp::MAX_SECRET_LEN]).unwrap());
        max.validate().unwrap();
        assert!(otp_from_config(&max, 0).is_ok());
    }

    #[test]
    fn out_of_range_configs_are_errors() {
        use crate::session::Error;
        let otp_err = |c: &OtpConfig| match otp_from_config(c, 0) {
            Err(Error::Otp(e)) => e,
            other => panic!("expected an OTP error, got {other:?}"),
        };
        let mut c = OtpConfig::totp(
            SecretVec::from_slice(&[7u8; mosaic_totp::MAX_SECRET_LEN + 1]).unwrap(),
        );
        assert_eq!(otp_err(&c), mosaic_totp::Error::SizeLimit);
        c = config(OtpKind::Totp, 9, 30);
        assert_eq!(otp_err(&c), mosaic_totp::Error::InvalidDigits);
        c = config(OtpKind::Hotp, 5, 30);
        assert_eq!(otp_err(&c), mosaic_totp::Error::InvalidDigits);
        c = config(OtpKind::Totp, 6, 301);
        assert_eq!(otp_err(&c), mosaic_totp::Error::InvalidPeriod);
        c = config(OtpKind::Steam, 5, 0);
        assert_eq!(otp_err(&c), mosaic_totp::Error::InvalidPeriod);
        let entry = OtpAuth::parse("otpauth://totp/a?secret=JBSWY3DPEHPK3PXP").unwrap();
        let (c, _) = config_from_otp_auth(&entry).unwrap();
        assert!(c.issuer.is_none());
        assert_eq!(c.label.as_ref().map(Text::as_str), Some("a"));
    }
}
