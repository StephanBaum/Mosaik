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
//! The folder form (v2, ADR-0161) is for accounts that sync through a cloud
//! folder: `"mosaic-setup:2:" || base64url_unpadded(JSON)` with
//! `{"v":2,"kind":"folder","account":"<32 hex>","provider":"..","server":"..","path":"..","sk":".."}`
//! (seven members in this order). It carries no secret that v1 does not.
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

/// The prefix of the folder form (v2).
pub const SETUP_CODE_PREFIX_V2: &str = "mosaic-setup:2:";

/// The longest path hint of a folder setup code, in bytes.
pub const MAX_PATH_HINT_LEN: usize = 512;

const FOLDER_HEAD: &str = "{\"v\":2,\"kind\":\"folder\",\"account\":\"";
const FOLDER_PROVIDER: &str = "\",\"provider\":\"";
const FOLDER_SERVER: &str = "\",\"server\":\"";
const FOLDER_PATH: &str = "\",\"path\":\"";

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

/// Which provider the desktop's folder lives with (a hint for the phone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderProviderHint {
    /// Dropbox.
    Dropbox,
    /// OneDrive (personal or work/school).
    OneDrive,
    /// Nextcloud or another WebDAV server.
    WebDav,
    /// Unknown: the phone asks the user.
    Other,
}

impl FolderProviderHint {
    /// The wire name.
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Dropbox => "dropbox",
            Self::OneDrive => "onedrive",
            Self::WebDav => "webdav",
            Self::Other => "other",
        }
    }

    /// The provider for a wire name, `None` for any other string.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "dropbox" => Some(Self::Dropbox),
            "onedrive" => Some(Self::OneDrive),
            "webdav" => Some(Self::WebDav),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

/// What a folder setup code carries. Not `Clone`; `Debug` redacts the Secret
/// Key.
#[derive(Debug)]
pub struct FolderSetupCode {
    /// The folder account's ID.
    pub account: [u8; 16],
    /// The provider hint.
    pub provider: FolderProviderHint,
    /// The canonical `https` origin for `WebDav`, otherwise empty.
    pub server: String,
    /// The path hint below the provider root, or empty.
    pub path: String,
    /// The account's Secret Key.
    pub secret_key: SecretKey,
}

/// A parsed setup code of either form.
#[derive(Debug)]
pub enum ParsedSetupCode {
    /// A hosted account (v1).
    Hosted(SetupCode),
    /// A folder account (v2).
    Folder(FolderSetupCode),
}

