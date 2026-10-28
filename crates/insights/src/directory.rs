//! Sites that support one-time codes (TOTP), from the 2FA Directory
//! (<https://2fa.directory>, `data/2fa-directory-totp.json`, MIT, Copyright
//! 2factorauth and Contributors and Josh Davis; see `data/README.md` and the
//! license next to it). The data is embedded at build time; nothing is
//! fetched at run time. The directory asks for attribution where its data is
//! visible to users: the dashboard shows `insights.attribution.twoFactor`
//! (ADR-0021).
//!
//! # Lookup
//!
//! [`totp_site`] takes a host (from [`crate::url::parse`]) and normalizes it
//! like `mosaic_generator::rules_for_domain` does: ASCII lowercase, one
//! trailing `.` removed, plain DNS labels only (`a-z`, `0-9`, `-`).
//! Internationalized names must be in their ASCII (punycode) form. It tries
//! the host and then each parent domain down to two labels; the most specific
//! entry wins. A host ending in an all-digit label (an IPv4 address) matches
//! only exactly.
//!
//! Some directory domains are public suffixes or hosting platforms, under
//! which unrelated parties get their own names (`gov.uk`, `myfritz.net`,
//! `someblog.substack.com`). Such a domain matches only the host itself and
//! its `www.` host: [`EXACT_ONLY`] lists them. The list is checked against
//! the Public Suffix List (ICANN and private sections) whenever the data is
//! updated (see `data/README.md`); a test checks that every entry is in the
//! data and every two-label domain in the data whose first label is a
//! common second-level label (`co`, `com`, `gov`, `ac`, ...).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

const TOTP_JSON: &str = include_str!("../data/2fa-directory-totp.json");

/// Directory domains under which other parties get their own names: these
/// match only exactly (or with `www.`), never for another subdomain.
///
/// - On the Public Suffix List (checked 2026-10-28 with the list in
///   `tldts` 7.4.19): `carrd.co`, `gov.uk` (ICANN), `myfritz.net`,
///   `omg.lol`, `pythonanywhere.com`, `square.site` (private section).
/// - Not on the list, but hosting user sites on subdomains: `squarespace.com`,
///   `substack.com`, `tumblr.com`, `weebly.com`, `wordpress.com`.
pub const EXACT_ONLY: &[&str] = &[
    "carrd.co",
    "gov.uk",
    "myfritz.net",
    "omg.lol",
    "pythonanywhere.com",
    "square.site",
    "squarespace.com",
    "substack.com",
    "tumblr.com",
    "weebly.com",
    "wordpress.com",
];

/// Longest host name accepted, per RFC 1035.
const MAX_HOST_LEN: usize = 253;

/// One directory entry: a service that supports TOTP. A cheap handle into
/// the embedded directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TotpSite {
    entry: &'static Entry,
}

impl TotpSite {
    /// The service name, as the directory spells it.
    #[must_use]
    pub fn name(&self) -> &'static str {
        &self.entry.name
    }

    /// The entry's main domain.
    #[must_use]
    pub fn domain(&self) -> &'static str {
        &self.entry.domain
    }

    /// The service's own page about setting up two-factor authentication
    /// (`https://` only), if the directory has one. Open it only on a user
    /// action.
    #[must_use]
    pub fn documentation(&self) -> Option<&'static str> {
        self.entry.documentation.as_deref()
    }

    /// The main domain and every additional domain the directory lists for
    /// this service, main domain first.
    pub fn domains(&self) -> impl Iterator<Item = &'static str> + use<> {
        let entry: &'static Entry = self.entry;
        std::iter::once(entry.domain.as_str()).chain(entry.additional.iter().map(String::as_str))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    name: String,
    domain: String,
    additional: Vec<String>,
    documentation: Option<String>,
}

struct Directory {
    sites: Vec<Entry>,
    /// Every domain and additional domain → index into `sites`.
    by_domain: HashMap<String, usize>,
    /// Lowercase service names → index into `sites`.
    by_name: HashMap<String, usize>,
}

impl Directory {
    fn site(&'static self, index: usize) -> Option<TotpSite> {
        self.sites.get(index).map(|entry| TotpSite { entry })
    }
}

static DIRECTORY: OnceLock<Option<Directory>> = OnceLock::new();

/// The parsed directory; `None` only if the embedded file is malformed (a
/// broken build, caught by the tests). Lookups then find nothing.
fn directory() -> Option<&'static Directory> {
    DIRECTORY.get_or_init(|| parse(TOTP_JSON)).as_ref()
}

