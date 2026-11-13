//! Origin matching for the browser extension (brief §4.1 to §4.3, ADR-0037).
//!
//! The desktop decides every fill; this crate holds the rules. Nothing here
//! touches secrets.
//!
//! - [`PageOrigin`]: the origin a browser reports for a page or frame,
//!   parsed with the WHATWG URL parser and required to be in its canonical
//!   serialization (no path, query, userinfo or trailing slash).
//! - [`saved_origin`]: the origin of a URL a user typed into a login.
//!   A URL without a scheme is `https`; userinfo is ignored.
//! - [`fill_allowed`]: exact origin by default; the registrable domain
//!   (eTLD+1 from the compiled-in public suffix list) only for a URL saved
//!   with [`UrlMatch::Etld1`], with equal schemes and any port. A saved
//!   `http` URL never matches an `https` page. A match on a non-loopback
//!   `http` page needs the URL's `allow_http` flag ([`Refusal::Insecure`]).
//! - [`idn_unicode`] and [`lookalike_of`]: warnings, never matches.
//! - [`webauthn_rp_id`]: the WebAuthn relying party ID a page may use
//!   (https only, a registrable-domain suffix of the host, ADR-0040).
//! - [`canonical_rp_id`]: an rp ID read from a file (an import) brought into
//!   the one canonical form [`webauthn_rp_id`] accepts.
//!
//! Mobile (M10, ADR-0092): [`app`] (Android app URLs bound to a signing
//! certificate), [`android_browsers`] (the vendored allowlist of browsers that
//! may pass a web origin), [`target`] (what an Android or iOS autofill request
//! is for), [`fields`] (which fields of a view tree to fill, never those of a
//! cross-origin frame) and [`assetlinks`] (Digital Asset Links parsing).
//!
//! The public suffix list is a build-time snapshot of the `psl` crate.
#![forbid(unsafe_code)]

pub mod android_browsers;
pub mod app;
pub mod assetlinks;
pub mod fields;
pub mod target;

use mosaic_core::model::{LoginUrl, UrlMatch};
use url::{Host, Origin, Url};

/// The longest origin string [`PageOrigin::parse`] accepts, in bytes.
pub const MAX_ORIGIN_LEN: usize = 2048;

/// The longest saved URL [`saved_origin`] looks at, in bytes.
pub const MAX_SAVED_URL_LEN: usize = 8192;

/// Why a string is not a page origin.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OriginError {
    /// Longer than [`MAX_ORIGIN_LEN`].
    #[error("origin too long")]
    TooLong,
    /// Not parseable as a URL with a host.
    #[error("not a valid origin")]
    Malformed,
    /// The scheme is not `http` or `https`.
    #[error("unsupported scheme")]
    Scheme,
    /// A URL, but not the canonical serialization of its own origin (it has
    /// a path, query, fragment, userinfo, uppercase letters or a default
    /// port written out).
    #[error("not a canonical origin")]
    NotCanonical,
}

/// `http` or `https`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    const fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

/// The origin of a page or frame: scheme (`http` or `https`), host and port.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PageOrigin {
    scheme: Scheme,
    host: Host<String>,
    port: u16,
}

impl PageOrigin {
    /// Parses an origin as a browser serializes it (`location.origin`).
    ///
    /// # Errors
    /// [`OriginError`] if `s` is longer than [`MAX_ORIGIN_LEN`], is not a
    /// URL, is not `http` or `https`, or is not exactly the ASCII
    /// serialization of its own origin (so `https://a/`, `null`, `file:`,
    /// `chrome-extension:` and `data:` are all refused).
    pub fn parse(s: &str) -> Result<Self, OriginError> {
        if s.len() > MAX_ORIGIN_LEN {
            return Err(OriginError::TooLong);
        }
        let url = Url::parse(s).map_err(|_| OriginError::Malformed)?;
        let origin = Self::from_url(&url)?;
        if url.origin().ascii_serialization() != s {
            return Err(OriginError::NotCanonical);
        }
        Ok(origin)
    }

    fn from_url(url: &Url) -> Result<Self, OriginError> {
        let scheme = Scheme::parse(url.scheme()).ok_or(OriginError::Scheme)?;
        let Origin::Tuple(_, host, port) = url.origin() else {
            return Err(OriginError::Malformed);
        };
        Ok(Self { scheme, host, port })
    }

