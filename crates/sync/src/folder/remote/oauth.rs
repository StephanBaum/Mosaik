//! The OAuth 2.0 pieces the provider links share (RFC 7636 PKCE, RFC 8252
//! custom-scheme redirect): the verifier and challenge, the `state` with a
//! constant-time check, the callback parser, form and path encoding, and the
//! provider endpoints.
//!
//! The verifier, the code and the tokens are zeroizing and never in an error.

use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::transport::TransportError;

/// The redirect URI of the app: a private-use URI scheme named after the
/// reversed package (RFC 8252 section 7.1).
pub const REDIRECT_URI: &str = "io.github.stephanbaum.mosaic.oauth:/cb";

/// The longest callback URL that is parsed.
const MAX_CALLBACK_LEN: usize = 4096;

/// A PKCE verifier (RFC 7636): 32 random bytes as 43 base64url characters.
pub struct Pkce {
    verifier: Zeroizing<String>,
}

impl Pkce {
    /// A fresh verifier.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the system has no random source.
    pub fn new() -> Result<Self, TransportError> {
        let mut bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(bytes.as_mut_slice()).map_err(|_| TransportError::Io)?;
        Ok(Self {
            verifier: Zeroizing::new(Base64UrlUnpadded::encode_string(bytes.as_slice())),
        })
    }

    #[cfg(test)]
    fn from_verifier_for_test(verifier: &str) -> Self {
        Self {
            verifier: Zeroizing::new(verifier.to_owned()),
        }
    }

    /// The `code_challenge` (`S256`): base64url of the SHA-256 of the verifier.
    #[must_use]
    pub fn challenge(&self) -> String {
        Base64UrlUnpadded::encode_string(Sha256::digest(self.verifier.as_bytes()).as_slice())
    }

    /// The `code_verifier` for the token request.
    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }
}

impl core::fmt::Debug for Pkce {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Pkce([REDACTED])")
    }
}

/// The `state` of one authorization request: 32 random bytes.
pub struct AuthState([u8; 32]);

impl AuthState {
    /// A fresh `state`.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the system has no random source.
    pub fn new() -> Result<Self, TransportError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| TransportError::Io)?;
        Ok(Self(bytes))
    }

    /// The value for the `state` query parameter (base64url).
    #[must_use]
    pub fn as_param(&self) -> String {
        Base64UrlUnpadded::encode_string(&self.0)
    }

    /// Whether `other` (the `state` of a callback) is this state: decoded,
    /// then compared in constant time.
    #[must_use]
    pub fn matches(&self, other: &str) -> bool {
        let Ok(decoded) = Base64UrlUnpadded::decode_vec(other) else {
            return false;
        };
        decoded.len() == self.0.len() && bool::from(decoded.as_slice().ct_eq(&self.0))
    }
}

/// What the redirect brought back.
pub struct Callback {
    /// The authorization code.
    pub code: Zeroizing<String>,
    /// The `state` as it came back (check it with [`AuthState::matches`]).
    pub state: String,
}

/// Why a callback is not usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackError {
    /// Not our redirect, malformed, repeated keys, or `code` or `state` missing.
    Invalid,
    /// The user (or the provider) refused: the URL carries `error`.
    Denied,
}

const fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Percent-decodes (`+` is a space when `plus_is_space`); `None` for a bad
/// escape or a result that is not UTF-8.
pub(crate) fn percent_decode(s: &str, plus_is_space: bool) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = hex_digit(bytes.next()?)?;
                let lo = hex_digit(bytes.next()?)?;
                out.push(hi << 4 | lo);
            }
            b'+' if plus_is_space => out.push(b' '),
            _ => out.push(b),
        }
    }
    String::from_utf8(out).ok()
}

/// Percent-encodes everything except the unreserved characters
/// (`A-Z a-z 0-9 - . _ ~`).
fn encode_into(out: &mut Vec<u8>, s: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b);
        } else {
            out.push(b'%');
            for nibble in [b >> 4, b & 15] {
                out.push(HEX.get(usize::from(nibble)).copied().unwrap_or(b'0'));
            }
        }
    }
}

/// An `application/x-www-form-urlencoded` body of `pairs`.
#[must_use]
pub(crate) fn form(pairs: &[(&str, &str)]) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::new());
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(b'&');
        }
        encode_into(&mut out, k);
        out.push(b'=');
        encode_into(&mut out, v);
    }
    out
}

/// One URL path segment: everything but the unreserved characters is
/// percent-encoded (so a `/` in a name stays inside the segment).
#[must_use]
pub(crate) fn path_segment(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    encode_into(&mut out, s);
    String::from_utf8(out).unwrap_or_default()
}

/// Reads the callback URL of the redirect.
///
/// The URL must start with exactly `redirect` and `?` and be at most 4096
/// bytes. Keys and values are percent-decoded; a key that appears twice is
/// [`CallbackError::Invalid`]; `error` with any value is
/// [`CallbackError::Denied`]; `code` and `state` must both be there. Unknown
/// keys are ignored.
///
/// # Errors
/// [`CallbackError`].
pub fn parse_callback(url: &str, redirect: &str) -> Result<Callback, CallbackError> {
    if url.len() > MAX_CALLBACK_LEN || url.contains('#') {
        return Err(CallbackError::Invalid);
    }
    let query = url
        .strip_prefix(redirect)
        .and_then(|rest| rest.strip_prefix('?'))
        .ok_or(CallbackError::Invalid)?;
    let mut seen: Vec<String> = Vec::new();
    let (mut code, mut state, mut denied) = (None, None, false);
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k, true).ok_or(CallbackError::Invalid)?;
        let value = percent_decode(v, true).ok_or(CallbackError::Invalid)?;
        if seen.contains(&key) {
            return Err(CallbackError::Invalid);
        }
        match key.as_str() {
            "error" => denied = true,
            "code" => code = Some(Zeroizing::new(value)),
            "state" => state = Some(value),
            _ => {}
        }
        seen.push(key);
    }
    if denied {
        return Err(CallbackError::Denied);
    }
    match (code, state) {
        (Some(code), Some(state)) if !code.is_empty() && !state.is_empty() => {
            Ok(Callback { code, state })
        }
        _ => Err(CallbackError::Invalid),
    }
}

