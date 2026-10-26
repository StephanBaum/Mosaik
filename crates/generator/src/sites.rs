//! Per-site password rules from Apple's Password Manager Resources quirks
//! (`data/password-rules.json`, MIT, Copyright Apple Inc.; see
//! `data/README.md` and the license next to it).
//!
//! # Lookup
//!
//! [`rules_for_domain`] takes a host name (from a URL the caller already
//! parsed) and normalizes it: ASCII lowercase, one trailing `.` removed.
//! Internationalized names must be passed in their ASCII (punycode) form.
//! Anything that is not a plain DNS name (`a-z`, `0-9`, `-`, non-empty
//! labels) finds nothing.
//!
//! It then tries the host itself, and then each parent domain obtained by
//! removing the leftmost label, down to two labels: for
//! `login.secure.example.co.uk` it tries `login.secure.example.co.uk`,
//! `secure.example.co.uk`, `example.co.uk` and `co.uk`. The most specific
//! entry wins. This follows the upstream semantics ("a rule for
//! `example.com` matches `example.com` and `*.example.com`"). Entries marked
//! `"exact-domain-match-only": true` match only the host itself. There is
//! no Public Suffix List: a public suffix such as `co.uk` is tried like
//! any other parent, which is harmless because the data has no entries for
//! public suffixes (a test checks that every key has at least two labels
//! and that none is a common multi-label public suffix). A host whose last
//! label is all digits (an IPv4 address; no top-level domain is numeric)
//! matches only exactly.

use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::rules::PasswordRules;

const RULES_JSON: &str = include_str!("../data/password-rules.json");

static TABLE: OnceLock<Option<Map<String, Value>>> = OnceLock::new();

/// The parsed quirks table. `None` only if the embedded file is not a JSON
/// object (a broken build); lookups then find nothing.
fn table() -> Option<&'static Map<String, Value>> {
    TABLE
        .get_or_init(|| match serde_json::from_str::<Value>(RULES_JSON) {
            Ok(Value::Object(map)) => Some(map),
            _ => None,
        })
        .as_ref()
}

/// One quirks entry: its key, its rule string, and whether it is limited
/// to the exact domain.
fn entry<'t>(table: &'t Map<String, Value>, domain: &str) -> Option<(&'t str, &'t str, bool)> {
    let (key, value) = table.get_key_value(domain)?;
    let rules = value.get("password-rules")?.as_str()?;
    let exact_only = value
        .get("exact-domain-match-only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some((key.as_str(), rules, exact_only))
}

/// The most specific entry of `table` for the normalized `host`:
/// `(key, rule string)`.
fn lookup<'t>(table: &'t Map<String, Value>, host: &str) -> Option<(&'t str, &'t str)> {
    let mut candidate = host;
    loop {
        if let Some((key, rules, exact_only)) = entry(table, candidate)
            && (!exact_only || candidate == host)
        {
            return Some((key, rules));
        }
        if is_numeric(host) {
            return None;
        }
        let (_, parent) = candidate.split_once('.')?;
        if !parent.contains('.') {
            return None;
        }
        candidate = parent;
    }
}

/// Every `(domain, rule string)` pair in the embedded data.
#[cfg(test)]
pub(crate) fn entries() -> impl Iterator<Item = (&'static str, &'static str)> {
    table().into_iter().flatten().filter_map(|(domain, value)| {
        let rules = value.get("password-rules")?.as_str()?;
        Some((domain.as_str(), rules))
    })
}

/// The longest host name accepted, per RFC 1035.
const MAX_HOST_LEN: usize = 253;

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

/// Whether `host` ends in an all-digit label, as IPv4 addresses do. Such a
/// host has no parent domains.
fn is_numeric(host: &str) -> bool {
    host.rsplit('.')
        .next()
        .is_some_and(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_digit()))
}

/// The rules that apply to a site, and the quirks key they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteRules {
    domain: &'static str,
    rules: PasswordRules,
}

impl SiteRules {
    /// The quirks entry that matched (the host itself or a parent domain).
    #[must_use]
    pub fn domain(&self) -> &str {
        self.domain
    }

    /// The parsed rules.
    #[must_use]
    pub fn rules(&self) -> &PasswordRules {
        &self.rules
    }

    /// The parsed rules, by value.
    #[must_use]
    pub fn into_rules(self) -> PasswordRules {
        self.rules
    }
}

