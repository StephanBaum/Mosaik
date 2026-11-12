//! The vendored allowlist of Android browsers that may pass a web origin
//! (`CallingAppInfo.getOrigin`, ADR-0092).
//!
//! Source: Google's published list of privileged apps,
//! `https://www.gstatic.com/gpm-passkeys-privileged-apps/apps.json`, fetched
//! 2026-11-11 (M10 T2). Only the `release` signatures of Chrome (stable,
//! beta), Firefox (release, beta, Fenix), Edge (stable, beta), Brave (stable,
//! beta), Samsung Internet (stable, beta), `DuckDuckGo`, Vivaldi and Opera were
//! kept; `userdebug` builds and every other app are left out. The file keeps
//! Google's format, so it can be compared with the source.

use std::sync::OnceLock;

const DATA: &str = include_str!("../data/android-privileged-browsers.json");

/// The allowlist file as vendored, in the `getOrigin` format of Credential
/// Manager: `{"apps":[{"type":"android","info":{"package_name":…,"signatures":[…]}}]}`.
#[must_use]
pub const fn privileged_browsers_json() -> &'static str {
    DATA
}

fn parse_fingerprint(s: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    let mut parts = s.split(':');
    for slot in &mut out {
        let part = parts.next()?;
        if part.len() != 2 {
            return None;
        }
        *slot = u8::from_str_radix(part, 16).ok()?;
    }
    parts.next().is_none().then_some(out)
}

fn parse(json: &str) -> Option<Vec<(String, Vec<[u8; 32]>)>> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut entries = Vec::new();
    for app in value.get("apps")?.as_array()? {
        let info = app.get("info")?;
        let package = info.get("package_name")?.as_str()?.to_owned();
        let mut certs = Vec::new();
        for sig in info.get("signatures")?.as_array()? {
            certs.push(parse_fingerprint(
                sig.get("cert_fingerprint_sha256")?.as_str()?,
            )?);
        }
        entries.push((package, certs));
    }
    Some(entries)
}

fn entries() -> &'static [(String, Vec<[u8; 32]>)] {
    static CACHE: OnceLock<Vec<(String, Vec<[u8; 32]>)>> = OnceLock::new();
    // A file that does not parse trusts no browser (fail closed).
    CACHE.get_or_init(|| parse(DATA).unwrap_or_default())
}

/// Whether the caller is an allowlisted browser: the package is listed and the
/// caller's **one** current signing certificate (SHA-256) is one of the
/// browser's listed release certificates (a browser may list several, for
/// rotation). A caller that presents more than one current signer is refused
/// (ADR-0092 amendment, G1-9): its identity must be a single certificate.
#[must_use]
pub fn is_trusted_browser(package: &str, certs: &[[u8; 32]]) -> bool {
    let [cert] = certs else {
        return false;
    };
    entries()
        .iter()
        .any(|(p, known)| p == package && known.contains(cert))
}

/// The number of allowlisted packages (for tests).
#[cfg(test)]
pub(crate) fn package_count() -> usize {
    entries().len()
}

#[cfg(test)]
pub(crate) fn first_cert(package: &str) -> Option<[u8; 32]> {
    entries()
        .iter()
        .find(|(p, _)| p == package)
        .and_then(|(_, c)| c.first().copied())
}