    /// The canonical origin string: `scheme://host[:port]`, the port only
    /// when it is not the scheme's default.
    #[must_use]
    pub fn serialize(&self) -> String {
        let mut out = format!("{}://{}", self.scheme.as_str(), self.host);
        if self.port != self.scheme.default_port() {
            out.push(':');
            out.push_str(&self.port.to_string());
        }
        out
    }

    /// The host as serialized in a URL (punycode for IDNs, IPv6 in square
    /// brackets).
    #[must_use]
    pub fn host_str(&self) -> String {
        self.host.to_string()
    }

    /// Whether the scheme is `http`.
    #[must_use]
    pub fn is_http(&self) -> bool {
        self.scheme == Scheme::Http
    }

    /// Whether the host is `localhost`, `*.localhost`, in `127.0.0.0/8` or
    /// `::1`.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        match &self.host {
            Host::Domain(d) => {
                let d = d.strip_suffix('.').unwrap_or(d);
                d == "localhost" || d.ends_with(".localhost")
            }
            Host::Ipv4(ip) => ip.is_loopback(),
            Host::Ipv6(ip) => ip.is_loopback(),
        }
    }
}

/// The origin of a URL as saved in a login: trimmed; a URL without
/// `scheme://` is read as `https`; userinfo, path, query and fragment are
/// dropped. `None` unless the result is an `http` or `https` origin.
#[must_use]
pub fn saved_origin(url: &str) -> Option<PageOrigin> {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_SAVED_URL_LEN {
        return None;
    }
    let parsed = if url.contains("://") {
        Url::parse(url)
    } else {
        Url::parse(&format!("https://{url}"))
    }
    .ok()?;
    PageOrigin::from_url(&parsed).ok()
}

/// The registrable domain (eTLD+1) of `host` from the public suffix list.
/// `None` for IP addresses, single labels, bare public suffixes and hosts
/// with a trailing dot.
#[must_use]
pub fn registrable_domain(host: &str) -> Option<String> {
    let Ok(Host::Domain(domain)) = Host::parse(host) else {
        return None;
    };
    if domain.ends_with('.') {
        return None;
    }
    psl::domain_str(&domain).map(str::to_owned)
}

/// Why a WebAuthn relying party ID is refused for an origin.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RpIdError {
    /// The origin is not `https`.
    #[error("insecure origin")]
    InsecureOrigin,
    /// The host is an IP address or loopback, not a domain.
    #[error("not a domain")]
    NotADomain,
    /// The rp ID is empty, too long, or not in canonical form (lowercase
    /// ASCII, punycode, no trailing dot).
    #[error("rp id not canonical")]
    NotCanonical,
    /// The rp ID is neither the host nor a dot-suffix of it, or it is above the
    /// host's registrable domain.
    #[error("rp id not a suffix of the origin")]
    NotASuffix,
    /// The rp ID is a public suffix (it has no registrable domain).
    #[error("rp id is a public suffix")]
    PublicSuffix,
}

/// The longest relying party ID [`webauthn_rp_id`] accepts, in bytes.
pub const MAX_RP_ID_LEN: usize = 253;

/// Whether `rp` is a canonical domain name: not empty, at most
/// [`MAX_RP_ID_LEN`] bytes, no empty label (no leading, trailing or doubled
/// dot), and `Host::parse(rp)` gives the same domain (so no U-labels, IP
/// addresses, URLs, spaces or uppercase).
fn is_canonical_domain(rp: &str) -> bool {
    !rp.is_empty()
        && rp.len() <= MAX_RP_ID_LEN
        && rp.split('.').all(|label| !label.is_empty())
        && Host::parse(rp) == Ok(Host::Domain(rp.to_owned()))
}

