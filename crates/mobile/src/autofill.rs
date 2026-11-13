//! Autofill of passwords and one-time codes (ADR-0092, brief §4): which
//! fields of a view tree to fill, what a request is for, which logins may
//! fill it, and what a tap hands to the platform.
//!
//! The rules live in `mosaic-origin`; this module maps the platform records to
//! them and reads the vault. Candidates carry no secret value. `fill_values`
//! and `fill_otp` check the match **again** for the chosen login, so a request
//! the platform fabricates cannot fill another site.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mosaic_core::model::{Item, ItemType, LoginUrl};
use mosaic_core::{ItemId, VaultId};
use mosaic_origin::app::{AndroidAppUrl, is_valid_package_name};
use mosaic_origin::assetlinks::{Relation, app_is_delegated, parse_asset_links};
use mosaic_origin::fields::{self, FieldRole, InputKind};
use mosaic_origin::target::{self, IosIdentifier, TargetError};
use mosaic_origin::{
    HttpPolicy, MatchKind, PageOrigin, Refusal, app, fill_allowed_with, idn_unicode, lookalike_of,
    saved_origin,
};
use mosaic_sync::transport::TransportError;
use mosaic_vault::session::{ItemFilter as SessionFilter, UnlockedSession, VaultProtection};

use crate::Core;
use crate::config::Platform;
use crate::error::{MobileError, Result};
use crate::items::{ItemKey, parse_item_key};
use crate::otp::OtpCode;
use crate::secret::SecretValue;
use crate::state::Phase;

/// How long an asset-links answer is kept (in memory, per process).
const ASSET_LINKS_TTL_MS: u64 = 24 * 60 * 60 * 1000;
/// How long a fill session waits for the user's tap (like a passkey session).
const FILL_SESSION_TTL_MS: u64 = 300_000;
/// Most fill sessions waiting at once; the oldest is dropped.
const MAX_FILL_SESSIONS: usize = 8;

/// An Android app that asks (the activity of the client, with its signers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidCaller {
    /// The package name.
    pub package: String,
    /// SHA-256 of the **current** signing certificate (32 bytes): the platform's
    /// `apkContentsSigners`, or the **last** entry of `signingCertificateHistory`
    /// (the newest certificate of a rotated key), chosen in Kotlin (ADR-0092,
    /// amendment G1-9). Never the old certificates of the history. Exactly one
    /// signer binds an app or answers a passkey request; more than one is
    /// refused (`invalidInput`), and a browser with more than one is not trusted.
    pub current_signers: Vec<Vec<u8>>,
    /// The `webDomain` of the root view node, if any.
    pub web_domain: Option<String>,
    /// The `webScheme` of the root view node, if any.
    pub web_scheme: Option<String>,
}

/// What kind of identifier an iOS service identifier is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceIdentifierKind {
    /// `.domain`.
    Domain,
    /// `.URL`.
    Url,
}

/// An iOS `ASCredentialServiceIdentifier`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceIdentifier {
    /// Domain or URL.
    pub kind: ServiceIdentifierKind,
    /// The host name or the URL.
    pub value: String,
}

/// What a fill is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FillTargetKind {
    /// A web page.
    Web {
        /// The page origin (`scheme://host[:port]`).
        origin: String,
    },
    /// An Android app that is not a trusted browser.
    AndroidApp {
        /// The package name.
        package: String,
        /// SHA-256 of each current signing certificate.
        certs: Vec<Vec<u8>>,
    },
}

/// Why the user is warned about the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginWarning {
    /// The host has punycode labels; `unicode` is how it may display.
    Punycode {
        /// The Unicode form.
        unicode: String,
    },
    /// The host imitates a host of a saved login.
    Lookalike {
        /// The saved host it imitates.
        saved_host: String,
    },
}

/// A fill request's target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillTarget {
    /// What it is.
    pub kind: FillTargetKind,
    /// What the UI shows (the host, or the package).
    pub display: String,
    /// A warning known without the vault.
    pub warning: Option<OriginWarning>,
}

/// How a saved URL matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMatch {
    /// The origin (or the app and its certificate) is equal.
    Exact,
    /// The registrable domain is equal (opt-in per URL).
    Etld1,
}

