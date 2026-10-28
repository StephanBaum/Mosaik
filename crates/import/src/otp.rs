//! One-time-password values as export files store them: `otpauth://` URIs,
//! Bitwarden's `steam://` secrets, bare base32 secrets, and KeePass's
//! `TimeOtp-*` / `HmacOtp-*` and `TOTP Seed` / `TOTP Settings` strings.
//!
//! Everything converts to the item model's [`OtpConfig`] through
//! `mosaic-totp` and `mosaic_vault::otp`, so the stored configuration is
//! always one that produces codes.

use mosaic_core::SecretVec;
use mosaic_core::model::{OtpAlgorithm, OtpConfig, OtpKind};
use mosaic_totp::{OtpAuth, OtpSecret};
use mosaic_vault::otp::config_from_otp_auth;

use crate::buf::{self, Buf};

/// An OTP configuration and its HOTP counter (0 for TOTP and Steam).
pub(crate) type Parsed = (OtpConfig, u64);

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
}

/// Parses an OTP value: an `otpauth://` URI, a `steam://` secret, or a
/// bare base32 secret (TOTP with the RFC 6238 defaults). `None` if the
/// value is none of these or is out of range.
pub(crate) fn parse(text: &str) -> Option<Parsed> {
    let t = text.trim();
    if starts_with_ignore_case(t, "otpauth://") {
        let auth = OtpAuth::parse(t).ok()?;
        return config_from_otp_auth(&auth).ok();
    }
    if starts_with_ignore_case(t, "steam://") {
        let secret = OtpSecret::from_base32(t.get("steam://".len()..)?).ok()?;
        return steam(secret.expose_secret());
    }
    let secret = OtpSecret::from_base32(t).ok()?;
    totp(secret.expose_secret(), OtpAlgorithm::Sha1, 6, 30)
}

fn config(secret: &[u8]) -> Option<OtpConfig> {
    Some(OtpConfig::totp(SecretVec::from_slice(secret).ok()?))
}

fn checked(c: OtpConfig, counter: u64) -> Option<Parsed> {
    c.validate().ok()?;
    Some((c, counter))
}

fn totp(secret: &[u8], algorithm: OtpAlgorithm, digits: u8, period: u32) -> Option<Parsed> {
    let mut c = config(secret)?;
    c.algorithm = algorithm;
    c.digits = digits;
    c.period = period;
    checked(c, 0)
}

fn steam(secret: &[u8]) -> Option<Parsed> {
    let mut c = config(secret)?;
    c.kind = OtpKind::Steam;
    c.digits = 5;
    checked(c, 0)
}

/// The KeePass `TimeOtp-*` / `HmacOtp-*` attributes of one entry (KeePass
/// 2.47+), and the `TOTP Seed` / `TOTP Settings` pair of the `KeeTrayTOTP`
/// plugin.
#[derive(Default)]
pub(crate) struct KeepassOtp {
    /// `TimeOtp-Secret*` or `HmacOtp-Secret*`, decoded.
    secret: Option<Buf>,
    hotp: bool,
    length: Option<u8>,
    period: Option<u32>,
    algorithm: Option<OtpAlgorithm>,
    counter: Option<u64>,
    /// `TOTP Seed`.
    seed: Option<Buf>,
    /// `TOTP Settings`.
    settings: Option<String>,
    /// An attribute had a value no scheme defines.
    invalid: bool,
}

impl KeepassOtp {
    /// Takes the attribute `key` if it belongs to an OTP scheme. Returns
    /// `false` (the caller keeps the attribute) for other keys. A value that
    /// does not decode is taken anyway and makes [`KeepassOtp::finish`]
    /// fail, so the caller can keep the raw values instead.
    pub(crate) fn take(&mut self, key: &str, value: &str) -> bool {
        let (prefix, hotp) = if let Some(rest) = key.strip_prefix("TimeOtp-") {
            (rest, false)
        } else if let Some(rest) = key.strip_prefix("HmacOtp-") {
            (rest, true)
        } else if key == "TOTP Seed" {
            self.seed = Some(decode_base32(value).unwrap_or_default());
            return true;
        } else if key == "TOTP Settings" {
            self.settings = Some(value.trim().to_owned());
            return true;
        } else {
            return false;
        };
        let v = value.trim();
        match prefix {
            "Secret" => self.set_secret(hotp, copy(v.as_bytes())),
            "Secret-Hex" => self.set_secret(hotp, decode_hex(v)),
            "Secret-Base32" => self.set_secret(hotp, decode_base32(v)),
            "Secret-Base64" => self.set_secret(hotp, buf::base64_decode(v).ok()),
            "Length" if !hotp => self.length = Some(v.parse().unwrap_or(0)),
            "Period" if !hotp => self.period = Some(v.parse().unwrap_or(0)),
            "Counter" if hotp => self.counter = Some(v.parse().unwrap_or(u64::MAX)),
            "Algorithm" if !hotp => {
                self.algorithm = match v {
                    "HMAC-SHA-1" | "" => Some(OtpAlgorithm::Sha1),
                    "HMAC-SHA-256" => Some(OtpAlgorithm::Sha256),
                    "HMAC-SHA-512" => Some(OtpAlgorithm::Sha512),
                    _ => {
                        self.invalid = true;
                        None
                    }
                };
            }
            _ => return false,
        }
        true
    }