/// The provider endpoints of the OAuth providers.
#[derive(Clone, Debug)]
pub struct Endpoints {
    /// The Dropbox authorization page.
    pub dropbox_auth: String,
    /// The Dropbox API (`/oauth2/token`, `/2/...`).
    pub dropbox_api: String,
    /// The Dropbox content host (`/2/files/download`, `/2/files/upload`).
    pub dropbox_content: String,
    /// The Microsoft identity platform's v2 base (`/authorize`, `/token`).
    pub ms_login: String,
    /// The Microsoft Graph base, with the version.
    pub graph: String,
}

impl Endpoints {
    /// The real providers.
    #[must_use]
    pub fn production() -> Self {
        Self {
            dropbox_auth: "https://www.dropbox.com/oauth2/authorize".to_owned(),
            dropbox_api: "https://api.dropboxapi.com".to_owned(),
            dropbox_content: "https://content.dropboxapi.com".to_owned(),
            ms_login: "https://login.microsoftonline.com/common/oauth2/v2.0".to_owned(),
            graph: "https://graph.microsoft.com/v1.0".to_owned(),
        }
    }

    /// Every endpoint on one loopback server (`origin` is
    /// `http://127.0.0.1:<port>`), for the tests.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    pub fn loopback(origin: &str) -> Self {
        Self {
            dropbox_auth: format!("{origin}/oauth2/authorize"),
            dropbox_api: origin.to_owned(),
            dropbox_content: origin.to_owned(),
            ms_login: format!("{origin}/common/oauth2/v2.0"),
            graph: format!("{origin}/v1.0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_rfc7636_appendix_b() {
        let pkce = Pkce::from_verifier_for_test("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(pkce.challenge(), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let fresh = Pkce::new().unwrap();
        assert_eq!(fresh.verifier().len(), 43);
        assert_ne!(fresh.verifier(), Pkce::new().unwrap().verifier());
        assert!(!format!("{fresh:?}").contains(fresh.verifier()));
    }

    #[test]
    fn state_matches_only_itself() {
        let a = AuthState::new().unwrap();
        let b = AuthState::new().unwrap();
        assert!(a.matches(&a.as_param()));
        assert!(!a.matches(&b.as_param()));
        assert!(!a.matches(""));
        assert!(!a.matches("not base64 !!"));
        assert!(!a.matches(&a.as_param()[..42]));
        assert!(!a.matches(&format!("{}A", a.as_param())));
    }

    #[test]
    fn parse_callback_table() {
        let r = REDIRECT_URI;
        let ok = parse_callback(&format!("{r}?code=ab%20c&state=xyz&extra=1"), r).unwrap();
        assert_eq!(ok.code.as_str(), "ab c");
        assert_eq!(ok.state, "xyz");
        let long = format!("{r}?code={}&state=x", "a".repeat(MAX_CALLBACK_LEN));
        for bad in [
            "https://example.com/cb?code=a&state=b".to_owned(),
            "io.github.stephanbaum.mosaic.oauth:/cb2?code=a&state=b".to_owned(),
            "io.github.stephanbaum.mosaic.oauth:/cb/x?code=a&state=b".to_owned(),
            r.to_string(),
            format!("{r}?code=a"),
            format!("{r}?state=b"),
            format!("{r}?code=&state=b"),
            format!("{r}?code=a&state=b&state=c"),
            format!("{r}?code=a&code=b&state=c"),
            format!("{r}?code=%zz&state=b"),
            format!("{r}?code=a&state=b#frag"),
            long,
        ] {
            assert_eq!(parse_callback(&bad, r).err(), Some(CallbackError::Invalid), "{bad}");
        }
        for denied in [
            format!("{r}?error=access_denied&state=x"),
            format!("{r}?error=&state=x"),
            format!("{r}?code=a&state=x&error=server_error"),
        ] {
            assert_eq!(
                parse_callback(&denied, r).err(),
                Some(CallbackError::Denied),
                "{denied}"
            );
        }
    }

    #[test]
    fn encoding_keeps_only_unreserved_characters() {
        let body = form(&[("grant_type", "authorization_code"), ("code", "a b&c=d/é~")]);
        assert_eq!(
            std::str::from_utf8(&body).unwrap(),
            "grant_type=authorization_code&code=a%20b%26c%3Dd%2F%C3%A9~"
        );
        assert_eq!(path_segment("a b/c.txt"), "a%20b%2Fc.txt");
        assert_eq!(percent_decode("a%20b%2F", false).as_deref(), Some("a b/"));
        assert_eq!(percent_decode("a+b", false).as_deref(), Some("a+b"));
        assert_eq!(percent_decode("a+b", true).as_deref(), Some("a b"));
        assert_eq!(percent_decode("%ff", false), None);
        assert_eq!(percent_decode("%4", false), None);
    }
}
