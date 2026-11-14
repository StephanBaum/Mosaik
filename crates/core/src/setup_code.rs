//! The setup code: a desktop shows its account's server, email and Secret
//! Key as a QR code so a phone can join without typing (CRYPTO_SPEC §6.9.7,
//! ADR-0096).
//!
//! ```text
//! setup_code = "mosaic-setup:1:" || base64url_unpadded(JSON)
//! JSON       = {"v":1,"server":"<canonical origin>","email":"<normalized email>","sk":"<M1-...>"}
//! ```
//!
//! The JSON is canonical (these four members in this order, no whitespace)
//! and the code is at most [`MAX_SETUP_CODE_LEN`] bytes. The format uses no
//! label and no primitive. Both directions work in zeroizing buffers.
//!
//! The server origin and the email normalization (§9.2) belong to
//! `mosaic-sync`, which depends on this crate; [`parse`] therefore takes a
//! `canonical` function that returns the canonical forms, and requires the code
//! to carry exactly them. `mosaic_sync::setup_code::parse` supplies it.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use zeroize::Zeroizing;

use crate::{Error, Result, SecretKey};

/// The exact start of every setup code.
pub const SETUP_CODE_PREFIX: &str = "mosaic-setup:1:";

/// The longest setup code, in bytes.
pub const MAX_SETUP_CODE_LEN: usize = 1024;

const JSON_HEAD: &str = "{\"v\":1,\"server\":\"";
const JSON_EMAIL: &str = "\",\"email\":\"";
const JSON_SK: &str = "\",\"sk\":\"";
const JSON_TAIL: &str = "\"}";

/// What a setup code carries. Not `Clone`; `Debug` redacts the Secret Key.
#[derive(Debug)]
pub struct SetupCode {
    /// The canonical server origin.
    pub server: String,
    /// The normalized account email.
    pub email: String,
    /// The account's Secret Key.
    pub secret_key: SecretKey,
}

/// A string that can sit in a JSON member without escaping.
fn plain(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c == '"' || c == '\\' || c.is_control())
}

/// Builds the setup code.
///
/// # Errors
/// [`Error::Malformed`] if `server` or `email` is empty or contains `"`, `\`
/// or a control character, [`Error::SizeLimit`] if the code would exceed
/// [`MAX_SETUP_CODE_LEN`], [`Error::Allocation`].
pub fn build(code: &SetupCode) -> Result<Zeroizing<String>> {
    if !plain(&code.server) || !plain(&code.email) {
        return Err(Error::Malformed);
    }
    let sk = code.secret_key.to_display()?;
    let mut json = Zeroizing::new(String::with_capacity(
        JSON_HEAD.len() + code.server.len() + code.email.len() + 96,
    ));
    json.push_str(JSON_HEAD);
    json.push_str(&code.server);
    json.push_str(JSON_EMAIL);
    json.push_str(&code.email);
    json.push_str(JSON_SK);
    json.push_str(sk.expose_secret());
    json.push_str(JSON_TAIL);
    let encoded_len = Base64UrlUnpadded::encoded_len(json.as_bytes());
    if SETUP_CODE_PREFIX.len() + encoded_len > MAX_SETUP_CODE_LEN {
        return Err(Error::SizeLimit);
    }
    let mut buf = Zeroizing::new(vec![0u8; encoded_len]);
    let encoded =
        Base64UrlUnpadded::encode(json.as_bytes(), &mut buf).map_err(|_| Error::Malformed)?;
    let mut out = Zeroizing::new(String::with_capacity(SETUP_CODE_PREFIX.len() + encoded_len));
    out.push_str(SETUP_CODE_PREFIX);
    out.push_str(encoded);
    Ok(out)
}

