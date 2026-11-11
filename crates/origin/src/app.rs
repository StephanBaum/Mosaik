//! Android app URLs on login items (ADR-0092): `androidapp://<package>?cert=<b64url(sha256)>`.
//!
//! The certificate binds the URL to one signing key, so a look-alike app with
//! the same package name and another key never matches. A URL without `cert`
//! (for example from an import) never matches either: it is shown as "needs
//! linking".

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::LoginUrl;

use crate::{MatchKind, Refusal};

/// The longest Android package name accepted, in bytes.
pub const MAX_PACKAGE_LEN: usize = 255;

const SCHEME_PREFIX: &str = "androidapp://";
const CERT_KEY: &str = "?cert=";

/// Why a string is not an Android app URL.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppUrlError {
    /// Not an `androidapp://` URL.
    #[error("not an app url")]
    NotAnAppUrl,
    /// The package name is empty, too long or has characters outside
    /// `[A-Za-z0-9_.]`.
    #[error("bad package name")]
    BadPackage,
    /// The query is not exactly `cert=<43 base64url characters>` of 32 bytes.
    #[error("bad certificate")]
    BadCert,
}

/// Whether `package` is a valid Android package name for Mosaic: 1 to
/// [`MAX_PACKAGE_LEN`] bytes of `[A-Za-z0-9_.]`.
#[must_use]
pub fn is_valid_package_name(package: &str) -> bool {
    !package.is_empty()
        && package.len() <= MAX_PACKAGE_LEN
        && package
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
}

/// A parsed Android app URL.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AndroidAppUrl {
    package: String,
    cert: Option<[u8; 32]>,
}

impl AndroidAppUrl {
    /// An app URL for `package`, bound to `cert_sha256` when given.
    ///
    /// # Errors
    /// [`AppUrlError::BadPackage`].
    pub fn new(package: &str, cert_sha256: Option<[u8; 32]>) -> Result<Self, AppUrlError> {
        if !is_valid_package_name(package) {
            return Err(AppUrlError::BadPackage);
        }
        Ok(Self {
            package: package.to_owned(),
            cert: cert_sha256,
        })
    }

    /// Parses the canonical form (surrounding whitespace is ignored). No path,
    /// fragment, other query member or padding is accepted.
    ///
    /// # Errors
    /// [`AppUrlError`].
    pub fn parse(s: &str) -> Result<Self, AppUrlError> {
        let rest = s
            .trim()
            .strip_prefix(SCHEME_PREFIX)
            .ok_or(AppUrlError::NotAnAppUrl)?;
        let (package, cert) = match rest.split_once(CERT_KEY) {
            Some((package, cert)) => {
                let mut bytes = [0u8; 32];
                let decoded = Base64UrlUnpadded::decode(cert, &mut bytes)
                    .map_err(|_| AppUrlError::BadCert)?;
                if decoded.len() != 32 {
                    return Err(AppUrlError::BadCert);
                }
                (package, Some(bytes))
            }
            None => (rest, None),
        };
        Self::new(package, cert)
    }

    /// The package name.
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The SHA-256 of the signing certificate this URL is bound to.
    #[must_use]
    pub const fn cert_sha256(&self) -> Option<&[u8; 32]> {
        self.cert.as_ref()
    }

    /// The canonical string.
    #[must_use]
    pub fn canonical(&self) -> String {
        match &self.cert {
            Some(cert) => format!(
                "{SCHEME_PREFIX}{}{CERT_KEY}{}",
                self.package,
                Base64UrlUnpadded::encode_string(cert)
            ),
            None => format!("{SCHEME_PREFIX}{}", self.package),
        }
    }
}

/// Whether the saved URL allows filling the app `package` whose **current**
/// signers are `certs` (SHA-256 of each certificate).
///
/// Only an equal package and a `cert` equal to one of `certs` match; the
/// match is reported as [`MatchKind::ExactOrigin`]. A URL without `cert`, a
/// web URL and an empty `certs` never match.
///
/// # Errors
/// [`Refusal::NoMatch`].
pub fn app_fill_allowed(
    saved: &LoginUrl,
    package: &str,
    certs: &[[u8; 32]],
) -> Result<MatchKind, Refusal> {
    let url = AndroidAppUrl::parse(saved.url.as_str()).map_err(|_| Refusal::NoMatch)?;
    let cert = url.cert_sha256().ok_or(Refusal::NoMatch)?;
    if url.package() == package && certs.contains(cert) {
        Ok(MatchKind::ExactOrigin)
    } else {
        Err(Refusal::NoMatch)
    }
}
