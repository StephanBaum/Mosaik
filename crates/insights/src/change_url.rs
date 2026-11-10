//! Where a site changes its password (M7b, ADR-0070): a URL for the user to
//! open, found without any network request.
//!
//! [`resolve`] tries, in order:
//!
//! 1. [`ChangeUrlSource::Stored`]: the item's own
//!    `login.change_password_url`, if it is a plain `https` URL (no user
//!    info, no whitespace) whose registrable domain is one of the item's
//!    `https` URLs. Anything else is ignored (the editor shows the form
//!    error; an item from a newer or hostile writer cannot point the user
//!    at an unrelated site).
//! 2. [`ChangeUrlSource::Quirks`]: Apple's Password Manager Resources
//!    `quirks/change-password-URLs.json` (MIT, `data/README.md`), keyed by
//!    the host of an item URL or one of its parent domains.
//! 3. [`ChangeUrlSource::WellKnown`]: the `/.well-known/change-password`
//!    path on the origin of the first `https` item URL (W3C "A Well-Known URL
//!    for Changing Passwords").
//!
//! The URL is never fetched: the W3C draft's "status check" would be a
//! request from this device to the site. A site without support shows its own
//! 404 in the user's tab.

use std::collections::HashMap;
use std::sync::OnceLock;

use mosaic_core::model::{Item, TextField};
use mosaic_generator::host::{candidates, normalize};
use mosaic_origin::{PageOrigin, registrable_domain, saved_origin};
use serde_json::Value;

const QUIRKS_JSON: &str = include_str!("../data/change-password-URLs.json");

/// The longest change-page URL accepted, in bytes.
pub const MAX_URL_LEN: usize = 2048;

/// Where a [`ChangeUrl`] came from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChangeUrlSource {
    /// The item's own change-page URL.
    Stored,
    /// The vendored per-site URL list.
    Quirks,
    /// The `/.well-known/change-password` of the item's site.
    WellKnown,
}

impl ChangeUrlSource {
    /// The name used across the IPC boundary: `stored`, `quirks`,
    /// `wellKnown`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Quirks => "quirks",
            Self::WellKnown => "wellKnown",
        }
    }
}

/// A change-page URL (always `https`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChangeUrl {
    /// The URL to open.
    pub url: String,
    /// Where it came from.
    pub source: ChangeUrlSource,
}

/// The change-page URL of `item`, or `None` if it has no `https` URL.
#[must_use]
pub fn resolve(item: &Item) -> Option<ChangeUrl> {
    let origins = https_origins(item);
    if let Some(url) = item
        .text(TextField::ChangePasswordUrl)
        .and_then(|s| valid_stored(s, &origins))
    {
        return Some(ChangeUrl {
            url,
            source: ChangeUrlSource::Stored,
        });
    }
    for origin in &origins {
        if let Some(url) = quirk_for(&origin.host_str()) {
            return Some(ChangeUrl {
                url: url.to_owned(),
                source: ChangeUrlSource::Quirks,
            });
        }
    }
    let first = origins.first()?;
    Some(ChangeUrl {
        url: format!("{}/.well-known/change-password", first.serialize()),
        source: ChangeUrlSource::WellKnown,
    })
}

/// Whether `candidate` may be stored as the item's change-page URL (the
/// editor's form check): [`resolve`] accepts exactly these.
#[must_use]
pub fn is_valid_stored(item: &Item, candidate: &str) -> bool {
    valid_stored(candidate, &https_origins(item)).is_some()
}

/// The `https` origins of the item's URLs, in creation order.
fn https_origins(item: &Item) -> Vec<PageOrigin> {
    item.urls()
        .iter()
        .filter_map(|(_, u)| saved_origin(u.url.as_str()))
        .filter(|o| !o.is_http())
        .collect()
}

/// The trimmed `candidate` if it is a clean `https` URL on the registrable
/// domain of one of `origins`.
fn valid_stored(candidate: &str, origins: &[PageOrigin]) -> Option<String> {
    let url = candidate.trim();
    if url.len() > MAX_URL_LEN
        || url.chars().any(|c| c.is_whitespace() || c.is_control())
        || !url
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
    {
        return None;
    }
    let authority_end = url
        .get(8..)?
        .find(['/', '?', '#'])
        .map_or(url.len(), |i| i.saturating_add(8));
    if url.get(8..authority_end)?.contains('@') {
        return None;
    }
    let origin = saved_origin(url)?;
    if origin.is_http() || origin.is_loopback() {
        return None;
    }
    let domain = registrable_domain(&origin.host_str())?;
    origins
        .iter()
        .any(|o| registrable_domain(&o.host_str()).as_deref() == Some(domain.as_str()))
        .then(|| url.to_owned())
}

/// The vendored URL for `host` or its nearest parent domain.
fn quirk_for(host: &str) -> Option<&'static str> {
    let host = normalize(host)?;
    let table = quirks();
    candidates(&host).find_map(|d| table.get(d).map(String::as_str))
}

/// The vendored table, parsed once. Entries that are not a plain `https`
/// URL (or whose key is not a DNS name) are skipped, so the data cannot
/// hand the user anything else.
fn quirks() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| parse_quirks(QUIRKS_JSON))
}