/// Parses the v3 API format: `[[name, { domain, additional-domains,
/// documentation, tfa, ... }], ...]`. Entries without `totp` in `tfa` or
/// without a usable domain are skipped. The first entry for a domain wins.
fn parse(json: &str) -> Option<Directory> {
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(json) else {
        return None;
    };
    let mut sites = Vec::with_capacity(entries.len());
    let mut by_domain = HashMap::new();
    let mut by_name = HashMap::new();
    for entry in &entries {
        let Some((name, fields)) = entry.as_array().and_then(|e| match e.as_slice() {
            [Value::String(name), Value::Object(fields)] => Some((name, fields)),
            _ => None,
        }) else {
            continue;
        };
        let supports_totp = fields
            .get("tfa")
            .and_then(Value::as_array)
            .is_some_and(|tfa| tfa.iter().any(|m| m.as_str() == Some("totp")));
        let Some(domain) = fields
            .get("domain")
            .and_then(Value::as_str)
            .and_then(normalize)
        else {
            continue;
        };
        if !supports_totp {
            continue;
        }
        let index = sites.len();
        let additional = fields
            .get("additional-domains")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(normalize)
            .collect::<Vec<_>>();
        for d in std::iter::once(&domain).chain(&additional) {
            by_domain.entry(d.clone()).or_insert(index);
        }
        by_name.entry(name.to_lowercase()).or_insert(index);
        sites.push(Entry {
            name: name.clone(),
            domain,
            additional,
            documentation: fields
                .get("documentation")
                .and_then(Value::as_str)
                .filter(|d| d.starts_with("https://"))
                .map(str::to_owned),
        });
    }
    Some(Directory {
        sites,
        by_domain,
        by_name,
    })
}

/// Lowercases and validates `host`; `None` if it is not a DNS name.
fn normalize(host: &str) -> Option<String> {
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() || host.len() > MAX_HOST_LEN {
        return None;
    }
    let host = host.to_ascii_lowercase();
    let valid = host.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    valid.then_some(host)
}

fn is_numeric(host: &str) -> bool {
    host.rsplit('.')
        .next()
        .is_some_and(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_digit()))
}

/// The directory entry for `host` or its closest listed parent domain (see
/// the module documentation).
#[must_use]
pub fn totp_site(host: &str) -> Option<TotpSite> {
    let dir = directory()?;
    let host = normalize(host)?;
    let mut candidate = host.as_str();
    loop {
        if let Some(&index) = dir.by_domain.get(candidate)
            && (!EXACT_ONLY.contains(&candidate)
                || candidate == host
                || host.strip_prefix("www.") == Some(candidate))
        {
            return dir.site(index);
        }
        if is_numeric(&host) {
            return None;
        }
        let (_, parent) = candidate.split_once('.')?;
        if !parent.contains('.') {
            return None;
        }
        candidate = parent;
    }
}

/// The directory entries an OTP issuer names, compared case-insensitively:
/// the entry with that service name, the entry for that host (as
/// [`totp_site`]), and every entry whose domain starts with that label
/// (`github` names `github.com`). An entry stands for all its additional
/// domains: `Amazon` names `amazon.com`, whose entry also lists
/// `amazon.de` and `amazon.co.uk`.
#[must_use]
pub fn totp_sites_for_issuer(issuer: &str) -> Vec<TotpSite> {
    let Some(dir) = directory() else {
        return Vec::new();
    };
    let issuer = issuer.trim().to_lowercase();
    if issuer.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<TotpSite> = dir
        .by_name
        .get(&issuer)
        .and_then(|&index| dir.site(index))
        .into_iter()
        .chain(totp_site(&issuer))
        .collect();
    for (index, entry) in dir.sites.iter().enumerate() {
        if entry.domain.split('.').next() == Some(issuer.as_str())
            && let Some(site) = dir.site(index)
        {
            out.push(site);
        }
    }
    out.sort_by_key(TotpSite::domain);
    out.dedup();
    out
}

/// How many entries the embedded directory has.
#[must_use]
pub fn site_count() -> usize {
    directory().map_or(0, |d| d.sites.len())
}