/// A login that may fill a request. No secret value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillCandidate {
    /// The login.
    pub key: ItemKey,
    /// Its title.
    pub title: String,
    /// Its user name.
    pub username: Option<String>,
    /// How it matched.
    pub match_kind: FillMatch,
    /// It has a one-time-password generator.
    pub has_otp: bool,
}

/// The logins that may fill a request, and the session the values are asked
/// with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillCandidates {
    /// The opaque ID of the fill session: Rust holds the resolved target
    /// (5 minutes, dropped on lock). `fill_values` and `fill_otp` take it, not
    /// a target the platform could change after the user chose (G1-12).
    pub session_id: String,
    /// The logins, best match first. No secret value.
    pub candidates: Vec<FillCandidate>,
}

/// What a tap hands to the platform.
pub struct FillValues {
    /// The user name.
    pub username: Option<String>,
    /// The password.
    pub password: Option<Arc<SecretValue>>,
}

/// An HTML attribute of a view node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attr {
    /// The name.
    pub name: String,
    /// The value.
    pub value: String,
}

/// One node of the platform's view tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldNode {
    /// The platform's ID of the node (returned in the selection).
    pub id: u32,
    /// The node has the input focus.
    pub focused: bool,
    /// The node is visible.
    pub visible: bool,
    /// The `webDomain` of the node.
    pub web_domain: Option<String>,
    /// The Android `inputType`.
    pub input_type: u32,
    /// The platform autofill hints.
    pub autofill_hints: Vec<String>,
    /// The HTML tag.
    pub html_tag: Option<String>,
    /// The HTML attributes.
    pub html_attrs: Vec<Attr>,
    /// The `idEntry`.
    pub id_entry: Option<String>,
    /// The hint text.
    pub hint: Option<String>,
}

/// Why nothing is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRefusal {
    /// The focused node is in a frame of another web domain.
    ForeignFrame,
    /// No field of the tree is fillable.
    NoFillableField,
}

/// The fields to fill, by node ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldSelection {
    /// The user name field.
    pub username: Option<u32>,
    /// The password field.
    pub password: Option<u32>,
    /// The new-password field.
    pub new_password: Option<u32>,
    /// The one-time-code field.
    pub otp: Option<u32>,
    /// Set when the request is refused (and no field is set).
    pub refused: Option<FieldRefusal>,
}

/// What the site's asset-links file says about an app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetLinksStatus {
    /// A web domain of the login lists the app and its certificate. `host` is
    /// the domain whose file did, which need not be the login's primary host
    /// (M10-G3-3).
    Verified {
        /// The host that lists the app.
        host: String,
    },
    /// The files were read and none lists the app.
    NotListed,
    /// No file could be read.
    Unavailable,
}

/// The fetch of an asset-links file (the real one is `mosaic_sync`'s; the test
/// hooks replace it).
pub(crate) type AssetLinksFetch =
    Arc<dyn Fn(&str) -> std::result::Result<Vec<u8>, TransportError> + Send + Sync>;

/// The asset-links fetch and its answers of the last 24 hours, in memory.
pub(crate) struct AssetLinks {
    pub(crate) fetch: Mutex<AssetLinksFetch>,
    cache: Mutex<HashMap<AssetLinksKey, (u64, bool)>>,
}

/// The most asset-links answers kept in memory (a hostile app can ask about
/// any relying party it likes).
const MAX_ASSET_LINKS_CACHE: usize = 256;

/// The relying party, the package and the signer an answer is for.
type AssetLinksKey = (String, String, [u8; 32]);