    fn set_secret(&mut self, hotp: bool, secret: Option<Buf>) {
        self.hotp = hotp;
        // An undecodable secret leaves an empty one, which fails validation.
        self.secret = Some(secret.unwrap_or_default());
    }

    /// Whether any OTP attribute was taken.
    pub(crate) fn is_present(&self) -> bool {
        self.invalid
            || self.secret.is_some()
            || self.seed.is_some()
            || self.settings.is_some()
            || self.length.is_some()
            || self.period.is_some()
            || self.algorithm.is_some()
            || self.counter.is_some()
    }

    /// The configuration the attributes describe; `None` if they are
    /// incomplete or out of range.
    pub(crate) fn finish(&self) -> Option<Parsed> {
        if self.invalid {
            return None;
        }
        if let Some(secret) = &self.secret {
            if self.hotp {
                let mut c = config(secret)?;
                c.kind = OtpKind::Hotp;
                return checked(c, self.counter.unwrap_or(0));
            }
            return totp(
                secret,
                self.algorithm.unwrap_or_default(),
                self.length.unwrap_or(6),
                self.period.unwrap_or(30),
            );
        }
        let seed = self.seed.as_ref()?;
        let settings = self.settings.as_deref().unwrap_or("30;6");
        let mut parts = settings.split(';');
        let period: u32 = parts.next()?.trim().parse().ok()?;
        match parts.next().map(str::trim) {
            Some("S") => {
                let mut c = config(seed)?;
                c.kind = OtpKind::Steam;
                c.digits = 5;
                c.period = period;
                checked(c, 0)
            }
            Some(d) => totp(seed, OtpAlgorithm::Sha1, d.parse().ok()?, period),
            None => totp(seed, OtpAlgorithm::Sha1, 6, period),
        }
    }
}

fn copy(bytes: &[u8]) -> Option<Buf> {
    let mut b = buf::with_capacity(bytes.len()).ok()?;
    b.extend_from_slice(bytes);
    Some(b)
}

fn decode_base32(v: &str) -> Option<Buf> {
    copy(OtpSecret::from_base32(v).ok()?.expose_secret())
}