fn parse_quirks(json: &str) -> HashMap<String, String> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(json) else {
        return HashMap::new();
    };
    map.into_iter()
        .filter_map(|(domain, url)| {
            let domain = normalize(&domain)?;
            let url = url.as_str()?;
            let clean = url.len() <= MAX_URL_LEN
                && url.starts_with("https://")
                && !url.chars().any(|c| c.is_whitespace() || c.is_control());
            clean.then(|| (domain, url.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use core::fmt::Write as _;

    use mosaic_core::model::{ItemType, LoginUrl};
    use mosaic_core::{DeviceId, Timestamp};
    use sha2::{Digest, Sha256};

    use super::*;

    fn login(urls: &[&str], stored: Option<&str>) -> Item {
        let d = DeviceId::new_v7().unwrap();
        let at = Timestamp::from_unix_millis(1);
        let mut item = Item::new(&ItemType::Login, d, at).unwrap();
        let mut e = item.edit(d, at);
        for u in urls {
            e.add_url(&LoginUrl::new(u)).unwrap();
        }
        if let Some(s) = stored {
            e.set_text(TextField::ChangePasswordUrl, s).unwrap();
        }
        item
    }

    fn resolved(urls: &[&str], stored: Option<&str>) -> Option<(String, ChangeUrlSource)> {
        resolve(&login(urls, stored)).map(|c| (c.url, c.source))
    }

    #[test]
    fn data_matches_recorded_sha256() {
        let hex = Sha256::digest(QUIRKS_JSON.as_bytes())
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            });
        assert_eq!(
            hex,
            "62838bb54084663e5dad60396c0008b78d13584955e1e9df0c5b93a0237cf2b0"
        );
        let json: Value = serde_json::from_str(QUIRKS_JSON).unwrap();
        let n = json.as_object().unwrap().len();
        assert_eq!(n, 805);
        assert_eq!(quirks().len(), n, "every vendored entry is usable");
    }

    #[test]
    fn stored_wins() {
        let r = resolved(
            &["https://example.com/login"],
            Some("https://accounts.example.com/security/password?x=1"),
        );
        assert_eq!(
            r,
            Some((
                "https://accounts.example.com/security/password?x=1".to_owned(),
                ChangeUrlSource::Stored
            ))
        );
        // Also when the quirks list knows the site.
        let r = resolved(&["https://23andme.com"], Some("https://23andme.com/mine"));
        assert_eq!(r.unwrap().1, ChangeUrlSource::Stored);
    }

    #[test]
    fn stored_must_be_related_https() {
        let urls = ["https://example.com/login"];
        for bad in [
            "http://example.com/pw",
            "https://evil.test/pw",
            "https://example.com.evil.test/pw",
            "https://example.com@evil.test/pw",
            "https://user@example.com/pw",
            "https://example.com/p w",
            "//example.com/pw",
            "example.com/pw",
            "javascript:alert(1)",
            "https://127.0.0.1/pw",
            "https://co.uk/pw",
            "",
        ] {
            let r = resolved(&urls, Some(bad)).unwrap();
            assert_eq!(r.1, ChangeUrlSource::WellKnown, "{bad:?} must be ignored");
            assert!(!is_valid_stored(&login(&urls, None), bad), "{bad:?}");
        }
        assert!(is_valid_stored(
            &login(&urls, None),
            " HTTPS://www.example.com/pw "
        ));
        // An http-only item has nothing to be related to.
        assert_eq!(
            resolved(&["http://example.com"], Some("https://example.com/pw")),
            None
        );
        assert_eq!(resolved(&[], Some("https://example.com/pw")), None);
    }

    #[test]
    fn quirks_lookup() {
        let r = resolved(&["https://23andme.com/login"], None).unwrap();
        assert_eq!(r.1, ChangeUrlSource::Quirks);
        assert_eq!(r.0, "https://you.23andme.com/user/edit/password/");
        // A subdomain of a listed domain finds the parent's entry.
        let r = resolved(&["https://www.500px.com"], None).unwrap();
        assert_eq!(r.1, ChangeUrlSource::Quirks);
        assert!(r.0.starts_with("https://"));
        // An http URL of a listed site does not count; the https one does.
        assert_eq!(
            resolved(&["http://23andme.com", "https://example.org"], None)
                .unwrap()
                .1,
            ChangeUrlSource::WellKnown
        );
        // The table only holds https URLs.
        assert!(quirks().values().all(|u| u.starts_with("https://")));
        let bad = r#"{"a.example":"http://a.example/p","b.example":"https://b.example/p q","c.example":"https://c.example/p","Bad Key":"https://x.example","d.example":1}"#;
        let t = parse_quirks(bad);
        assert_eq!(t.len(), 1);
        assert!(t.contains_key("c.example"));
        assert!(parse_quirks("not json").is_empty());
        assert!(parse_quirks("[]").is_empty());
    }

    #[test]
    fn well_known_fallback() {
        assert_eq!(
            resolved(&["https://unknown-site.example/login"], None),
            Some((
                "https://unknown-site.example/.well-known/change-password".to_owned(),
                ChangeUrlSource::WellKnown
            ))
        );
        // The first https URL, with its port; a bare host is https.
        assert_eq!(
            resolved(&["http://old.example", "unknown-site.example:8443/x"], None),
            Some((
                "https://unknown-site.example:8443/.well-known/change-password".to_owned(),
                ChangeUrlSource::WellKnown
            ))
        );
        assert_eq!(resolved(&[], None), None);
        assert_eq!(ChangeUrlSource::WellKnown.as_str(), "wellKnown");
    }
}
