//! Digital Asset Links (`/.well-known/assetlinks.json`): does a site delegate
//! to an Android app (ADR-0092, ADR-0093, CRYPTO_SPEC §16)?
//!
//! Pure parsing and matching; the fetch lives in `mosaic_sync::http::assetlinks`.

use serde_json::Value;

/// The largest file accepted, in bytes.
pub const MAX_ASSET_LINKS_LEN: usize = 64 * 1024;

/// The most statements accepted.
pub const MAX_STATEMENTS: usize = 256;

/// A relation a site can grant an app.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relation {
    /// `delegate_permission/common.get_login_creds`: passwords and passkeys.
    GetLoginCreds,
    /// `delegate_permission/common.handle_all_urls`: accepted for passkeys
    /// only, as Credential Manager does.
    HandleAllUrls,
}

impl Relation {
    /// The relation string in the file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetLoginCreds => "delegate_permission/common.get_login_creds",
            Self::HandleAllUrls => "delegate_permission/common.handle_all_urls",
        }
    }
}

/// The target of a statement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Target {
    /// `namespace: "android_app"`.
    AndroidApp {
        /// The package name.
        package: String,
        /// The `sha256_cert_fingerprints`, as written (colon-hex).
        cert_fingerprints: Vec<String>,
    },
    /// `namespace: "web"`.
    Web {
        /// The site.
        site: String,
    },
}

/// One statement of the file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Statement {
    /// The relation strings.
    pub relation: Vec<String>,
    /// The target.
    pub target: Target,
}

/// Why a file is refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetLinksError {
    /// Larger than [`MAX_ASSET_LINKS_LEN`].
    #[error("asset links file too large")]
    TooLarge,
    /// Not the expected JSON shape.
    #[error("malformed asset links file")]
    Malformed,
    /// More than [`MAX_STATEMENTS`] statements.
    #[error("too many statements")]
    TooManyStatements,
    /// A `target.namespace` other than `android_app` or `web`.
    #[error("unsupported namespace")]
    UnsupportedNamespace,
}

fn strings(v: &Value) -> Result<Vec<String>, AssetLinksError> {
    v.as_array()
        .ok_or(AssetLinksError::Malformed)?
        .iter()
        .map(|s| {
            s.as_str()
                .map(str::to_owned)
                .ok_or(AssetLinksError::Malformed)
        })
        .collect()
}

fn string_member(v: &Value, key: &str) -> Result<String, AssetLinksError> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(AssetLinksError::Malformed)
}

fn statement(v: &Value) -> Result<Statement, AssetLinksError> {
    let relation = strings(v.get("relation").ok_or(AssetLinksError::Malformed)?)?;
    let target = v.get("target").ok_or(AssetLinksError::Malformed)?;
    let target = match string_member(target, "namespace")?.as_str() {
        "android_app" => Target::AndroidApp {
            package: string_member(target, "package_name")?,
            cert_fingerprints: strings(
                target
                    .get("sha256_cert_fingerprints")
                    .ok_or(AssetLinksError::Malformed)?,
            )?,
        },
        "web" => Target::Web {
            site: string_member(target, "site")?,
        },
        _ => return Err(AssetLinksError::UnsupportedNamespace),
    };
    Ok(Statement { relation, target })
}

/// Parses an asset links file: strict JSON, at most [`MAX_ASSET_LINKS_LEN`]
/// bytes and [`MAX_STATEMENTS`] statements; unknown members are ignored.
///
/// # Errors
/// [`AssetLinksError`].
pub fn parse_asset_links(bytes: &[u8]) -> Result<Vec<Statement>, AssetLinksError> {
    if bytes.len() > MAX_ASSET_LINKS_LEN {
        return Err(AssetLinksError::TooLarge);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| AssetLinksError::Malformed)?;
    let items = value.as_array().ok_or(AssetLinksError::Malformed)?;
    if items.len() > MAX_STATEMENTS {
        return Err(AssetLinksError::TooManyStatements);
    }
    items.iter().map(statement).collect()
}

/// `AA:BB:...` (uppercase colon-hex) of a certificate hash.
fn colon_hex(cert: &[u8; 32]) -> String {
    use core::fmt::Write;
    let mut out = String::with_capacity(95);
    for (i, b) in cert.iter().enumerate() {
        if i > 0 {
            out.push(':');
        }
        let _ = write!(out, "{b:02X}");
    }
    out
}

/// Whether a statement grants `relation` to the app `package` signed by the
/// certificate `cert_sha256` (fingerprints compare case-insensitively).
#[must_use]
pub fn app_is_delegated(
    statements: &[Statement],
    package: &str,
    cert_sha256: &[u8; 32],
    relation: Relation,
) -> bool {
    let wanted = colon_hex(cert_sha256);
    statements.iter().any(|s| {
        s.relation.iter().any(|r| r == relation.as_str())
            && matches!(&s.target, Target::AndroidApp { package: p, cert_fingerprints }
                if p == package && cert_fingerprints.iter().any(|f| f.eq_ignore_ascii_case(&wanted)))
    })
}

/// Whether the app may use the site's passkeys: either relation is enough.
#[must_use]
pub fn app_is_delegated_for_passkeys(
    statements: &[Statement],
    package: &str,
    cert_sha256: &[u8; 32],
) -> bool {
    app_is_delegated(statements, package, cert_sha256, Relation::GetLoginCreds)
        || app_is_delegated(statements, package, cert_sha256, Relation::HandleAllUrls)
}
