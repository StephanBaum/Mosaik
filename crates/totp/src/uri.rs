//! `otpauth://` URIs (Google Authenticator Key Uri Format), parse and format.
//!
//! ```text
//! otpauth://TYPE/LABEL?PARAMETERS
//! TYPE   = totp | hotp | steam                      (ASCII case ignored)
//! LABEL  = [ISSUER ":"] ACCOUNT                     (percent-encoded)
//! PARAMS = secret=B32 [&issuer=..] [&algorithm=SHA1|SHA256|SHA512]
//!          [&digits=6|7|8] [&period=1..300] [&counter=u64] [&encoder=steam]
//! ```
//!
//! # Parsing rules
//!
//! - Scheme and type ignore ASCII case. A fragment (`#`) is rejected.
//! - Label and values are percent-decoded and must be UTF-8. In parameter
//!   values `+` means a space (form encoding); in the label it is literal.
//! - Parameter names ignore ASCII case. A known parameter given twice is an
//!   error; unknown parameters (`image`, `color`, ...) are ignored; empty
//!   segments (`&&`) are skipped; a segment without `=` is an error.
//! - `secret` is required and decoded as lenient base32
//!   ([`OtpSecret::from_base32`]). `counter` is required for HOTP. A
//!   parameter that does not apply to the type (`counter` on TOTP, `period`
//!   on HOTP) is ignored.
//! - Issuer: the `issuer` parameter wins. If the label starts with that
//!   issuer followed by `:`, the prefix is removed; otherwise the label is
//!   split at its first `:` and any prefix is dropped. Without the
//!   parameter the label prefix before the first `:` is the issuer. Issuer
//!   and account name are trimmed; an empty issuer is `None`. Without an
//!   issuer, an account name containing `:` (left over after a blank
//!   prefix, as in `%20:a:b`) is rejected, because it could not be
//!   formatted back unambiguously.
//! - Issuer and account name are at most [`MAX_TEXT_LEN`] bytes and must
//!   not contain control characters or bidirectional formatting characters
//!   (which could make a label display as something else).
//!
//! # Steam Guard
//!
//! A URI is Steam Guard if any of these holds:
//! 1. the type is `steam` (Aegis convention),
//! 2. it has `encoder=steam` (`KeePassXC` convention),
//! 3. the type is `totp`, there is no `digits` parameter, and the issuer is
//!    `Steam` in any ASCII case (the `uri` field Steam's mobile app writes
//!    into its `.maFile`, `otpauth://totp/Steam:NAME?secret=..&issuer=Steam`).
//!    Steam offers no plain TOTP, so this cannot misread a real account; an
//!    explicit `digits` parameter keeps the URI plain TOTP.
//!
//! A Steam URI may carry `digits=5` and nothing else for `digits`.
//! `encoder=steam` with HOTP and any other `encoder` value are errors.
//!
//! # Formatting
//!
//! [`OtpAuth::to_uri`] writes `otpauth://totp|hotp/ISSUER:ACCOUNT?secret=..`
//! with canonical base32 (no padding), then `issuer`, `algorithm`, and
//! `digits`+`period`, `digits`+`counter`, or `period`+`encoder=steam`.
//! Everything but RFC 3986 unreserved characters is percent-encoded. Output
//! parses back to an equal value.

use core::str::FromStr;

use zeroize::Zeroize;

use crate::base32;
use crate::error::{Error, Result};
use crate::otp::{Algorithm, Digits, Otp, OtpKind, Period};
use crate::secret::{OtpSecret, SecretText};

/// Longest accepted URI, in bytes (QR codes hold at most 2953).
pub const MAX_URI_LEN: usize = 4096;

/// Longest issuer or account name, in bytes after percent-decoding.
pub const MAX_TEXT_LEN: usize = 256;

/// Most query parameters (known and unknown) in one URI.
pub const MAX_PARAMS: usize = 32;

const SCHEME: &[u8] = b"otpauth://";

/// A parsed `otpauth://` URI: the OTP configuration plus its label.
#[derive(Debug, PartialEq, Eq)]
pub struct OtpAuth {
    /// Secret, algorithm and kind.
    pub otp: Otp,
    /// The service, if known.
    pub issuer: Option<String>,
    /// The account name; may be empty.
    pub account_name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UriType {
    Totp,
    Hotp,
    Steam,
}

#[derive(Default)]
struct RawParams<'a> {
    secret: Option<&'a [u8]>,
    issuer: Option<&'a [u8]>,
    algorithm: Option<&'a [u8]>,
    digits: Option<&'a [u8]>,
    period: Option<&'a [u8]>,
    counter: Option<&'a [u8]>,
    encoder: Option<&'a [u8]>,
}