/// Parses a setup code strictly.
///
/// `canonical(server, email)` returns the canonical origin and the normalized
/// email for the two strings, or `None` if either is invalid; the code must
/// carry exactly those forms.
///
/// # Errors
/// [`Error::Malformed`] for anything that is not exactly the format above
/// (prefix, base64url, members, order, `v`, size, non-canonical server or
/// email, a Secret Key not in display form), [`Error::SecretKeyMistyped`] if
/// the Secret Key's checksum fails.
pub fn parse(
    bytes: &[u8],
    canonical: impl Fn(&str, &str) -> Option<(String, String)>,
) -> Result<SetupCode> {
    if bytes.len() > MAX_SETUP_CODE_LEN {
        return Err(Error::Malformed);
    }
    let text = core::str::from_utf8(bytes).map_err(|_| Error::Malformed)?;
    let encoded = text
        .strip_prefix(SETUP_CODE_PREFIX)
        .ok_or(Error::Malformed)?;
    let mut buf = Zeroizing::new([0u8; MAX_SETUP_CODE_LEN]);
    let json =
        Base64UrlUnpadded::decode(encoded, buf.as_mut_slice()).map_err(|_| Error::Malformed)?;
    let json = core::str::from_utf8(json).map_err(|_| Error::Malformed)?;
    let rest = json.strip_prefix(JSON_HEAD).ok_or(Error::Malformed)?;
    let (server, rest) = rest.split_once(JSON_EMAIL).ok_or(Error::Malformed)?;
    let (email, rest) = rest.split_once(JSON_SK).ok_or(Error::Malformed)?;
    let sk = rest.strip_suffix(JSON_TAIL).ok_or(Error::Malformed)?;
    if !plain(server) || !plain(email) {
        return Err(Error::Malformed);
    }
    match canonical(server, email) {
        Some((s, e)) if s == server && e == email => {}
        _ => return Err(Error::Malformed),
    }
    let secret_key = SecretKey::parse(sk)?;
    // Only the display form is canonical: no lowercase, no missing dashes.
    let shown = secret_key.to_display()?;
    if shown.expose_secret() != sk {
        return Err(Error::Malformed);
    }
    Ok(SetupCode {
        server: server.to_owned(),
        email: email.to_owned(),
        secret_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecretBytes;

    /// A stand-in for the sync crate's rules: lowercase origin and email.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the parser takes a function returning an Option"
    )]
    fn lower(server: &str, email: &str) -> Option<(String, String)> {
        Some((server.to_lowercase(), email.to_lowercase()))
    }

    /// The golden Secret Key `00 01 .. 0f`.
    fn fixed_key() -> SecretKey {
        let mut bytes: [u8; 16] = core::array::from_fn(|i| u8::try_from(i).unwrap());
        SecretKey::from_secret(SecretBytes::from_array(&mut bytes))
    }

    fn sample() -> SetupCode {
        SetupCode {
            server: "https://sync.example.com".to_owned(),
            email: "alice@example.com".to_owned(),
            secret_key: fixed_key(),
        }
    }

    fn encode(json: &str) -> String {
        let mut buf = vec![0u8; Base64UrlUnpadded::encoded_len(json.as_bytes())];
        let encoded = Base64UrlUnpadded::encode(json.as_bytes(), &mut buf).unwrap();
        format!("{SETUP_CODE_PREFIX}{encoded}")
    }

    fn json_for(code: &SetupCode) -> String {
        format!(
            "{{\"v\":1,\"server\":\"{}\",\"email\":\"{}\",\"sk\":\"{}\"}}",
            code.server,
            code.email,
            code.secret_key.to_display().unwrap().expose_secret()
        )
    }

    #[test]
    fn setup_code_round_trips() {
        let code = sample();
        let text = build(&code).unwrap();
        assert!(text.starts_with(SETUP_CODE_PREFIX));
        assert!(text.len() <= MAX_SETUP_CODE_LEN);
        let back = parse(text.as_bytes(), lower).unwrap();
        assert_eq!(back.server, code.server);
        assert_eq!(back.email, code.email);
        assert_eq!(back.secret_key, code.secret_key);
        // Debug never shows the key.
        assert!(
            !format!("{back:?}").contains(code.secret_key.to_display().unwrap().expose_secret())
        );
    }

    #[test]
    fn setup_code_rejects_wrong_prefix_version_keys_and_oversize() {
        let code = sample();
        let good = json_for(&code);
        assert!(parse(encode(&good).as_bytes(), lower).is_ok());
        let cases = [
            // Wrong prefix, version.
            good.replace("\"v\":1", "\"v\":2"),
            // Member order, whitespace, extra and missing members.
            good.replace("{\"v\":1,", "{ \"v\":1,"),
            good.replace(",\"sk\"", ",\"x\":1,\"sk\""),
            good.replace("\"v\":1,", ""),
            format!("{good} "),
            // Other server shapes the stand-in refuses.
            good.replace("https://sync.example.com", ""),
        ];
        for json in &cases {
            assert_eq!(
                parse(encode(json).as_bytes(), lower).err(),
                Some(Error::Malformed),
                "{json}"
            );
        }
        let text = encode(&good);
        assert!(
            parse(
                text.replacen("mosaic-setup:1:", "mosaic-setup:2:", 1)
                    .as_bytes(),
                lower
            )
            .is_err()
        );
        assert!(
            parse(
                text.replacen("mosaic-setup:1:", "mosaic-setup:1", 1)
                    .as_bytes(),
                lower
            )
            .is_err()
        );
        assert!(parse(format!("{text}=").as_bytes(), lower).is_err());
        assert!(parse(format!("{text}\n").as_bytes(), lower).is_err());
        assert!(parse(b"", lower).is_err());
        assert!(parse(&[0xFF, 0xFE], lower).is_err());
        // Oversize: over 1 024 bytes, even if otherwise valid.
        let mut long = sample();
        long.email = format!("{}@example.com", "a".repeat(900));
        assert_eq!(build(&long).err(), Some(Error::SizeLimit));
        let json = json_for(&long);
        assert_eq!(
            parse(encode(&json).as_bytes(), lower).err(),
            Some(Error::Malformed)
        );
        // A server or email that needs JSON escaping is refused on both sides.
        let mut quoted = sample();
        quoted.email = "a\"b@example.com".to_owned();
        assert_eq!(build(&quoted).err(), Some(Error::Malformed));
    }

    #[test]
    fn setup_code_checks_the_secret_key_checksum() {
        let code = sample();
        let shown = code.secret_key.to_display().unwrap();
        let sk = shown.expose_secret();
        // Change one symbol of the first group: the checksum no longer matches.
        let first = sk.chars().nth(4).unwrap();
        let other = if first == '2' { '3' } else { '2' };
        let bad = format!("{}{other}{}", &sk[..4], &sk[5..]);
        let json = json_for(&code).replace(sk, &bad);
        assert_eq!(
            parse(encode(&json).as_bytes(), lower).err(),
            Some(Error::SecretKeyMistyped)
        );
        // Not a Secret Key at all, and a lowercase (non-canonical) one.
        let json = json_for(&code).replace(sk, "not-a-key");
        assert!(parse(encode(&json).as_bytes(), lower).is_err());
        let json = json_for(&code).replace(sk, &sk.to_lowercase());
        assert_eq!(
            parse(encode(&json).as_bytes(), lower).err(),
            Some(Error::Malformed)
        );
    }

    #[test]
    fn setup_code_normalizes_email_and_origin() {
        let mut code = sample();
        code.server = "https://Sync.Example.com".to_owned();
        let text = build(&code).unwrap();
        // The code must carry the canonical forms exactly.
        assert_eq!(parse(text.as_bytes(), lower).err(), Some(Error::Malformed));
        code.server = "https://sync.example.com".to_owned();
        code.email = "Alice@Example.com".to_owned();
        let text = build(&code).unwrap();
        assert_eq!(parse(text.as_bytes(), lower).err(), Some(Error::Malformed));
        // A checker that refuses refuses the code.
        let good = build(&sample()).unwrap();
        assert_eq!(
            parse(good.as_bytes(), |_, _| None).err(),
            Some(Error::Malformed)
        );
    }
}