/// Whether every label is a DNS label: ASCII letters, digits and hyphens, no
/// leading or trailing hyphen, at most 63 bytes (M10-G3-6). The WHATWG host
/// parser lets `*`, `!`, `$` and the like through; a DNS name never has them.
fn is_ldh_name(rp: &str) -> bool {
    rp.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

/// The canonical relying party ID for an rp ID read from a file (an import
/// from another password manager), or `None` if it cannot be one: ASCII
/// letters are lowercased and one trailing dot is removed; the result must
/// be a canonical DNS name (letters, digits and hyphens per label; the
/// check [`webauthn_rp_id`] applies: no U-labels, IP addresses, URLs, spaces or direction-changing characters, at
/// most [`MAX_RP_ID_LEN`] bytes) with a registrable domain in the public
/// suffix list (so `com` is refused).
#[must_use]
pub fn canonical_rp_id(raw: &str) -> Option<String> {
    let prepared = raw.to_ascii_lowercase();
    let rp = prepared.strip_suffix('.').unwrap_or(&prepared);
    (is_canonical_domain(rp) && is_ldh_name(rp) && registrable_domain(rp).is_some())
        .then(|| rp.to_owned())
}

/// The WebAuthn relying party ID for a request from `origin` (WebAuthn L3
/// 5.1.3 step 8 and 5.1.4.1 step 7), restricted (ADR-0040): `https` only;
/// the host must be a domain, not loopback; the rp ID defaults to the host;
/// a given rp ID must be canonical (`Host::parse(rp)` is the same domain,
/// DNS labels only (letters, digits, hyphens), no trailing dot, at most [`MAX_RP_ID_LEN`] bytes), equal the host or be a
/// dot-suffix of it, have a registrable domain in the public suffix list, and
/// lie at or under the host's registrable domain (so a tenant of a wildcard
/// public suffix such as `*.platformsh.site` cannot claim the parent).
///
/// # Errors
/// [`RpIdError`], checked in this order: `InsecureOrigin`, `NotADomain`,
/// `NotCanonical`, `NotASuffix` (not the host or a dot-suffix of it),
/// `PublicSuffix` (the rp ID has no registrable domain), `NotASuffix` again
/// (above the host's registrable domain).
pub fn webauthn_rp_id(origin: &PageOrigin, requested: Option<&str>) -> Result<String, RpIdError> {
    if origin.is_http() {
        return Err(RpIdError::InsecureOrigin);
    }
    let Host::Domain(host) = &origin.host else {
        return Err(RpIdError::NotADomain);
    };
    if origin.is_loopback() {
        return Err(RpIdError::NotADomain);
    }
    let rp = requested.unwrap_or(host.as_str());
    if !is_canonical_domain(rp) || !is_ldh_name(rp) {
        return Err(RpIdError::NotCanonical);
    }
    let is_suffix = host == rp
        || host
            .strip_suffix(rp)
            .is_some_and(|rest| rest.ends_with('.'));
    if !is_suffix {
        return Err(RpIdError::NotASuffix);
    }
    if registrable_domain(rp).is_none() {
        return Err(RpIdError::PublicSuffix);
    }
    // The rp ID must also lie at or under the host's own registrable domain:
    // under a wildcard public suffix (`*.kawasaki.jp`, `*.platformsh.site`)
    // the parent is registrable by itself, but it belongs to the platform,
    // not to the tenant host below it.
    let Some(site) = registrable_domain(host) else {
        return Err(RpIdError::PublicSuffix);
    };
    if rp != site && !rp.ends_with(&format!(".{site}")) {
        return Err(RpIdError::NotASuffix);
    }
    Ok(rp.to_owned())
}

/// How a saved URL matched a page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchKind {
    /// Scheme, host and port are equal.
    ExactOrigin,
    /// The registrable domains are equal (opt-in per URL).
    Etld1,
}

/// Why a saved URL does not allow a fill.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// The URL does not match the page.
    NoMatch,
    /// The URL matches an `http` page that is not loopback, and the URL
    /// does not carry `allow_http`.
    Insecure,
}

/// Whether the saved URL allows filling a page at `page`.
///
/// 1. Exact: scheme, host and port equal.
/// 2. Otherwise, for a URL saved as [`UrlMatch::Etld1`]: equal schemes and
///    equal registrable domains (both `Some`); the port is ignored.
/// 3. Otherwise [`Refusal::NoMatch`].
/// 4. A match on a non-loopback `http` page without `allow_http` is
///    [`Refusal::Insecure`].
///
/// A saved `http` URL never matches an `https` page (and the reverse),
/// because schemes must be equal in both rules.
///
/// # Errors
/// [`Refusal`] as above.
pub fn fill_allowed(saved: &LoginUrl, page: &PageOrigin) -> Result<MatchKind, Refusal> {
    fill_allowed_with(saved, page, HttpPolicy::LoopbackExempt)
}

/// How an `http` page on a loopback address is treated by [`fill_allowed_with`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HttpPolicy {
    /// Loopback `http` needs no `allow_http` (ADR-0037: on a desktop, other
    /// processes of the same user are out of scope).
    LoopbackExempt,
    /// Loopback `http` needs `allow_http` like any other `http` page. On a
    /// phone any installed app can listen on `127.0.0.1` and serve a fake form
    /// (ADR-0092 amendment, M10-G3-4).
    LoopbackNeedsAllowHttp,
}