impl AssetLinks {
    pub(crate) fn new() -> Self {
        Self {
            fetch: Mutex::new(Arc::new(mosaic_sync::http::assetlinks::fetch)),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Caches an answer until `until` (ms). The map holds at most
    /// [`MAX_ASSET_LINKS_CACHE`] answers; a new one beyond that evicts the
    /// oldest (the one that expires first).
    fn remember(&self, key: AssetLinksKey, until: u64, delegated: bool) {
        let Ok(mut cache) = self.cache.lock() else {
            return;
        };
        if cache.len() >= MAX_ASSET_LINKS_CACHE && !cache.contains_key(&key) {
            let oldest = cache
                .iter()
                .min_by_key(|(_, (u, _))| *u)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (until, delegated));
    }

    /// Replaces the fetch and forgets what was cached (test hooks).
    #[cfg(feature = "test-hooks")]
    pub(crate) fn replace_fetch(&self, fetch: AssetLinksFetch) {
        if let Ok(mut current) = self.fetch.lock() {
            *current = fetch;
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    fn fetch(&self, host: &str) -> std::result::Result<Vec<u8>, TransportError> {
        let fetch = self
            .fetch
            .lock()
            .map(|f| Arc::clone(&f))
            .map_err(|_| TransportError::Io)?;
        fetch(host)
    }
}

/// The resolved targets of the fill requests waiting for a tap.
#[derive(Default)]
pub(crate) struct FillSessions {
    waiting: Vec<FillWaiting>,
}

struct FillWaiting {
    id: String,
    expires_at_ms: u64,
    target: Resolved,
}

impl FillSessions {
    fn insert(&mut self, target: Resolved, now_ms: u64) -> Result<String> {
        let id = crate::storage::hex(&mosaic_core::random_public_bytes::<16>()?);
        self.waiting.retain(|w| now_ms < w.expires_at_ms);
        while self.waiting.len() >= MAX_FILL_SESSIONS {
            self.waiting.remove(0);
        }
        self.waiting.push(FillWaiting {
            id: id.clone(),
            expires_at_ms: now_ms.saturating_add(FILL_SESSION_TTL_MS),
            target,
        });
        Ok(id)
    }

    /// The target of a session that has not run out.
    fn target(&mut self, id: &str, now_ms: u64) -> Option<Resolved> {
        self.waiting.retain(|w| now_ms < w.expires_at_ms);
        self.waiting
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.target.clone())
    }

    /// Forgets one session; an unknown one is ignored.
    fn release(&mut self, id: &str) {
        self.waiting.retain(|w| w.id != id);
    }

    /// Forgets every session (lock).
    pub(crate) fn clear(&mut self) {
        self.waiting.clear();
    }
}

/// What a fill request resolved to.
#[derive(Clone)]
pub(crate) enum Resolved {
    /// A page.
    Web(PageOrigin),
    /// An app.
    App {
        /// The package.
        package: String,
        /// Its current signers.
        certs: Vec<[u8; 32]>,
    },
}

fn certs_of(certs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>> {
    certs
        .iter()
        .map(|c| <[u8; 32]>::try_from(c.as_slice()).map_err(|_| MobileError::InvalidInput))
        .collect()
}

/// The one current signer of an app (G1-9): a caller with none or several is
/// refused, because the app's identity must be a single certificate.
pub(crate) fn single_signer(certs: &[Vec<u8>]) -> Result<[u8; 32]> {
    match certs {
        [only] => <[u8; 32]>::try_from(only.as_slice()).map_err(|_| MobileError::InvalidInput),
        _ => Err(MobileError::InvalidInput),
    }
}

fn target_error(e: TargetError) -> MobileError {
    match e {
        TargetError::Scheme => MobileError::InsecureOrigin,
        // A browser screen without a page (the address bar): nothing to fill.
        TargetError::BrowserWithoutDomain | TargetError::UntrustedBrowser => MobileError::NoMatch,
        _ => MobileError::InvalidInput,
    }
}

fn warning_of(idn: Option<String>) -> Option<OriginWarning> {
    idn.map(|unicode| OriginWarning::Punycode { unicode })
}

fn fill_target_of(t: target::FillTarget) -> FillTarget {
    match t {
        target::FillTarget::Web {
            origin,
            idn_unicode,
        } => FillTarget {
            display: origin.host_str(),
            kind: FillTargetKind::Web {
                origin: origin.serialize(),
            },
            warning: warning_of(idn_unicode),
        },
        target::FillTarget::AndroidApp { package, certs } => FillTarget {
            display: package.clone(),
            kind: FillTargetKind::AndroidApp {
                package,
                certs: certs.iter().map(|c| c.to_vec()).collect(),
            },
            warning: None,
        },
    }
}

/// The input kind of an Android `inputType`.
fn input_kind(input_type: u32) -> InputKind {
    let class = input_type & 0x0000_000f;
    let variation = input_type & 0x0000_0ff0;
    match (class, variation) {
        // TYPE_CLASS_TEXT with a password variation (plain, visible, web), or
        // TYPE_CLASS_NUMBER with the password variation.
        (1, 0x80 | 0x90 | 0xe0) | (2, 0x10) => InputKind::Password,
        (1, 0x20 | 0xd0) | (2 | 3, _) => InputKind::Other,
        _ => InputKind::Text,
    }
}

fn node_of(n: &FieldNode) -> fields::FieldNode {
    let autocomplete = n
        .html_attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("autocomplete"))
        .map(|a| a.value.clone());
    let id_entry = match (&n.id_entry, &n.hint) {
        (Some(id), Some(hint)) => Some(format!("{id} {hint}")),
        (Some(text), None) | (None, Some(text)) => Some(text.clone()),
        (None, None) => None,
    };
    fields::FieldNode {
        web_domain: n.web_domain.clone(),
        is_web: n.html_tag.is_some(),
        hints: n.autofill_hints.clone(),
        autocomplete,
        input_kind: input_kind(n.input_type),
        id_entry,
        visible: n.visible,
        focused: n.focused,
    }
}

/// The nodes a selection can point at; the same cap as the origin crate's.
fn select_fields(nodes: &[FieldNode], target: &FillTarget) -> FieldSelection {
    // A web page is rooted at the target's own host: nodes of any other domain,
    // or of none, are foreign (M10-G3-1). A target that is not an origin has no
    // host, so every node is foreign.
    let host = match &target.kind {
        FillTargetKind::Web { origin } => Some(
            PageOrigin::parse(origin)
                .map(|o| o.host_str())
                .unwrap_or_default(),
        ),
        FillTargetKind::AndroidApp { .. } => None,
    };
    let scope = host
        .as_deref()
        .map_or(fields::FieldScope::App, |host| fields::FieldScope::Web {
            host,
        });
    let limited = nodes.get(..fields::MAX_NODES).unwrap_or(nodes);
    let mapped: Vec<fields::FieldNode> = limited.iter().map(node_of).collect();
    let chosen = fields::select_scoped_fill_fields(&mapped, scope);
    let mut out = FieldSelection::default();
    if chosen.refused == Some(fields::FieldRefusal::ForeignFrame) {
        out.refused = Some(FieldRefusal::ForeignFrame);
        return out;
    }
    for field in &chosen.fields {
        let Some(id) = limited.get(field.index).map(|n| n.id) else {
            continue;
        };
        let slot = match field.role {
            FieldRole::Username => &mut out.username,
            FieldRole::Password => &mut out.password,
            FieldRole::NewPassword => &mut out.new_password,
            FieldRole::Otp => &mut out.otp,
        };
        slot.get_or_insert(id);
    }
    if out.username.is_none()
        && out.password.is_none()
        && out.new_password.is_none()
        && out.otp.is_none()
    {
        out.refused = Some(FieldRefusal::NoFillableField);
    }
    out
}

/// The vaults that hold logins this session can read and fill from.
pub(crate) fn usable_vaults(session: &UnlockedSession) -> HashSet<VaultId> {
    session
        .vaults()
        .into_iter()
        .filter(|v| {
            v.available
                && !matches!(
                    session.vault_protection(v.vault_id),
                    Ok(VaultProtection::HardwareKey { .. })
                )
        })
        .map(|v| v.vault_id)
        .collect()
}

/// How the saved URLs of a login match a target: the best match, else why not.
fn match_item(item: &Item, target: &Resolved) -> std::result::Result<MatchKind, Refusal> {
    let mut insecure = false;
    let mut best: Option<MatchKind> = None;
    for (_, url) in item.urls() {
        let verdict = match target {
            Resolved::Web(page) => {
                fill_allowed_with(&url, page, HttpPolicy::LoopbackNeedsAllowHttp)
            }
            Resolved::App { package, certs } => app::app_fill_allowed(&url, package, certs),
        };
        match verdict {
            Ok(MatchKind::ExactOrigin) => best = Some(MatchKind::ExactOrigin),
            Ok(kind) => best = best.or(Some(kind)),
            Err(Refusal::Insecure) => insecure = true,
            Err(_) => {}
        }
    }
    best.ok_or(if insecure {
        Refusal::Insecure
    } else {
        Refusal::NoMatch
    })
}

fn candidate_of(key: ItemKey, item: &Item, kind: MatchKind) -> FillCandidate {
    FillCandidate {
        key,
        title: item.title().unwrap_or_default().to_owned(),
        username: item
            .text(mosaic_core::model::TextField::Username)
            .filter(|u| !u.is_empty())
            .map(str::to_owned),
        match_kind: match kind {
            MatchKind::ExactOrigin => FillMatch::Exact,
            MatchKind::Etld1 => FillMatch::Etld1,
        },
        has_otp: item.otp().is_some(),
    }
}

impl Core {
    /// An Android-only entry point on an iOS core (or the other way round) is
    /// refused: the platform of the core decides which calls exist (G1-5).
    pub(crate) fn require_platform(&self, platform: Platform) -> Result<()> {
        if self.config.platform == platform {
            Ok(())
        } else {
            Err(MobileError::InvalidInput)
        }
    }

    fn resolve_target(target: &FillTarget) -> Result<Resolved> {
        match &target.kind {
            FillTargetKind::Web { origin } => PageOrigin::parse(origin)
                .map(Resolved::Web)
                .map_err(|_| MobileError::InvalidInput),
            FillTargetKind::AndroidApp { package, certs } => {
                if !is_valid_package_name(package) {
                    return Err(MobileError::InvalidInput);
                }
                Ok(Resolved::App {
                    package: package.clone(),
                    certs: certs_of(certs)?,
                })
            }
        }
    }

    /// Which fields of a view tree to fill for `target` (from
    /// `fill_target_android`). An app target gets native fields only, never web
    /// content in a `WebView`; a web target starts at the page root. Pure;
    /// works while locked.
    #[must_use]
    pub fn select_fill_fields(&self, nodes: Vec<FieldNode>, target: &FillTarget) -> FieldSelection {
        select_fields(&nodes, target)
    }

    /// What an Android request is for. Pure; works while locked. A
    /// `web_domain` counts only for a trusted browser.
    ///
    /// # Errors
    /// `invalidInput` (a bad package, certificate or domain), `insecureOrigin`
    /// (a scheme that is neither `http` nor `https`), `noMatch` (an allowlisted
    /// browser that sent no `webDomain`: the browser app is never a target).
    pub fn fill_target_android(&self, caller: AndroidCaller) -> Result<FillTarget> {
        self.require_platform(Platform::Android)?;
        let certs = certs_of(&caller.current_signers)?;
        let target = target::android_fill_target(&target::AndroidCaller {
            package: &caller.package,
            certs: &certs,
            web_domain: caller.web_domain.as_deref(),
            web_scheme: caller.web_scheme.as_deref(),
        })
        .map_err(target_error)?;
        Ok(fill_target_of(target))
    }

    /// What an iOS request is for. Pure.
    ///
    /// # Errors
    /// `invalidInput`, `insecureOrigin`.
    pub fn fill_target_ios(&self, identifier: ServiceIdentifier) -> Result<FillTarget> {
        self.require_platform(Platform::Ios)?;
        let id = match identifier.kind {
            ServiceIdentifierKind::Domain => IosIdentifier::Domain(&identifier.value),
            ServiceIdentifierKind::Url => IosIdentifier::Url(&identifier.value),
        };
        target::ios_fill_target(id)
            .map(fill_target_of)
            .map_err(target_error)
    }

    /// The warning a request deserves given the saved logins: the punycode
    /// warning, else the saved host the page imitates.
    ///
    /// # Errors
    /// `locked`, `invalidInput`.
    pub fn fill_warning(&self, target: FillTarget) -> Result<Option<OriginWarning>> {
        let Resolved::Web(origin) = Self::resolve_target(&target)? else {
            return Ok(None);
        };
        if let Some(unicode) = idn_unicode(&origin.host_str()) {
            return Ok(Some(OriginWarning::Punycode { unicode }));
        }
        let hosts = self.with_session(|s| {
            let usable = usable_vaults(s);
            let mut hosts: Vec<String> = s
                .items(&SessionFilter::default())
                .into_iter()
                .filter(|r| usable.contains(&r.vault_id))
                .flat_map(|r| r.item.urls())
                .filter_map(|(_, u)| saved_origin(u.url.as_str()).map(|o| o.host_str()))
                .collect();
            hosts.sort();
            hosts.dedup();
            Ok(hosts)
        })?;
        Ok(
            lookalike_of(&origin.host_str(), hosts.iter().map(String::as_str)).map(|saved| {
                OriginWarning::Lookalike {
                    saved_host: saved.to_owned(),
                }
            }),
        )
    }

    /// The logins that may fill the target, and the fill session the values
    /// are asked with. No secret value.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `insecureOrigin` (matching logins exist, but
    /// the page is plain `http` and they do not allow it).
    pub fn fill_candidates(&self, target: FillTarget) -> Result<FillCandidates> {
        let resolved = Self::resolve_target(&target)?;
        let held = resolved.clone();
        let (mut list, insecure) = self.with_session(|s| {
            let usable = usable_vaults(s);
            let mut list = Vec::new();
            let mut insecure = false;
            for entry in s.items(&SessionFilter::default()) {
                if !usable.contains(&entry.vault_id)
                    || entry.item.item_type() != Some(ItemType::Login)
                {
                    continue;
                }
                match match_item(entry.item, &resolved) {
                    Ok(kind) => {
                        let key = ItemKey {
                            vault_id: entry.vault_id.to_string(),
                            item_id: entry.item_id.to_string(),
                        };
                        list.push(candidate_of(key, entry.item, kind));
                    }
                    Err(Refusal::Insecure) => insecure = true,
                    Err(_) => {}
                }
            }
            Ok((list, insecure))
        })?;
        if list.is_empty() && insecure {
            return Err(MobileError::InsecureOrigin);
        }
        list.sort_by(|a, b| {
            let rank = |m: FillMatch| u8::from(m == FillMatch::Etld1);
            rank(a.match_kind)
                .cmp(&rank(b.match_kind))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                .then_with(|| a.key.item_id.cmp(&b.key.item_id))
        });
        let now = self.now_ms();
        let mut guard = self.guard()?;
        if !matches!(guard.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        let session_id = guard.slots.fill.insert(held, now)?;
        Ok(FillCandidates {
            session_id,
            candidates: list,
        })
    }

    /// Forgets a fill session the caller will not use (a list built for the
    /// system picker opens one per request and never fills from it). An
    /// unknown or expired session, and a locked core, are ignored: there is
    /// nothing to release (M10-G3).
    pub fn fill_release(&self, session_id: String) {
        if let Ok(mut guard) = self.guard() {
            guard.slots.fill.release(&session_id);
        }
    }

    /// The target a fill session holds; an unknown, expired or dropped one is
    /// refused.
    fn fill_session_target(&self, session_id: &str) -> Result<Resolved> {
        let now = self.now_ms();
        let mut guard = self.guard()?;
        if !matches!(guard.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        guard
            .slots
            .fill
            .target(session_id, now)
            .ok_or(MobileError::NotFound)
    }

    /// Checks again that `candidate` may fill the target of the fill session
    /// and runs `f` on it.
    fn with_matching_login<R>(
        &self,
        session_id: &str,
        candidate: &ItemKey,
        f: impl FnOnce(&mut UnlockedSession, VaultId, ItemId) -> Result<R>,
    ) -> Result<R> {
        let resolved = self.fill_session_target(session_id)?;
        let (vault, id) = parse_item_key(&candidate.vault_id, &candidate.item_id)?;
        self.with_session(|s| {
            if !usable_vaults(s).contains(&vault) {
                return Err(MobileError::VaultUnavailable);
            }
            let item = s.item(vault, id)?;
            if item.is_deleted() || item.trashed_at().is_some() {
                return Err(MobileError::NotFound);
            }
            if item.item_type() != Some(ItemType::Login) {
                return Err(MobileError::NoMatch);
            }
            match match_item(item, &resolved) {
                Ok(_) => {}
                Err(Refusal::Insecure) => return Err(MobileError::InsecureOrigin),
                Err(_) => {
                    return Err(match resolved {
                        Resolved::Web(_) => MobileError::NoMatch,
                        Resolved::App { .. } => MobileError::AppNotLinked,
                    });
                }
            }
            f(s, vault, id)
        })
    }

    /// The user name and password of a candidate, after the match is checked
    /// again against the target the fill session holds.
    ///
    /// # Errors
    /// `locked`, `notFound` (also an unknown or expired fill session),
    /// `noMatch`, `appNotLinked`, `insecureOrigin`, `vaultUnavailable`,
    /// `invalidInput`.
    pub fn fill_values(&self, session_id: String, candidate: ItemKey) -> Result<FillValues> {
        let generation = self.generation();
        let (username, password) =
            self.with_matching_login(&session_id, &candidate, |s, v, i| {
                let item = s.item(v, i)?;
                let username = item
                    .text(mosaic_core::model::TextField::Username)
                    .filter(|u| !u.is_empty())
                    .map(str::to_owned);
                let password = item
                    .secret(mosaic_core::model::SecretField::Password)
                    .map_err(MobileError::from)?
                    .map(|p| SecretValue::from_text(p.expose_secret()));
                Ok((username, password))
            })?;
        self.work_pause();
        let password = password
            .map(|p| self.issue_gated(p, generation))
            .transpose()?;
        Ok(FillValues { username, password })
    }

    /// The one-time code of a candidate, after the match is checked again.
    ///
    /// # Errors
    /// As [`Core::fill_values`], and `notFound` for a login without a
    /// generator.
    pub fn fill_otp(&self, session_id: String, candidate: ItemKey) -> Result<Arc<SecretValue>> {
        self.with_matching_login(&session_id, &candidate, |_, _, _| Ok(()))?;
        let code: OtpCode = self.otp_code(&candidate.vault_id, &candidate.item_id)?;
        Ok(code.code)
    }

    /// The privileged browsers, in the format of `CallingAppInfo.getOrigin`.
    #[must_use]
    pub fn privileged_browsers_json(&self) -> String {
        mosaic_origin::android_browsers::privileged_browsers_json().to_owned()
    }

    /// Binds an Android app to a login: writes `androidapp://<package>?cert=…`
    /// into the login (the explicit choice on the login itself) and removes
    /// the login's other URLs of the same package.
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput` (a bad package or certificate, or
    /// not a login), `vaultUnavailable`.
    pub fn link_android_app(
        &self,
        vault_id: &str,
        item_id: &str,
        caller: AndroidCaller,
    ) -> Result<()> {
        self.require_platform(Platform::Android)?;
        // A browser is a web client: linking it would make the login fill on
        // every site (M10-G2 H1).
        if mosaic_origin::android_browsers::is_browser_package(&caller.package) {
            return Err(MobileError::InvalidInput);
        }
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        // The one current signer binds the app (the one `android_app_origin`
        // uses); several are refused (G1-9).
        let cert = single_signer(&caller.current_signers)?;
        let linked = AndroidAppUrl::new(&caller.package, Some(cert))
            .map_err(|_| MobileError::InvalidInput)?;
        let new_url = LoginUrl::new(&linked.canonical());
        self.with_session(|s| {
            if !usable_vaults(s).contains(&vault) {
                return Err(MobileError::VaultUnavailable);
            }
            let item = s.item(vault, id)?;
            if item.is_deleted() || item.item_type() != Some(ItemType::Login) {
                return Err(MobileError::InvalidInput);
            }
            // Keep one equal binding, drop every other URL of the package.
            let mut present = false;
            let mut remove = Vec::new();
            for (element, saved) in item.urls() {
                let Ok(app) = AndroidAppUrl::parse(saved.url.as_str()) else {
                    continue;
                };
                if app.package() != caller.package {
                    continue;
                }
                if saved.url.as_str() == new_url.url.as_str() && !present {
                    present = true;
                } else {
                    remove.push(element);
                }
            }
            s.update_item(vault, id, |e| {
                for element in &remove {
                    e.remove_url(element)?;
                }
                if !present {
                    e.add_url(&new_url)?;
                }
                Ok(())
            })?;
            Ok(())
        })
    }

    /// What the asset-links files of the login's web domains say about the
    /// app (network).
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput` (also in the extension context and
    /// on iOS: the network and the Android entry points are not there).
    pub fn asset_links_status(
        &self,
        vault_id: &str,
        item_id: &str,
        caller: AndroidCaller,
    ) -> Result<AssetLinksStatus> {
        self.require_app()?;
        self.require_platform(Platform::Android)?;
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        let cert = &single_signer(&caller.current_signers)?;
        if !is_valid_package_name(&caller.package) {
            return Err(MobileError::InvalidInput);
        }
        let hosts: Vec<String> = self.with_session(|s| {
            let item = s.item(vault, id)?;
            if item.is_deleted() {
                return Err(MobileError::NotFound);
            }
            let mut hosts: Vec<String> = item
                .urls()
                .iter()
                .filter_map(|(_, u)| saved_origin(u.url.as_str()))
                .filter(|o| !o.is_http())
                .map(|o| {
                    o.serialize()
                        .strip_prefix("https://")
                        .unwrap_or_default()
                        .to_owned()
                })
                .filter(|h| !h.is_empty())
                .collect();
            hosts.sort();
            hosts.dedup();
            Ok(hosts)
        })?;
        let mut read_any = false;
        for host in &hosts {
            let body = match self.assetlinks.fetch(host) {
                Ok(body) => body,
                Err(TransportError::NotFound) => {
                    read_any = true;
                    continue;
                }
                Err(_) => continue,
            };
            read_any = true;
            if parse_asset_links(&body).is_ok_and(|st| {
                app_is_delegated(&st, &caller.package, cert, Relation::GetLoginCreds)
            }) {
                return Ok(AssetLinksStatus::Verified { host: host.clone() });
            }
        }
        Ok(if read_any {
            AssetLinksStatus::NotListed
        } else {
            AssetLinksStatus::Unavailable
        })
    }

    /// Whether the site `rp_id` delegates passkeys to the app (cached for 24
    /// hours per relying party, package and certificate).
    ///
    /// # Errors
    /// `appNotVerified` (the site does not list the app), `assetLinksUnavailable`
    /// (the file could not be read).
    pub(crate) fn check_asset_links(
        &self,
        rp_id: &str,
        package: &str,
        cert: &[u8; 32],
    ) -> Result<()> {
        // The fetch is network: the extension context never opens one
        // (CRYPTO_SPEC §6.9.3, G1-8), and only Android runs this check.
        self.require_app()?;
        self.require_platform(Platform::Android)?;
        let key = (rp_id.to_owned(), package.to_owned(), *cert);
        let now = self.now_ms();
        let cached = self
            .assetlinks
            .cache
            .lock()
            .ok()
            .and_then(|c| c.get(&key).copied())
            .filter(|(until, _)| now < *until);
        let delegated = if let Some((_, delegated)) = cached {
            delegated
        } else {
            let delegated = match self.assetlinks.fetch(rp_id) {
                Ok(body) => parse_asset_links(&body).is_ok_and(|st| {
                    mosaic_origin::assetlinks::app_is_delegated_for_passkeys(&st, package, cert)
                }),
                Err(TransportError::NotFound) => false,
                Err(_) => return Err(MobileError::AssetLinksUnavailable),
            };
            self.assetlinks
                .remember(key, now.saturating_add(ASSET_LINKS_TTL_MS), delegated);
            delegated
        };
        if delegated {
            Ok(())
        } else {
            Err(MobileError::AppNotVerified)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_asset_links_cache_is_bounded_and_evicts_the_oldest() {
        let links = AssetLinks::new();
        let key = |n: usize| (format!("rp{n}.example"), "pkg".to_owned(), [0u8; 32]);
        for n in 0..MAX_ASSET_LINKS_CACHE + 10 {
            links.remember(key(n), 1000 + n as u64, true);
        }
        let cache = links.cache.lock().unwrap();
        assert_eq!(cache.len(), MAX_ASSET_LINKS_CACHE);
        assert!(!cache.contains_key(&key(0)), "the oldest answer is evicted");
        assert!(cache.contains_key(&key(MAX_ASSET_LINKS_CACHE + 9)));
    }
}