/// Looks up the vendored password rules for `host` (see the module
/// documentation for normalization and the fallback to parent domains).
#[must_use]
pub fn rules_for_domain(host: &str) -> Option<SiteRules> {
    let host = normalize(host)?;
    let (domain, rules) = lookup(table()?, &host)?;
    Some(SiteRules {
        domain,
        rules: PasswordRules::parse(rules),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn data_loads() {
        assert!(table().is_some());
        assert_eq!(entries().count(), 436);
    }

    #[test]
    fn every_key_is_a_normalized_registrable_style_domain() {
        // Common multi-label public suffixes that must never be keys, since
        // the fallback would otherwise apply them to unrelated sites.
        let public_suffixes = [
            "co.uk",
            "org.uk",
            "gov.uk",
            "ac.uk",
            "com.au",
            "net.au",
            "co.jp",
            "co.nz",
            "com.br",
            "com.cn",
            "co.in",
            "co.za",
            "com.mx",
            "github.io",
            "blogspot.com",
            "herokuapp.com",
        ];
        for (domain, _) in entries() {
            assert_eq!(normalize(domain).as_deref(), Some(domain), "{domain}");
            assert!(domain.contains('.'), "{domain} has one label");
            assert!(!public_suffixes.contains(&domain), "{domain}");
        }
    }

    #[test]
    fn every_entry_parses_and_generates() {
        let mut rng = SplitMix64::new(0x5175_6972_6b73);
        for (domain, text) in entries() {
            let rules = PasswordRules::parse(text);
            // Canonical display parses back to the same rules.
            assert_eq!(PasswordRules::parse(&rules.to_string()), rules, "{domain}");
            for preferred in [1, 8, 20, 64, 128] {
                for _ in 0..5 {
                    let p = rules
                        .generate_with(preferred, &mut rng)
                        .unwrap_or_else(|e| panic!("{domain}: {e}"));
                    assert!(
                        rules.is_satisfied_by(p.expose_secret()),
                        "{domain}: generated password violates {text}"
                    );
                    assert!(!p.expose_secret().contains(' '), "{domain}");
                }
            }
        }
    }

    #[test]
    fn exact_host_and_parent_fallback() {
        let r = rules_for_domain("163.com").unwrap();
        assert_eq!(r.domain(), "163.com");
        assert_eq!(r.rules().min_length(), Some(6));
        assert_eq!(r.rules().max_length(), Some(16));
        let sub = rules_for_domain("mail.login.163.com").unwrap();
        assert_eq!(sub.domain(), "163.com");
        assert_eq!(sub.into_rules(), r.into_rules());
        // A more specific key wins over its parent.
        let gov = rules_for_domain("www.access.service.gov.uk").unwrap();
        assert_eq!(gov.domain(), "access.service.gov.uk");
    }

    #[test]
    fn normalization() {
        assert_eq!(rules_for_domain("163.COM.").unwrap().domain(), "163.com");
        for bad in [
            "",
            ".",
            "163.com..",
            ".163.com",
            "a..163.com",
            "163.com:443",
            "https://163.com",
            "user@163.com",
            "bücher.163.com",
            "163 .com",
        ] {
            assert!(rules_for_domain(bad).is_none(), "{bad:?}");
        }
        assert!(rules_for_domain(&"a.".repeat(130)).is_none(), "too long");
        assert!(rules_for_domain("com").is_none());
        assert!(rules_for_domain("example.invalid").is_none());
        assert!(rules_for_domain("not-listed.example.com").is_none());
    }

    #[test]
    fn ip_addresses_match_only_exactly() {
        assert!(is_numeric("10.0.0.1"));
        assert!(is_numeric("10.0.0"));
        assert!(!is_numeric("1.0.0.a"));
        assert!(!is_numeric("163.com"));
        assert!(!is_numeric(""));
        assert!(rules_for_domain("10.0.163.com").is_some(), "not an IP");
        assert!(rules_for_domain("192.168.0.1").is_none());
    }

    #[test]
    fn exact_domain_match_only_is_respected() {
        // The vendored data has no such entry today, so check the logic on
        // a table built for the test.
        let table: Map<String, Value> = serde_json::from_str(
            r#"{
                "exact.example.com": {"password-rules": "minlength: 9;", "exact-domain-match-only": true},
                "example.com": {"password-rules": "minlength: 7;"},
                "other.example.org": {"password-rules": "minlength: 5;", "exact-domain-match-only": false},
                "broken.example.org": {"no-rules": true},
                "1.2.3.4": {"password-rules": "minlength: 4;"}
            }"#,
        )
        .unwrap();
        assert_eq!(
            lookup(&table, "exact.example.com"),
            Some(("exact.example.com", "minlength: 9;"))
        );
        // A subdomain of an exact-only entry falls through to the parent.
        assert_eq!(
            lookup(&table, "www.exact.example.com"),
            Some(("example.com", "minlength: 7;"))
        );
        assert_eq!(
            lookup(&table, "a.other.example.org"),
            Some(("other.example.org", "minlength: 5;"))
        );
        // An entry without a rule string is skipped.
        assert_eq!(lookup(&table, "broken.example.org"), None);
        assert_eq!(
            lookup(&table, "1.2.3.4"),
            Some(("1.2.3.4", "minlength: 4;"))
        );
        assert_eq!(
            lookup(&table, "0.1.2.3.4"),
            None,
            "no suffix matching for numeric hosts"
        );
        assert_eq!(lookup(&table, "x"), None);

        let flagged = table_flagged();
        assert!(flagged.is_empty(), "update the tests: {flagged:?}");
    }

    fn table_flagged() -> Vec<&'static str> {
        table()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.get("exact-domain-match-only").is_some())
            .map(|(k, _)| k.as_str())
            .collect()
    }
}