impl<'a> RawParams<'a> {
    fn parse(query: &'a [u8]) -> Result<Self> {
        let mut p = Self::default();
        let mut count = 0usize;
        for seg in query.split(|&c| c == b'&').filter(|s| !s.is_empty()) {
            count += 1;
            if count > MAX_PARAMS {
                return Err(Error::SizeLimit);
            }
            let eq = seg
                .iter()
                .position(|&c| c == b'=')
                .ok_or(Error::MalformedUri)?;
            let (key, value) = seg.split_at(eq);
            let value = value.get(1..).unwrap_or_default();
            let slot = match key.to_ascii_lowercase().as_slice() {
                b"secret" => &mut p.secret,
                b"issuer" => &mut p.issuer,
                b"algorithm" => &mut p.algorithm,
                b"digits" => &mut p.digits,
                b"period" => &mut p.period,
                b"counter" => &mut p.counter,
                b"encoder" => &mut p.encoder,
                _ => continue,
            };
            if slot.replace(value).is_some() {
                return Err(Error::DuplicateParameter);
            }
        }
        Ok(p)
    }
}

fn hex_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Percent-decodes `raw`. The buffer is reserved at its final bound and
/// zeroized on error, because it may hold the secret.
fn percent_decode(raw: &[u8], plus_is_space: bool) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(raw.len());
    let mut it = raw.iter().copied();
    while let Some(c) = it.next() {
        let b = match c {
            b'%' => {
                let hi = it.next().and_then(hex_value);
                let lo = it.next().and_then(hex_value);
                let Some((h, l)) = hi.zip(lo) else {
                    out.zeroize();
                    return Err(Error::MalformedUri);
                };
                (h << 4) | l
            }
            b'+' if plus_is_space => b' ',
            other => other,
        };
        out.push(b);
    }
    Ok(out)
}

fn decode_text(raw: &[u8], plus_is_space: bool) -> Result<String> {
    String::from_utf8(percent_decode(raw, plus_is_space)?).map_err(|_| Error::MalformedUri)
}

fn decode_number(raw: &[u8]) -> Result<u64> {
    let s = decode_text(raw, true)?;
    if s.is_empty() || s.len() > 20 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidParameter);
    }
    s.parse::<u64>().map_err(|_| Error::InvalidParameter)
}

/// Characters that may not appear in an issuer or account name: controls,
/// and the bidirectional formatting characters (Unicode `Bidi_Control`).
fn is_forbidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        )
}