fn decode_hex(v: &str) -> Option<Buf> {
    let mut digits = buf::with_capacity(v.len()).ok()?;
    digits.extend(v.bytes().filter(|b| !b.is_ascii_whitespace()));
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    let mut out = buf::with_capacity(digits.len() / 2).ok()?;
    for pair in digits.as_chunks::<2>().0 {
        let s = core::str::from_utf8(pair).ok()?;
        out.push(u8::from_str_radix(s, 16).ok()?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::*;

    const B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
    const RAW: &[u8] = b"12345678901234567890";

    #[test]
    fn uri_steam_and_base32() {
        let (c, n) = parse(&format!(
            "otpauth://totp/Ex:alice?secret={B32}&issuer=Ex&digits=8&algorithm=SHA256&period=60"
        ))
        .unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);
        assert_eq!((c.digits, c.period, n), (8, 60, 0));
        assert_eq!(c.algorithm, OtpAlgorithm::Sha256);
        assert_eq!(c.issuer.unwrap().as_str(), "Ex");
        let (c, n) = parse(&format!("otpauth://hotp/x?secret={B32}&counter=7")).unwrap();
        assert_eq!((c.kind, n), (OtpKind::Hotp, 7));
        let (c, _) = parse(&format!("  OTPAUTH://totp/x?secret={B32} ")).unwrap();
        assert_eq!(c.kind, OtpKind::Totp);
        let (c, _) = parse(&format!("steam://{B32}")).unwrap();
        assert_eq!((c.kind, c.digits), (OtpKind::Steam, 5));
        let (c, _) = parse("gezd gnbv gy3t qojq gezd gnbv gy3t qojq").unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);
        assert_eq!((c.kind, c.digits, c.period), (OtpKind::Totp, 6, 30));
        for bad in [
            "",
            "otpauth://totp/x",
            "steam://!!",
            "not base32!",
            "otpauth://totp/x?secret=GE&digits=12",
        ] {
            assert!(parse(bad).is_none(), "{bad}");
        }
    }

    fn kp(attrs: &[(&str, &str)]) -> KeepassOtp {
        let mut k = KeepassOtp::default();
        for (key, value) in attrs {
            assert!(k.take(key, value), "{key}");
        }
        k
    }

    #[test]
    fn keepass_time_otp() {
        assert!(!KeepassOtp::default().is_present());
        assert!(KeepassOtp::default().finish().is_none());
        let mut other = KeepassOtp::default();
        assert!(!other.take("Title", "x"));
        assert!(!other.take("TimeOtp-Unknown", "x"));
        assert!(!other.take("HmacOtp-Length", "8"));

        let k = kp(&[
            ("TimeOtp-Secret-Base32", B32),
            ("TimeOtp-Length", "8"),
            ("TimeOtp-Period", "60"),
            ("TimeOtp-Algorithm", "HMAC-SHA-512"),
        ]);
        assert!(k.is_present());
        let (c, _) = k.finish().unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);
        assert_eq!((c.digits, c.period), (8, 60));
        assert_eq!(c.algorithm, OtpAlgorithm::Sha512);

        let (c, _) = kp(&[("TimeOtp-Secret", "12345678901234567890")])
            .finish()
            .unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);
        let (c, _) = kp(&[
            (
                "TimeOtp-Secret-Hex",
                "3132333435363738393031323334353637383930",
            ),
            ("TimeOtp-Algorithm", "HMAC-SHA-256"),
        ])
        .finish()
        .unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);
        assert_eq!(c.algorithm, OtpAlgorithm::Sha256);
        let (c, _) = kp(&[
            ("TimeOtp-Secret-Base64", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTA="),
            ("TimeOtp-Algorithm", "HMAC-SHA-1"),
        ])
        .finish()
        .unwrap();
        assert_eq!(c.secret.expose_secret(), RAW);

        for bad in [
            &[("TimeOtp-Secret-Hex", "123")][..],
            &[("TimeOtp-Secret-Hex", "zz")],
            &[("TimeOtp-Secret-Base32", "!!")],
            &[("TimeOtp-Secret-Base64", "*")],
            &[("TimeOtp-Secret", B32), ("TimeOtp-Length", "x")],
            &[("TimeOtp-Secret", B32), ("TimeOtp-Period", "0")],
            &[("TimeOtp-Secret", B32), ("TimeOtp-Algorithm", "MD5")],
            &[("TimeOtp-Length", "6")],
        ] {
            assert!(kp(bad).finish().is_none());
        }
    }

    #[test]
    fn keepass_hmac_otp() {
        let (c, n) = kp(&[("HmacOtp-Secret-Base32", B32), ("HmacOtp-Counter", "5")])
            .finish()
            .unwrap();
        assert_eq!((c.kind, n), (OtpKind::Hotp, 5));
        let (_, n) = kp(&[("HmacOtp-Secret", "abcdefghij")]).finish().unwrap();
        assert_eq!(n, 0);
        assert!(
            kp(&[("HmacOtp-Secret", "abc"), ("HmacOtp-Counter", "x")])
                .finish()
                .is_some_and(|(_, n)| n == u64::MAX)
        );
    }

    #[test]
    fn keepass_tray_totp() {
        let (c, _) = kp(&[("TOTP Seed", B32), ("TOTP Settings", "30;8")])
            .finish()
            .unwrap();
        assert_eq!((c.digits, c.period), (8, 30));
        let (c, _) = kp(&[("TOTP Seed", B32), ("TOTP Settings", "30;S")])
            .finish()
            .unwrap();
        assert_eq!(c.kind, OtpKind::Steam);
        let (c, _) = kp(&[("TOTP Seed", B32), ("TOTP Settings", "45")])
            .finish()
            .unwrap();
        assert_eq!(c.period, 45);
        let (c, _) = kp(&[("TOTP Seed", B32)]).finish().unwrap();
        assert_eq!((c.digits, c.period), (6, 30));
        for bad in [
            &[("TOTP Seed", "!!")][..],
            &[("TOTP Seed", B32), ("TOTP Settings", "x;6")],
            &[("TOTP Seed", B32), ("TOTP Settings", "30;x")],
            &[("TOTP Settings", "30;6")],
        ] {
            let k = kp(bad);
            assert!(k.is_present());
            assert!(k.finish().is_none());
        }
    }
}