/// Whether `path` is an acceptable path hint: empty, or `/` followed by
/// components that are non-empty and not `.` or `..`, no trailing `/`, at most
/// [`MAX_PATH_HINT_LEN`] bytes, and no `"`, `\` or control character.
#[must_use]
pub fn valid_path_hint(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    if path.len() > MAX_PATH_HINT_LEN
        || path
            .chars()
            .any(|c| c == '"' || c == '\\' || c.is_control())
    {
        return false;
    }
    let Some(rest) = path.strip_prefix('/') else {
        return false;
    };
    rest.split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn hex_lower(bytes: &[u8; 16]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(32);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    out
}

fn parse_hex32(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (slot, pair) in out.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
        *slot = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}

/// A lowercase hex digit only.
fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// The server and provider must agree: a server exactly for `WebDav`.
fn server_matches_provider(provider: FolderProviderHint, server: &str) -> bool {
    if provider == FolderProviderHint::WebDav {
        plain(server)
    } else {
        server.is_empty()
    }
}

/// Builds the folder setup code (v2).
///
/// # Errors
/// [`Error::Malformed`] if a member breaks the rules of CRYPTO_SPEC §6.9.7
/// (a server not matching the provider, a bad path), [`Error::SizeLimit`] if
/// the code would exceed [`MAX_SETUP_CODE_LEN`], [`Error::Allocation`].
pub fn build_folder(code: &FolderSetupCode) -> Result<Zeroizing<String>> {
    if !server_matches_provider(code.provider, &code.server) || !valid_path_hint(&code.path) {
        return Err(Error::Malformed);
    }
    let sk = code.secret_key.to_display()?;
    let mut json = Zeroizing::new(String::with_capacity(
        FOLDER_HEAD.len() + code.server.len() + code.path.len() + 160,
    ));
    json.push_str(FOLDER_HEAD);
    json.push_str(&hex_lower(&code.account));
    json.push_str(FOLDER_PROVIDER);
    json.push_str(code.provider.as_wire());
    json.push_str(FOLDER_SERVER);
    json.push_str(&code.server);
    json.push_str(FOLDER_PATH);
    json.push_str(&code.path);
    json.push_str(JSON_SK);
    json.push_str(sk.expose_secret());
    json.push_str(JSON_TAIL);
    let encoded_len = Base64UrlUnpadded::encoded_len(json.as_bytes());
    if SETUP_CODE_PREFIX_V2.len() + encoded_len > MAX_SETUP_CODE_LEN {
        return Err(Error::SizeLimit);
    }
    let mut buf = Zeroizing::new(vec![0u8; encoded_len]);
    let encoded =
        Base64UrlUnpadded::encode(json.as_bytes(), &mut buf).map_err(|_| Error::Malformed)?;
    let mut out = Zeroizing::new(String::with_capacity(
        SETUP_CODE_PREFIX_V2.len() + encoded_len,
    ));
    out.push_str(SETUP_CODE_PREFIX_V2);
    out.push_str(encoded);
    Ok(out)
}

/// Parses a setup code of either form, strictly.
///
/// `canonical` is the v1 rule of [`parse`]. `canonical_server` returns the
/// canonical `https` origin of a WebDAV server for the v2 form, or `None`.
///
/// # Errors
/// As [`parse`] for v1. For v2, [`Error::Malformed`] for anything that is not
/// exactly the format of CRYPTO_SPEC §6.9.7, [`Error::SecretKeyMistyped`] if
/// the Secret Key's checksum fails.
pub fn parse_any(
    bytes: &[u8],
    canonical: impl Fn(&str, &str) -> Option<(String, String)>,
    canonical_server: impl Fn(&str) -> Option<String>,
) -> Result<ParsedSetupCode> {
    if bytes.len() > MAX_SETUP_CODE_LEN {
        return Err(Error::Malformed);
    }
    let text = core::str::from_utf8(bytes).map_err(|_| Error::Malformed)?;
    if text.starts_with(SETUP_CODE_PREFIX) {
        return parse(bytes, canonical).map(ParsedSetupCode::Hosted);
    }
    let encoded = text
        .strip_prefix(SETUP_CODE_PREFIX_V2)
        .ok_or(Error::Malformed)?;
    let mut buf = Zeroizing::new([0u8; MAX_SETUP_CODE_LEN]);
    let json =
        Base64UrlUnpadded::decode(encoded, buf.as_mut_slice()).map_err(|_| Error::Malformed)?;
    let json = core::str::from_utf8(json).map_err(|_| Error::Malformed)?;
    let rest = json.strip_prefix(FOLDER_HEAD).ok_or(Error::Malformed)?;
    let (account, rest) = rest.split_once(FOLDER_PROVIDER).ok_or(Error::Malformed)?;
    let (provider, rest) = rest.split_once(FOLDER_SERVER).ok_or(Error::Malformed)?;
    let (server, rest) = rest.split_once(FOLDER_PATH).ok_or(Error::Malformed)?;
    let (path, rest) = rest.split_once(JSON_SK).ok_or(Error::Malformed)?;
    let sk = rest.strip_suffix(JSON_TAIL).ok_or(Error::Malformed)?;
    let account = parse_hex32(account).ok_or(Error::Malformed)?;
    let provider = FolderProviderHint::from_wire(provider).ok_or(Error::Malformed)?;
    if !server_matches_provider(provider, server) || !valid_path_hint(path) {
        return Err(Error::Malformed);
    }
    if provider == FolderProviderHint::WebDav {
        match canonical_server(server) {
            Some(s) if s == server => {}
            _ => return Err(Error::Malformed),
        }
    }
    let secret_key = SecretKey::parse(sk)?;
    let shown = secret_key.to_display()?;
    if shown.expose_secret() != sk {
        return Err(Error::Malformed);
    }
    Ok(ParsedSetupCode::Folder(FolderSetupCode {
        account,
        provider,
        server: server.to_owned(),
        path: path.to_owned(),
        secret_key,
    }))
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

    fn folder_sample() -> FolderSetupCode {
        FolderSetupCode {
            account: core::array::from_fn(|i| u8::try_from(i).unwrap()),
            provider: FolderProviderHint::Dropbox,
            server: String::new(),
            path: "/Mosaic".to_owned(),
            secret_key: fixed_key(),
        }
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "the parser takes a function returning an Option"
    )]
    fn same_server(server: &str) -> Option<String> {
        Some(server.to_owned())
    }

    fn parse_v2(text: &str) -> Result<ParsedSetupCode> {
        parse_any(text.as_bytes(), lower, same_server)
    }

    fn folder_json(account: &str, provider: &str, server: &str, path: &str, sk: &str) -> String {
        format!(
            "{{\"v\":2,\"kind\":\"folder\",\"account\":\"{account}\",\"provider\":\"{provider}\",\"server\":\"{server}\",\"path\":\"{path}\",\"sk\":\"{sk}\"}}"
        )
    }

    fn encode_v2(json: &str) -> String {
        let mut buf = vec![0u8; Base64UrlUnpadded::encoded_len(json.as_bytes())];
        let encoded = Base64UrlUnpadded::encode(json.as_bytes(), &mut buf).unwrap();
        format!("{SETUP_CODE_PREFIX_V2}{encoded}")
    }

    const ACCOUNT_HEX: &str = "000102030405060708090a0b0c0d0e0f";

    #[test]
    fn folder_code_round_trips() {
        let code = folder_sample();
        let text = build_folder(&code).unwrap();
        let ParsedSetupCode::Folder(back) = parse_v2(&text).unwrap() else {
            panic!("expected a folder code");
        };
        assert_eq!(back.account, code.account);
        assert_eq!(back.provider, code.provider);
        assert_eq!(back.server, code.server);
        assert_eq!(back.path, code.path);
        assert_eq!(back.secret_key, code.secret_key);
        assert!(
            !format!("{back:?}").contains(code.secret_key.to_display().unwrap().expose_secret())
        );
        let mut webdav = folder_sample();
        webdav.provider = FolderProviderHint::WebDav;
        webdav.server = "https://cloud.example.com".to_owned();
        let text = build_folder(&webdav).unwrap();
        let ParsedSetupCode::Folder(back) = parse_v2(&text).unwrap() else {
            panic!("expected a folder code");
        };
        assert_eq!(back.server, "https://cloud.example.com");
    }

    #[test]
    fn folder_code_golden() {
        let sk = fixed_key().to_display().unwrap();
        let expected = encode_v2(&folder_json(
            ACCOUNT_HEX,
            "dropbox",
            "",
            "/Mosaic",
            sk.expose_secret(),
        ));
        assert_eq!(build_folder(&folder_sample()).unwrap().as_str(), expected);
        assert!(expected.starts_with("mosaic-setup:2:"));
    }

    #[test]
    fn folder_code_rejects_every_deviation() {
        let sk = fixed_key().to_display().unwrap();
        let sk = sk.expose_secret().to_owned();
        let good = folder_json(ACCOUNT_HEX, "dropbox", "", "/Mosaic", &sk);
        assert!(parse_v2(&encode_v2(&good)).is_ok());
        let webdav = |server: &str| folder_json(ACCOUNT_HEX, "webdav", server, "", &sk);
        assert!(parse_v2(&encode_v2(&webdav("https://cloud.example.com"))).is_ok());
        let long_path = format!("/{}", "a".repeat(512));
        let cases = [
            // Members reordered, missing, extra.
            good.replace("\"kind\":\"folder\",", ""),
            good.replace(",\"path\":\"/Mosaic\"", ""),
            good.replace(",\"sk\"", ",\"x\":1,\"sk\""),
            good.replace(
                "\"provider\":\"dropbox\",\"server\":\"\"",
                "\"server\":\"\",\"provider\":\"dropbox\"",
            ),
            format!("{good} "),
            // Version and kind.
            good.replace("\"v\":2", "\"v\":1"),
            good.replace("\"folder\"", "\"hosted\""),
            // Account.
            good.replace(ACCOUNT_HEX, &ACCOUNT_HEX.to_uppercase()),
            good.replace(ACCOUNT_HEX, &ACCOUNT_HEX[..31]),
            good.replace(ACCOUNT_HEX, &format!("{ACCOUNT_HEX}0")),
            // Provider.
            good.replace("dropbox", "gdrive"),
            // Server and provider must agree.
            good.replace(
                "\"server\":\"\"",
                "\"server\":\"https://cloud.example.com\"",
            ),
            webdav(""),
            // Paths.
            folder_json(ACCOUNT_HEX, "dropbox", "", "Mosaic", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/a//b", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/a/../b", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/a/./b", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/a/", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/", &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", &long_path, &sk),
            folder_json(ACCOUNT_HEX, "dropbox", "", "/a\\b", &sk),
            // Only the display form of the Secret Key is canonical.
            good.replace(&sk, &sk.to_lowercase()),
        ];
        for json in &cases {
            assert_eq!(
                parse_v2(&encode_v2(json)).err(),
                Some(Error::Malformed),
                "{json}"
            );
        }
        // A v2 body behind the v1 prefix, and a v1 body behind the v2 prefix.
        let text = encode_v2(&good);
        assert!(parse_v2(&text.replacen("mosaic-setup:2:", "mosaic-setup:1:", 1)).is_err());
        let v1 = build(&sample()).unwrap();
        assert!(parse_v2(&v1.replacen("mosaic-setup:1:", "mosaic-setup:2:", 1)).is_err());
        // Padding, trailing junk, empty, over 1 024 bytes.
        assert!(parse_v2(&format!("{text}=")).is_err());
        assert!(parse_v2(&format!("{text}\n")).is_err());
        assert!(parse_v2("").is_err());
        let longest = format!("/{}", "a".repeat(511));
        let host = format!("https://{}.example.com", "a".repeat(300));
        let long = folder_json(ACCOUNT_HEX, "webdav", &host, &longest, &sk);
        assert!(encode_v2(&long).len() > MAX_SETUP_CODE_LEN);
        assert_eq!(parse_v2(&encode_v2(&long)).err(), Some(Error::Malformed));
        let mut big = folder_sample();
        big.provider = FolderProviderHint::WebDav;
        big.server = host;
        big.path = longest;
        assert_eq!(build_folder(&big).err(), Some(Error::SizeLimit));
        // A checker that refuses the server refuses the code.
        let code = encode_v2(&webdav("https://cloud.example.com"));
        assert_eq!(
            parse_any(code.as_bytes(), lower, |_| None).err(),
            Some(Error::Malformed)
        );
        // The builder refuses what the parser refuses.
        let mut bad = folder_sample();
        bad.server = "https://cloud.example.com".to_owned();
        assert_eq!(build_folder(&bad).err(), Some(Error::Malformed));
        bad.server = String::new();
        bad.path = "Mosaic".to_owned();
        assert_eq!(build_folder(&bad).err(), Some(Error::Malformed));
    }

    #[test]
    fn folder_code_checks_the_secret_key_checksum() {
        let sk = fixed_key().to_display().unwrap();
        let sk = sk.expose_secret().to_owned();
        let first = sk.chars().nth(4).unwrap();
        let other = if first == '2' { '3' } else { '2' };
        let bad = format!("{}{other}{}", &sk[..4], &sk[5..]);
        let json = folder_json(ACCOUNT_HEX, "dropbox", "", "", &bad);
        assert_eq!(
            parse_v2(&encode_v2(&json)).err(),
            Some(Error::SecretKeyMistyped)
        );
    }

    #[test]
    fn v1_code_still_parses_through_parse_any() {
        let text = build(&sample()).unwrap();
        assert!(matches!(
            parse_v2(&text).unwrap(),
            ParsedSetupCode::Hosted(code) if code.email == "alice@example.com"
        ));
    }

    #[test]
    fn provider_wire_names_round_trip() {
        for p in [
            FolderProviderHint::Dropbox,
            FolderProviderHint::OneDrive,
            FolderProviderHint::WebDav,
            FolderProviderHint::Other,
        ] {
            assert_eq!(FolderProviderHint::from_wire(p.as_wire()), Some(p));
        }
        assert_eq!(FolderProviderHint::from_wire("Dropbox"), None);
    }
}