fn validate_text(s: &str) -> Result<()> {
    if s.len() > MAX_TEXT_LEN || s.trim() != s || s.chars().any(is_forbidden) {
        return Err(Error::InvalidLabel);
    }
    Ok(())
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

/// Splits the decoded label into issuer and account (see the module docs).
fn split_label(label: &str, issuer_param: Option<String>) -> (Option<String>, String) {
    match issuer_param {
        Some(p) => {
            let account = label
                .strip_prefix(p.as_str())
                .and_then(|r| r.strip_prefix(':'))
                .or_else(|| label.split_once(':').map(|(_, a)| a))
                .unwrap_or(label);
            (Some(p), account.trim().to_owned())
        }
        None => match label.split_once(':') {
            Some((prefix, account)) => (non_empty(prefix), account.trim().to_owned()),
            None => (None, label.trim().to_owned()),
        },
    }
}

fn parse_type(ty: &[u8]) -> Result<UriType> {
    match ty.to_ascii_lowercase().as_slice() {
        b"totp" => Ok(UriType::Totp),
        b"hotp" => Ok(UriType::Hotp),
        b"steam" => Ok(UriType::Steam),
        _ => Err(Error::UnsupportedType),
    }
}

fn parse_digits(raw: Option<&[u8]>) -> Result<Digits> {
    match raw {
        None => Ok(Digits::DEFAULT),
        Some(v) => {
            let n = decode_number(v)?;
            Digits::new(u8::try_from(n).map_err(|_| Error::InvalidDigits)?)
        }
    }
}

fn parse_period(raw: Option<&[u8]>) -> Result<Period> {
    match raw {
        None => Ok(Period::DEFAULT),
        Some(v) => {
            let n = decode_number(v)?;
            Period::new(u32::try_from(n).map_err(|_| Error::InvalidPeriod)?)
        }
    }
}

fn parse_secret(raw: &[u8]) -> Result<OtpSecret> {
    let mut text = percent_decode(raw, true)?;
    let secret = OtpSecret::from_base32_bytes(&text);
    text.zeroize();
    secret
}

fn parse(uri: &str) -> Result<OtpAuth> {
    let bytes = uri.as_bytes();
    if bytes.len() > MAX_URI_LEN {
        return Err(Error::SizeLimit);
    }
    let (scheme, rest) = bytes
        .split_at_checked(SCHEME.len())
        .ok_or(Error::MalformedUri)?;
    if !scheme.eq_ignore_ascii_case(SCHEME) || rest.contains(&b'#') {
        return Err(Error::MalformedUri);
    }
    let slash = rest
        .iter()
        .position(|&c| c == b'/')
        .ok_or(Error::MalformedUri)?;
    let (ty, path) = rest.split_at(slash);
    let ty = parse_type(ty)?;
    let path = path.get(1..).unwrap_or_default();
    let q = path
        .iter()
        .position(|&c| c == b'?')
        .ok_or(Error::MissingSecret)?;
    let (label_raw, query) = path.split_at(q);
    let query = query.get(1..).unwrap_or_default();

    let params = RawParams::parse(query)?;
    let secret = parse_secret(params.secret.ok_or(Error::MissingSecret)?)?;

    let label = decode_text(label_raw, false)?;
    let issuer_param = match params.issuer {
        Some(v) => non_empty(&decode_text(v, true)?),
        None => None,
    };
    let (issuer, account_name) = split_label(&label, issuer_param);
    if let Some(i) = &issuer {
        validate_text(i)?;
    }
    validate_text(&account_name)?;
    // Without an issuer a `:` in the account name would read as an issuer
    // separator when formatted (possible only after a blank label prefix).
    if issuer.is_none() && account_name.contains(':') {
        return Err(Error::InvalidLabel);
    }

    let algorithm = match params.algorithm {
        Some(v) => Algorithm::parse(&decode_text(v, true)?)?,
        None => Algorithm::default(),
    };
    let encoder_steam = match params.encoder {
        Some(v) if decode_text(v, true)?.eq_ignore_ascii_case("steam") => true,
        Some(_) => return Err(Error::UnsupportedEncoder),
        None => false,
    };

    let issuer_is_steam = issuer
        .as_deref()
        .is_some_and(|i| i.eq_ignore_ascii_case("steam"));
    let kind = match ty {
        UriType::Hotp if encoder_steam => return Err(Error::UnsupportedEncoder),
        UriType::Hotp => OtpKind::Hotp {
            digits: parse_digits(params.digits)?,
            counter: decode_number(params.counter.ok_or(Error::MissingCounter)?)?,
        },
        UriType::Steam | UriType::Totp
            if ty == UriType::Steam
                || encoder_steam
                || (issuer_is_steam && params.digits.is_none()) =>
        {
            if let Some(v) = params.digits
                && decode_number(v)? != 5
            {
                return Err(Error::InvalidDigits);
            }
            OtpKind::Steam {
                period: parse_period(params.period)?,
            }
        }
        UriType::Steam | UriType::Totp => OtpKind::Totp {
            digits: parse_digits(params.digits)?,
            period: parse_period(params.period)?,
        },
    };

    Ok(OtpAuth {
        otp: Otp {
            secret,
            algorithm,
            kind,
        },
        issuer,
        account_name,
    })
}

fn is_unreserved(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~')
}

fn percent_encode(s: &str, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &c in s.as_bytes() {
        if is_unreserved(c) {
            out.push(char::from(c));
        } else {
            out.push('%');
            for nibble in [c >> 4, c & 0x0F] {
                out.push(char::from(
                    HEX.get(usize::from(nibble)).copied().unwrap_or(b'0'),
                ));
            }
        }
    }
}

fn push_param(out: &mut String, name: &str, value: u64) {
    out.push('&');
    out.push_str(name);
    out.push('=');
    out.push_str(&value.to_string());
}

impl OtpAuth {
    /// Parses an `otpauth://` URI (see the module documentation for the
    /// exact rules).
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_URI_LEN`] or [`MAX_PARAMS`];
    /// [`Error::MalformedUri`], [`Error::UnsupportedType`],
    /// [`Error::MissingSecret`], [`Error::MissingCounter`],
    /// [`Error::DuplicateParameter`], [`Error::InvalidParameter`],
    /// [`Error::InvalidDigits`], [`Error::InvalidPeriod`],
    /// [`Error::UnsupportedAlgorithm`], [`Error::UnsupportedEncoder`],
    /// [`Error::InvalidLabel`] or [`Error::InvalidSecret`] for the
    /// corresponding defect.
    pub fn parse(uri: &str) -> Result<Self> {
        parse(uri)
    }

    /// Formats the entry as an `otpauth://` URI. The result contains the
    /// secret.
    ///
    /// # Errors
    /// [`Error::InvalidLabel`] if the issuer or account name would not
    /// parse back unchanged: longer than [`MAX_TEXT_LEN`], forbidden
    /// characters, surrounding whitespace, an empty issuer, or a `:` in the
    /// account name when there is no issuer.
    pub fn to_uri(&self) -> Result<SecretText> {
        if let Some(i) = &self.issuer {
            if i.is_empty() {
                return Err(Error::InvalidLabel);
            }
            validate_text(i)?;
        }
        validate_text(&self.account_name)?;
        if self.issuer.is_none() && self.account_name.contains(':') {
            return Err(Error::InvalidLabel);
        }

        let mut head = String::from("otpauth://");
        head.push_str(match self.otp.kind {
            OtpKind::Hotp { .. } => "hotp/",
            OtpKind::Totp { .. } | OtpKind::Steam { .. } => "totp/",
        });
        if let Some(i) = &self.issuer {
            percent_encode(i, &mut head);
            head.push(':');
        }
        percent_encode(&self.account_name, &mut head);
        head.push_str("?secret=");

        let mut tail = String::new();
        if let Some(i) = &self.issuer {
            tail.push_str("&issuer=");
            percent_encode(i, &mut tail);
        }
        tail.push_str("&algorithm=");
        tail.push_str(self.otp.algorithm.as_str());
        match self.otp.kind {
            OtpKind::Totp { digits, period } => {
                push_param(&mut tail, "digits", u64::from(digits.get()));
                push_param(&mut tail, "period", u64::from(period.get()));
            }
            OtpKind::Hotp { digits, counter } => {
                push_param(&mut tail, "digits", u64::from(digits.get()));
                push_param(&mut tail, "counter", counter);
            }
            OtpKind::Steam { period } => {
                push_param(&mut tail, "period", u64::from(period.get()));
                tail.push_str("&encoder=steam");
            }
        }

        let secret = self.otp.secret.expose_secret();
        // Exact size up front: the secret is never reallocated.
        let mut out =
            String::with_capacity(head.len() + base32::encoded_len(secret.len()) + tail.len());
        out.push_str(&head);
        base32::encode_into(secret, &mut out);
        out.push_str(&tail);
        Ok(SecretText(out))
    }
}

impl FromStr for OtpAuth {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::STEAM_ALPHABET;
    use proptest::prelude::*;

    const RFC_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    fn p(s: &str) -> Result<OtpAuth> {
        OtpAuth::parse(s)
    }

    fn ok(s: &str) -> OtpAuth {
        p(s).unwrap_or_else(|e| panic!("{s}: {e:?}"))
    }

    fn err(s: &str) -> Error {
        match p(s) {
            Ok(_) => panic!("{s} parsed"),
            Err(e) => e,
        }
    }

    #[test]
    fn google_example() {
        let a =
            ok("otpauth://totp/Example:alice@google.com?secret=JBSWY3DPEHPK3PXP&issuer=Example");
        assert_eq!(a.issuer.as_deref(), Some("Example"));
        assert_eq!(a.account_name, "alice@google.com");
        assert_eq!(a.otp.algorithm, Algorithm::Sha1);
        assert_eq!(a.otp.kind, OtpKind::DEFAULT_TOTP);
        assert_eq!(a.otp.secret.expose_secret(), b"Hello!\xDE\xAD\xBE\xEF");
        assert_eq!(
            a.to_uri().unwrap().expose_secret(),
            "otpauth://totp/Example:alice%40google.com?secret=JBSWY3DPEHPK3PXP\
             &issuer=Example&algorithm=SHA1&digits=6&period=30"
        );
    }

    #[test]
    fn full_parameters_and_codes() {
        let a = ok(&format!(
            "otpauth://totp/ACME%20Co:john.doe%40email.com?secret={RFC_B32}\
             &issuer=ACME%20Co&algorithm=SHA512&digits=8&period=60"
        ));
        assert_eq!(a.issuer.as_deref(), Some("ACME Co"));
        assert_eq!(a.account_name, "john.doe@email.com");
        assert_eq!(a.otp.algorithm, Algorithm::Sha512);
        assert_eq!(
            a.otp.kind,
            OtpKind::Totp {
                digits: Digits::new(8).unwrap(),
                period: Period::new(60).unwrap()
            }
        );
        let h = ok(&format!("otpauth://hotp/x?secret={RFC_B32}&counter=7"));
        assert_eq!(
            h.otp.kind,
            OtpKind::Hotp {
                digits: Digits::DEFAULT,
                counter: 7
            }
        );
        assert_eq!(h.otp.code_at(0).unwrap().expose_secret(), "162583");
        let max = ok(&format!(
            "otpauth://hotp/x?secret={RFC_B32}&counter=18446744073709551615&digits=7"
        ));
        assert_eq!(
            max.otp.kind,
            OtpKind::Hotp {
                digits: Digits::new(7).unwrap(),
                counter: u64::MAX
            }
        );
        assert_eq!(
            "otpauth://totp/a?secret=MFRGG"
                .parse::<OtpAuth>()
                .unwrap()
                .account_name,
            "a"
        );
    }

    #[test]
    fn leniency() {
        // Case of scheme, type, parameter names and algorithm; lower-case,
        // spaced and padded secret; `+` in values; empty segments; unknown
        // parameters; a space after the label colon.
        let big = ok(
            "OTPAUTH://TOTP/Big%20Corp:%20bob?SECRET=gezd+gnbv%20gy3tqojq\
                    gezdgnbvgy3tqojq%3D%3D&&Issuer=Big+Corp&algorithm=sha256\
                    &image=https%3A%2F%2Fexample.com%2Fx.png&color=ff0000&",
        );
        assert_eq!(big.issuer.as_deref(), Some("Big Corp"));
        assert_eq!(big.account_name, "bob");
        assert_eq!(big.otp.algorithm, Algorithm::Sha256);
        assert_eq!(big.otp.secret.expose_secret(), b"12345678901234567890");
        // Irrelevant parameters are ignored.
        let ignored = ok("otpauth://totp/a?secret=MFRGG&counter=zzz");
        assert_eq!(ignored.otp.kind, OtpKind::DEFAULT_TOTP);
        let counter_only = ok("otpauth://hotp/a?secret=MFRGG&counter=0&period=zzz");
        assert!(matches!(
            counter_only.otp.kind,
            OtpKind::Hotp { counter: 0, .. }
        ));
        // `+` is literal in the label, `%3A` separates like `:`.
        let plus = ok("otpauth://totp/a+b%3Ac?secret=MFRGG");
        assert_eq!(plus.issuer.as_deref(), Some("a+b"));
        assert_eq!(plus.account_name, "c");
        // Raw UTF-8 in the label.
        let utf8 = ok("otpauth://totp/B\u{fc}ro:j\u{f6}rg?secret=MFRGG");
        assert_eq!(utf8.issuer.as_deref(), Some("B\u{fc}ro"));
        assert_eq!(utf8.account_name, "j\u{f6}rg");
        // Empty label and empty issuer parameter.
        let empty = ok("otpauth://totp/?secret=MFRGG&issuer=");
        assert_eq!(empty.issuer, None);
        assert_eq!(empty.account_name, "");
        let blank_prefix = ok("otpauth://totp/%20:acct?secret=MFRGG");
        assert_eq!(blank_prefix.issuer, None);
        assert_eq!(blank_prefix.account_name, "acct");
        // Regression (fuzzing): a blank prefix leaves no issuer, so an
        // account name with a further `:` could not be formatted back.
        assert_eq!(
            err("otpauth://totp/%20:a:b?secret=MFRGG"),
            Error::InvalidLabel
        );
        assert_eq!(
            err("otpauth://totp/:a:b?secret=MFRGG&issuer=+"),
            Error::InvalidLabel
        );
    }

    #[test]
    fn issuer_resolution() {
        // Parameter wins over a differing label prefix.
        let a = ok("otpauth://totp/Old:me?secret=MFRGG&issuer=New");
        assert_eq!(
            (a.issuer.as_deref(), a.account_name.as_str()),
            (Some("New"), "me")
        );
        // Parameter without a label prefix.
        let b = ok("otpauth://totp/me?secret=MFRGG&issuer=New");
        assert_eq!(
            (b.issuer.as_deref(), b.account_name.as_str()),
            (Some("New"), "me")
        );
        // Issuer containing a colon is stripped as a whole.
        let c = ok("otpauth://totp/a%3Ab:c%3Ad?secret=MFRGG&issuer=a%3Ab");
        assert_eq!(
            (c.issuer.as_deref(), c.account_name.as_str()),
            (Some("a:b"), "c:d")
        );
        // Label prefix only.
        let d = ok("otpauth://totp/Pre:me:too?secret=MFRGG");
        assert_eq!(
            (d.issuer.as_deref(), d.account_name.as_str()),
            (Some("Pre"), "me:too")
        );
    }

    #[test]
    fn steam_conventions() {
        let steam_secret = "Z3ZBVSU5ZE7PW3276QDB5KZKFAMT7Y4L";
        for uri in [
            format!("otpauth://totp/Steam:gaben?secret={steam_secret}&issuer=Steam"),
            format!("otpauth://totp/gaben?secret={steam_secret}&issuer=steam"),
            format!("otpauth://totp/Steam:gaben?secret={steam_secret}"),
            format!("otpauth://totp/x?secret={steam_secret}&encoder=steam"),
            format!("otpauth://totp/x?secret={steam_secret}&encoder=Steam&digits=5"),
            format!("otpauth://steam/Steam:gaben?secret={steam_secret}&digits=5&period=30"),
            format!("otpauth://STEAM/x?secret={steam_secret}"),
        ] {
            let a = ok(&uri);
            assert_eq!(a.otp.kind, OtpKind::STEAM, "{uri}");
            assert_eq!(
                a.otp.code_at(1_616_374_841).unwrap().expose_secret(),
                "2F9J5"
            );
            let back = ok(a.to_uri().unwrap().expose_secret());
            assert_eq!(back, a);
        }
        // Explicit digits keep a Steam-issued URI plain TOTP.
        let t = ok(&format!(
            "otpauth://totp/Steam:x?secret={steam_secret}&digits=6"
        ));
        assert_eq!(t.otp.kind, OtpKind::DEFAULT_TOTP);
        let back = ok(t.to_uri().unwrap().expose_secret());
        assert_eq!(back.otp.kind, OtpKind::DEFAULT_TOTP);
        // Steam with other digits, steam HOTP, other encoders.
        assert_eq!(
            err(&format!(
                "otpauth://totp/x?secret={steam_secret}&encoder=steam&digits=6"
            )),
            Error::InvalidDigits
        );
        assert_eq!(
            err(&format!("otpauth://steam/x?secret={steam_secret}&digits=x")),
            Error::InvalidParameter
        );
        assert_eq!(
            err(&format!(
                "otpauth://hotp/x?secret={steam_secret}&counter=1&encoder=steam"
            )),
            Error::UnsupportedEncoder
        );
        assert_eq!(
            err(&format!(
                "otpauth://totp/x?secret={steam_secret}&encoder=base64"
            )),
            Error::UnsupportedEncoder
        );
        assert_eq!(
            err(&format!("otpauth://totp/x?secret={steam_secret}&encoder=")),
            Error::UnsupportedEncoder
        );
        let s = ok(&format!(
            "otpauth://steam/x?secret={steam_secret}&period=60&algorithm=SHA256"
        ));
        assert_eq!(
            s.otp.kind,
            OtpKind::Steam {
                period: Period::new(60).unwrap()
            }
        );
        let code = s.otp.code_at(0).unwrap();
        assert!(
            code.expose_secret()
                .bytes()
                .all(|c| STEAM_ALPHABET.contains(&c))
        );
    }

    const REJECTIONS: &[(&str, Error)] = &[
        ("", Error::MalformedUri),
        ("otpauth:/", Error::MalformedUri),
        ("http://totp/a?secret=MFRGG", Error::MalformedUri),
        ("otpauth:totp/a?secret=MFRGG", Error::MalformedUri),
        ("otpauth://totp?secret=MFRGG", Error::MalformedUri),
        ("otpauth://totp/a?secret=MFRGG#frag", Error::MalformedUri),
        ("otpauth://totp/a?secret", Error::MalformedUri),
        ("otpauth://totp/a%2?secret=MFRGG", Error::MalformedUri),
        ("otpauth://totp/a%zz?secret=MFRGG", Error::MalformedUri),
        ("otpauth://totp/a%FF?secret=MFRGG", Error::MalformedUri),
        (
            "otpauth://totp/a?secret=MFRGG&issuer=%C3",
            Error::MalformedUri,
        ),
        ("otpauth://totp/a?secret=MF%4", Error::MalformedUri),
        ("otpauth://motp/a?secret=MFRGG", Error::UnsupportedType),
        ("otpauth:///a?secret=MFRGG", Error::UnsupportedType),
        ("otpauth://totp/a", Error::MissingSecret),
        ("otpauth://totp/a?issuer=x", Error::MissingSecret),
        ("otpauth://totp/a?secret=", Error::InvalidSecret),
        ("otpauth://totp/a?secret=MFRGG1", Error::InvalidSecret),
        ("otpauth://totp/a?secret=MF%3DRGG", Error::InvalidSecret),
        ("otpauth://hotp/a?secret=MFRGG", Error::MissingCounter),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=",
            Error::InvalidParameter,
        ),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=-1",
            Error::InvalidParameter,
        ),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=+1",
            Error::InvalidParameter,
        ),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=18446744073709551616",
            Error::InvalidParameter,
        ),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=000000000000000000001",
            Error::InvalidParameter,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&secret=MFRGG",
            Error::DuplicateParameter,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&ISSUER=a&issuer=b",
            Error::DuplicateParameter,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&digits=5",
            Error::InvalidDigits,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&digits=9",
            Error::InvalidDigits,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&digits=256",
            Error::InvalidDigits,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&digits=six",
            Error::InvalidParameter,
        ),
        (
            "otpauth://hotp/a?secret=MFRGG&counter=1&digits=4",
            Error::InvalidDigits,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&period=0",
            Error::InvalidPeriod,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&period=301",
            Error::InvalidPeriod,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&period=4294967296",
            Error::InvalidPeriod,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&period=1.5",
            Error::InvalidParameter,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&algorithm=MD5",
            Error::UnsupportedAlgorithm,
        ),
        (
            "otpauth://totp/a?secret=MFRGG&algorithm=%ZZ",
            Error::MalformedUri,
        ),
        ("otpauth://totp/a%0Ab?secret=MFRGG", Error::InvalidLabel),
        (
            "otpauth://totp/a?secret=MFRGG&issuer=%E2%80%AEevil",
            Error::InvalidLabel,
        ),
        (
            "otpauth://totp/x:a%E2%81%A6b?secret=MFRGG",
            Error::InvalidLabel,
        ),
        ("otpauth://totp/a%C2%85b?secret=MFRGG", Error::InvalidLabel),
    ];

    #[test]
    fn rejections() {
        for (uri, want) in REJECTIONS {
            assert_eq!(err(uri), *want, "{uri}");
        }
    }

    #[test]
    fn limits() {
        let long_label = "a".repeat(MAX_TEXT_LEN);
        assert!(p(&format!("otpauth://totp/{long_label}?secret=MFRGG")).is_ok());
        let too_long = "a".repeat(MAX_TEXT_LEN + 1);
        assert_eq!(
            err(&format!("otpauth://totp/{too_long}?secret=MFRGG")),
            Error::InvalidLabel
        );
        assert_eq!(
            err(&format!("otpauth://totp/x?secret=MFRGG&issuer={too_long}")),
            Error::InvalidLabel
        );
        let pad = "x".repeat(MAX_URI_LEN);
        assert_eq!(
            err(&format!("otpauth://totp/a?secret=MFRGG&u={pad}")),
            Error::SizeLimit
        );
        let at_limit = String::from("otpauth://totp/a?secret=MFRGG&u=");
        let fill = "x".repeat(MAX_URI_LEN - at_limit.len());
        assert!(p(&format!("{at_limit}{fill}")).is_ok());
        let many = "&u=1".repeat(MAX_PARAMS - 1);
        assert!(p(&format!("otpauth://totp/a?secret=MFRGG{many}")).is_ok());
        let too_many = "&u=1".repeat(MAX_PARAMS);
        assert_eq!(
            err(&format!("otpauth://totp/a?secret=MFRGG{too_many}")),
            Error::SizeLimit
        );
        let big_secret = "A".repeat(208);
        assert_eq!(
            err(&format!("otpauth://totp/a?secret={big_secret}")),
            Error::SizeLimit
        );
    }

    fn entry(issuer: Option<&str>, account: &str, kind: OtpKind) -> OtpAuth {
        OtpAuth {
            otp: Otp {
                secret: OtpSecret::from_bytes(b"12345678901234567890").unwrap(),
                algorithm: Algorithm::Sha256,
                kind,
            },
            issuer: issuer.map(str::to_owned),
            account_name: account.to_owned(),
        }
    }

    #[test]
    fn format_rejects_unrepresentable_labels() {
        let k = OtpKind::DEFAULT_TOTP;
        for (issuer, account) in [
            (Some(""), "a"),
            (Some(" x"), "a"),
            (Some("x"), "a "),
            (None, "a:b"),
            (None, "a\u{7}"),
            (Some("\u{202E}"), "a"),
        ] {
            assert_eq!(
                entry(issuer, account, k).to_uri().err(),
                Some(Error::InvalidLabel),
                "{issuer:?} {account:?}"
            );
        }
        assert!(entry(Some("x"), "a:b", k).to_uri().is_ok());
    }

    #[test]
    fn format_shapes() {
        let h = entry(
            Some("A&B Co"),
            "m e",
            OtpKind::Hotp {
                digits: Digits::new(8).unwrap(),
                counter: 42,
            },
        );
        let uri = h.to_uri().unwrap();
        assert_eq!(
            uri.expose_secret(),
            format!(
                "otpauth://hotp/A%26B%20Co:m%20e?secret={RFC_B32}&issuer=A%26B%20Co\
                 &algorithm=SHA256&digits=8&counter=42"
            )
        );
        assert_eq!(uri.capacity(), uri.expose_secret().len());
        assert_eq!(ok(uri.expose_secret()), h);
        let s = entry(None, "", OtpKind::STEAM);
        assert_eq!(
            s.to_uri().unwrap().expose_secret(),
            format!("otpauth://totp/?secret={RFC_B32}&algorithm=SHA256&period=30&encoder=steam")
        );
        assert_eq!(format!("{:?}", s.to_uri().unwrap()), "[REDACTED]");
    }

    /// The largest representable entry still fits [`MAX_URI_LEN`], so
    /// formatting never needs a size check to keep `parse(to_uri(x)) == x`.
    #[test]
    fn largest_entry_fits() {
        let worst = "\u{1F600}".repeat(MAX_TEXT_LEN / 4);
        let mut e = entry(
            Some(&worst),
            &worst,
            OtpKind::Hotp {
                digits: Digits::new(8).unwrap(),
                counter: u64::MAX,
            },
        );
        e.otp.secret = OtpSecret::from_bytes(&[0xFF; crate::MAX_SECRET_LEN]).unwrap();
        let uri = e.to_uri().unwrap();
        assert!(uri.expose_secret().len() <= MAX_URI_LEN);
        assert_eq!(ok(uri.expose_secret()), e);
    }

    fn text() -> impl Strategy<Value = String> {
        // Printable ASCII including every URI delimiter, some non-ASCII, and
        // inner spaces; trimmed afterwards.
        proptest::string::string_regex("[ -~\u{e4}\u{3b1}\u{4e2d}\u{1F600}]{0,40}")
            .unwrap()
            .prop_map(|s| s.trim().to_owned())
    }

    fn kind() -> impl Strategy<Value = OtpKind> {
        prop_oneof![
            (6u8..=8, 1u32..=300).prop_map(|(d, p)| OtpKind::Totp {
                digits: Digits::new(d).unwrap(),
                period: Period::new(p).unwrap()
            }),
            (6u8..=8, any::<u64>()).prop_map(|(d, c)| OtpKind::Hotp {
                digits: Digits::new(d).unwrap(),
                counter: c
            }),
            (1u32..=300).prop_map(|p| OtpKind::Steam {
                period: Period::new(p).unwrap()
            }),
        ]
    }

    fn algorithm() -> impl Strategy<Value = Algorithm> {
        prop_oneof![
            Just(Algorithm::Sha1),
            Just(Algorithm::Sha256),
            Just(Algorithm::Sha512)
        ]
    }

    proptest! {
        #[test]
        fn round_trip(
            secret in proptest::collection::vec(any::<u8>(), 1..=crate::MAX_SECRET_LEN),
            algorithm in algorithm(),
            kind in kind(),
            issuer in proptest::option::of(text()),
            account in text(),
        ) {
            let issuer = issuer.filter(|i| !i.is_empty());
            let account = if issuer.is_none() { account.replace(':', "") } else { account };
            let account = account.trim().to_owned();
            let e = OtpAuth {
                otp: Otp { secret: OtpSecret::from_bytes(&secret).unwrap(), algorithm, kind },
                issuer,
                account_name: account,
            };
            let uri = e.to_uri().unwrap();
            let back = OtpAuth::parse(uri.expose_secret()).unwrap();
            prop_assert_eq!(&back, &e);
            let again = back.to_uri().unwrap();
            prop_assert_eq!(again.expose_secret(), uri.expose_secret());
        }

        /// Arbitrary input never panics; whatever parses re-formats to a URI
        /// that parses to the same value (formatting is canonical).
        #[test]
        fn parse_arbitrary(s in "otpauth://(totp|hotp|steam|x)/[ -~]{0,30}\\?[ -~]{0,80}") {
            if let Ok(a) = OtpAuth::parse(&s) {
                let uri = a.to_uri().unwrap();
                let b = OtpAuth::parse(uri.expose_secret()).unwrap();
                prop_assert_eq!(&b, &a);
            }
        }

        #[test]
        fn parse_structured(
            ty in "(totp|hotp|steam|TOTP)",
            label in "[A-Za-z0-9 :%@.+]{0,20}",
            params in proptest::collection::vec(
                ("(secret|issuer|algorithm|digits|period|counter|encoder|image)",
                 "(MFRGG|gezd gnbv|SHA1|sha512|MD5|5|6|8|30|0|301|steam|Steam|%41|)"),
                0..8),
        ) {
            let q: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let s = format!("otpauth://{ty}/{label}?{}", q.join("&"));
            if let Ok(a) = OtpAuth::parse(&s) {
                let b = OtpAuth::parse(a.to_uri().unwrap().expose_secret()).unwrap();
                prop_assert_eq!(&b, &a);
            }
        }
    }
}