#[cfg(test)]
mod tests {
    use core::fmt::Write as _;

    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn data_loads_and_matches_the_recorded_sha256() {
        let hex = Sha256::digest(TOTP_JSON.as_bytes())
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            });
        assert_eq!(
            hex,
            "a4652045def5f9e6c0e81034eb0f3f694909d84118edc7566b3225d293bd2746"
        );
        assert_eq!(site_count(), 1294);
        let dir = directory().unwrap();
        assert_eq!(dir.by_domain.len(), 1653, "domains, duplicates merged");
    }

    #[test]
    fn exact_and_parent_matches() {
        let gh = totp_site("github.com").unwrap();
        assert_eq!(gh.domain(), "github.com");
        assert_eq!(gh.name(), "GitHub");
        assert!(gh.documentation().unwrap().starts_with("https://"));
        assert_eq!(totp_site("GitHub.COM.").unwrap().domain(), "github.com");
        assert_eq!(totp_site("gist.github.com").unwrap().domain(), "github.com");
        assert_eq!(
            totp_site("a.b.c.github.com").unwrap().domain(),
            "github.com"
        );
        // An additional domain maps to its entry.
        assert_eq!(totp_site("1password.eu").unwrap().domain(), "1password.com");
        let one = totp_site("1password.eu").unwrap();
        assert_eq!(
            one.domains().collect::<Vec<_>>(),
            ["1password.com", "1password.ca", "1password.eu"]
        );
        // The most specific entry wins.
        let mail = totp_site("mail.google.com").unwrap();
        assert_eq!(mail.domain(), "mail.google.com");
        assert_eq!(
            totp_site("www.gmail.com").unwrap().domain(),
            "mail.google.com"
        );
        // google.com itself has no TOTP entry, so its other hosts find none.
        assert_eq!(totp_site("accounts.google.com"), None);
        // Additional domains report the entry's main domain.
        assert_eq!(totp_site("www.amazon.de").unwrap().domain(), "amazon.com");
    }

    #[test]
    fn no_unrelated_matches() {
        for host in [
            "evilgithub.com",
            "github.com.evil.net",
            "github",
            "com",
            "",
            ".",
            "example.invalid",
            "1.2.3.4",
            "gith ub.com",
            "bücher.de",
        ] {
            assert_eq!(totp_site(host), None, "{host:?}");
        }
        assert_eq!(totp_site(&"a".repeat(MAX_HOST_LEN + 1)), None);
    }

    #[test]
    fn public_suffix_entries_match_only_exactly() {
        assert_eq!(totp_site("gov.uk").unwrap().domain(), "gov.uk");
        assert_eq!(totp_site("my-council.gov.uk"), None);
        // A listed subdomain of a public suffix still matches its own tree.
        assert_eq!(
            totp_site("www.tax.service.gov.uk").unwrap().domain(),
            "access.service.gov.uk"
        );
        assert_eq!(
            totp_site("wordpress.com").unwrap().domain(),
            "wordpress.com"
        );
        assert_eq!(totp_site("someblog.wordpress.com"), None);
        assert_eq!(
            totp_site("www.wordpress.com").unwrap().domain(),
            "wordpress.com"
        );
        // Regression: user subdomains of hosting platforms mapped to the
        // platform and produced a misleading "set up 2FA" hint.
        for host in [
            "someblog.substack.com",
            "name.omg.lol",
            "blog.tumblr.com",
            "shop.squarespace.com",
            "site.weebly.com",
            "fritzbox123.myfritz.net",
            "me.carrd.co",
            "user.pythonanywhere.com",
            "store.square.site",
        ] {
            assert_eq!(totp_site(host), None, "{host}");
        }
        assert_eq!(totp_site("www.tumblr.com").unwrap().domain(), "tumblr.com");
        assert_eq!(totp_site("substack.com").unwrap().domain(), "substack.com");
    }

    #[test]
    fn every_exact_only_domain_is_in_the_data() {
        let dir = directory().unwrap();
        for domain in EXACT_ONLY {
            assert!(dir.by_domain.contains_key(*domain), "{domain}");
        }
    }

    #[test]
    fn every_public_suffix_like_domain_is_exact_only() {
        let second_level = [
            "co", "com", "gov", "govt", "ac", "edu", "org", "net", "ne", "or", "go", "gob", "nhs",
            "ltd", "plc", "me",
        ];
        let dir = directory().unwrap();
        for domain in dir.by_domain.keys() {
            let labels: Vec<&str> = domain.split('.').collect();
            assert!(labels.len() >= 2, "{domain}");
            if labels.len() == 2 && second_level.contains(&labels[0]) {
                assert!(
                    EXACT_ONLY.contains(&domain.as_str()),
                    "{domain} must be exact-only"
                );
            }
        }
    }

    #[test]
    fn issuers() {
        let domains = |issuer: &str| -> Vec<&str> {
            totp_sites_for_issuer(issuer)
                .iter()
                .map(TotpSite::domain)
                .collect()
        };
        assert_eq!(domains("GitHub"), ["github.com"]);
        assert_eq!(domains(" github.com "), ["github.com"]);
        assert_eq!(domains("github"), ["github.com"]);
        let amazon = domains("Amazon");
        assert_eq!(amazon, ["amazon.com"]);
        assert_eq!(domains(""), [] as [&str; 0]);
        assert_eq!(domains("no such issuer xyz"), [] as [&str; 0]);
    }

    #[test]
    fn malformed_data_yields_nothing() {
        assert!(parse("{}").is_none());
        assert!(parse("not json").is_none());
        let d = parse(
            r#"[["A",{"domain":"a.example","tfa":["sms"]}],
               ["B",{"domain":"Bad Domain","tfa":["totp"]}],
               ["C",{"domain":"c.example","tfa":["totp"],"documentation":"http://x"}],
               ["C2",{"domain":"c.example","tfa":["totp"]}],
               "junk", ["D"]]"#,
        )
        .unwrap();
        assert_eq!(d.sites.len(), 2);
        assert_eq!(d.sites[0].documentation, None);
        assert_eq!(d.by_domain.get("c.example"), Some(&0));
        assert!(!d.by_domain.contains_key("a.example"));
    }
}