/// [`fill_allowed`] with the loopback rule of `policy`.
///
/// # Errors
/// [`Refusal`] as for [`fill_allowed`].
pub fn fill_allowed_with(
    saved: &LoginUrl,
    page: &PageOrigin,
    policy: HttpPolicy,
) -> Result<MatchKind, Refusal> {
    let origin = saved_origin(saved.url.as_str()).ok_or(Refusal::NoMatch)?;
    let kind = if origin == *page {
        MatchKind::ExactOrigin
    } else if saved.match_mode == UrlMatch::Etld1 && same_registrable_domain(&origin, page) {
        MatchKind::Etld1
    } else {
        return Err(Refusal::NoMatch);
    };
    let exempt = policy == HttpPolicy::LoopbackExempt && page.is_loopback();
    if page.is_http() && !exempt && !saved.allow_http {
        return Err(Refusal::Insecure);
    }
    Ok(kind)
}

fn same_registrable_domain(a: &PageOrigin, b: &PageOrigin) -> bool {
    if a.scheme != b.scheme {
        return false;
    }
    match (
        registrable_domain(&a.host_str()),
        registrable_domain(&b.host_str()),
    ) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// The Unicode form of `host` if any label is punycode (`xn--`), so the
/// user sees what the browser's address bar may show. `None` for hosts
/// without punycode labels.
#[must_use]
pub fn idn_unicode(host: &str) -> Option<String> {
    let punycode = host.split('.').any(|label| {
        label
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("xn--"))
    });
    if !punycode {
        return None;
    }
    // Best effort: an invalid label still warns, with whatever converted.
    let (unicode, _) = idna::domain_to_unicode(host);
    Some(unicode)
}

/// A saved host that `page_host` imitates, if any: the first in sorted
/// order whose registrable domain has the same name under another public
/// suffix (`github.co` for `github.com`), or whose name differs by one
/// insertion, deletion, substitution or adjacent swap (Damerau, optimal
/// string alignment) when the shorter name has at least 4 characters
/// (`githuh.com`). A page on a saved registrable domain is not a lookalike
/// of it. Hosts without a registrable domain never match.
pub fn lookalike_of<'a>(
    page_host: &str,
    saved_hosts: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let page = registrable_domain(page_host)?;
    let (page_name, page_suffix) = split_domain(&page)?;
    let mut best: Option<&'a str> = None;
    for saved in saved_hosts {
        if best.is_some_and(|b| b <= saved) {
            continue;
        }
        let Some(domain) = registrable_domain(saved) else {
            continue;
        };
        if domain == page {
            continue;
        }
        let Some((name, suffix)) = split_domain(&domain) else {
            continue;
        };
        let same_name = name == page_name && suffix != page_suffix;
        let close = name != page_name
            && name.chars().count().min(page_name.chars().count()) >= 4
            && osa_distance_is_one(name, page_name);
        if same_name || close {
            best = Some(saved);
        }
    }
    best
}

/// `example.co.uk` into (`example`, `co.uk`).
fn split_domain(domain: &str) -> Option<(&str, &str)> {
    let suffix = psl::suffix_str(domain)?;
    let name = domain.strip_suffix(suffix)?.strip_suffix('.')?;
    (!name.is_empty()).then_some((name, suffix))
}

/// Whether the optimal-string-alignment (restricted Damerau-Levenshtein)
/// distance of `a` and `b` is exactly 1: one substitution, one adjacent
/// swap, or one insertion or deletion.
fn osa_distance_is_one(a: &str, b: &str) -> bool {
    use core::cmp::Ordering;

    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    match a.len().cmp(&b.len()) {
        Ordering::Equal => {
            let differing: Vec<usize> = a
                .iter()
                .zip(&b)
                .enumerate()
                .filter(|(_, (x, y))| x != y)
                .map(|(i, _)| i)
                .collect();
            match differing.as_slice() {
                [_] => true,
                [i, j] => *j == i + 1 && a.get(*i) == b.get(*j) && a.get(*j) == b.get(*i),
                _ => false,
            }
        }
        Ordering::Greater => is_one_deletion(&a, &b),
        Ordering::Less => is_one_deletion(&b, &a),
    }
}

/// Whether `short` is `long` with exactly one character removed.
fn is_one_deletion(long: &[char], short: &[char]) -> bool {
    if long.len() != short.len() + 1 {
        return false;
    }
    let common = long.iter().zip(short).take_while(|(x, y)| x == y).count();
    long.get(common + 1..) == short.get(common..)
}

#[cfg(test)]
mod mobile_tests;
#[cfg(test)]
mod redteam_g3_tests;
#[cfg(test)]
mod tests;
