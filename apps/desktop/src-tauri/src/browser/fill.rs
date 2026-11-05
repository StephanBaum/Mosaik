//! The vault side of the lookup, fill, copy and generate ops (ADR-0037).
//!
//! Every match decision goes through `mosaic-origin`. Nothing here lists the
//! vault for the extension: `listForOrigin` returns the logins that match
//! one page (at most 50), `search` at most 20 summaries without secrets.
//! Only `fill` and `copy` count as user activity.

use mosaic_core::model::{Item, ItemType, OtpKind, SecretField, TextField};
use mosaic_core::{ItemId, VaultId};
use mosaic_generator::{PasswordOptions, PasswordRules, generate_password, rules_for_domain};
use mosaic_native_messaging::bodies::{
    Blocked, CopyBody, CopyField, CopyResult, CopySource, FillBody, FillResult, FrameKind,
    GenerateResult, ListForOriginBody, ListForOriginResult, LoginMatch, MatchedBy, OriginWarning,
    RulesSource, SearchBody, SearchHit, SearchResult,
};
use mosaic_native_messaging::protocol::{ErrorCode as WireError, Secret};
use mosaic_origin::{
    MatchKind, PageOrigin, Refusal, fill_allowed, idn_unicode, lookalike_of, saved_origin,
};
use mosaic_vault::session::{ItemFilter, ItemRef, UnlockedSession, VaultProtection};
use serde::Deserialize;
use serde::de::value::{Error as ValueError, StrDeserializer};
use serde_json::value::RawValue;

use crate::commands::generator::{generate, generator_error, strength};
use crate::commands::totp::current_code;
use crate::contract::clipboard::{CopyField as DesktopField, CopyRequest, CopySource as Source};
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::generator::GenerateRequest;
use crate::contract::ids::{WireItemId, WireVaultId};
use crate::state::Backend;

/// The most matches `listForOrigin` returns.
pub const MAX_MATCHES: usize = 50;
/// The most hits `search` returns.
pub const MAX_HITS: usize = 20;
/// The longest search text, in characters.
pub const MAX_QUERY_CHARS: usize = 128;
/// The longest `rules` text of a `generate` request, in bytes.
pub const MAX_RULES_BYTES: usize = 4096;
/// The longest host of a `generate` request, in bytes.
const MAX_HOST_BYTES: usize = 253;

/// How an item matches a page.
pub(super) struct Matched {
    pub(super) kind: MatchKind,
    /// The URL matched, but the page is plain HTTP and the URL does not
    /// allow it.
    pub(super) blocked: bool,
    /// The host of the URL that matched.
    pub(super) host: Option<String>,
}

/// The IPC code as the wire reports it.
pub(super) const fn wire(error: IpcError) -> WireError {
    match error.code {
        ErrorCode::Locked => WireError::Locked,
        ErrorCode::ItemNotFound | ErrorCode::ItemDeleted => WireError::ItemNotFound,
        ErrorCode::VaultUnavailable | ErrorCode::VaultNotFound => WireError::VaultUnavailable,
        ErrorCode::ClipboardUnavailable => WireError::ClipboardUnavailable,
        ErrorCode::FieldEmpty => WireError::FieldEmpty,
        ErrorCode::NoOtp => WireError::NoOtp,
        ErrorCode::Busy => WireError::Busy,
        _ => WireError::Internal,
    }
}

/// Parses a page origin from the extension.
pub(super) fn parse_origin(text: &str) -> Result<PageOrigin, WireError> {
    PageOrigin::parse(text).map_err(|_| WireError::InvalidRequest)
}

/// Parses a vault and an item ID in the canonical form.
pub(super) fn parse_ids(vault: &str, item: &str) -> Result<(VaultId, ItemId), WireError> {
    let vault = WireVaultId::deserialize(StrDeserializer::<ValueError>::new(vault))
        .map_err(|_| WireError::InvalidRequest)?;
    let item = WireItemId::deserialize(StrDeserializer::<ValueError>::new(item))
        .map_err(|_| WireError::InvalidRequest)?;
    Ok((vault.get(), item.get()))
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty()).map(str::to_owned)
}

/// The best match of `item` for `page`: an allowed one before a blocked one,
/// an exact origin before an eTLD+1.
pub(super) fn best_match(item: &Item, page: &PageOrigin) -> Option<Matched> {
    let mut best: Option<Matched> = None;
    for (_, url) in item.urls() {
        let (kind, blocked) = match fill_allowed(&url, page) {
            Ok(kind) => (kind, false),
            Err(Refusal::Insecure) => {
                let exact = saved_origin(url.url.as_str()).as_ref() == Some(page);
                (
                    if exact {
                        MatchKind::ExactOrigin
                    } else {
                        MatchKind::Etld1
                    },
                    true,
                )
            }
            Err(Refusal::NoMatch) => continue,
        };
        let rank = |blocked: bool, kind: MatchKind| (blocked, kind == MatchKind::Etld1);
        if best
            .as_ref()
            .is_none_or(|b| rank(blocked, kind) < rank(b.blocked, b.kind))
        {
            best = Some(Matched {
                kind,
                blocked,
                host: saved_origin(url.url.as_str()).map(|o| o.host_str()),
            });
        }
    }
    best
}

/// The live logins outside the trash.
pub(super) fn logins(session: &UnlockedSession) -> Vec<ItemRef<'_>> {
    session
        .items(&ItemFilter::default())
        .into_iter()
        .filter(|entry| entry.item.item_type() == Some(ItemType::Login))
        .collect()
}

fn first_host(item: &Item) -> Option<String> {
    item.urls()
        .first()
        .and_then(|(_, url)| saved_origin(url.url.as_str()))
        .map(|origin| origin.host_str())
}

fn username_of(item: &Item) -> Option<String> {
    non_empty(item.text(TextField::Username))
}

fn login_match(entry: &ItemRef<'_>, matched: Matched) -> LoginMatch {
    LoginMatch {
        vault_id: entry.vault_id.to_string(),
        item_id: entry.item_id.to_string(),
        title: non_empty(entry.item.title()),
        username: username_of(entry.item),
        host: matched.host,
        has_otp: entry.item.otp().is_some(),
        favorite: entry.item.favorite(),
        matched_by: match matched.kind {
            MatchKind::ExactOrigin => MatchedBy::ExactOrigin,
            MatchKind::Etld1 => MatchedBy::Etld1,
        },
        blocked: matched.blocked.then_some(Blocked::Insecure),
    }
}

fn sort_key(entry: &LoginMatch) -> (bool, String, String) {
    (
        !entry.favorite,
        entry.title.as_deref().unwrap_or_default().to_lowercase(),
        entry.item_id.clone(),
    )
}

fn warnings(page: &PageOrigin, no_matches: bool, session: &UnlockedSession) -> Vec<OriginWarning> {
    let mut out = Vec::new();
    let host = page.host_str();
    if let Some(unicode_host) = idn_unicode(&host) {
        out.push(OriginWarning::Idn { unicode_host });
    }
    if page.is_http() && !page.is_loopback() {
        out.push(OriginWarning::Insecure {});
    }
    if no_matches {
        let saved: Vec<String> = logins(session)
            .iter()
            .flat_map(|entry| {
                entry
                    .item
                    .urls()
                    .into_iter()
                    .filter_map(|(_, url)| saved_origin(url.url.as_str()).map(|o| o.host_str()))
            })
            .collect();
        if let Some(saved_host) = lookalike_of(&host, saved.iter().map(String::as_str)) {
            out.push(OriginWarning::Lookalike {
                saved_host: saved_host.to_owned(),
            });
        }
    }
    out
}

impl Backend {
    /// `listForOrigin`: the logins that match one page.
    pub(crate) fn browser_list_for_origin(
        &self,
        body: &ListForOriginBody,
    ) -> Result<ListForOriginResult, WireError> {
        let page = parse_origin(&body.origin)?;
        parse_origin(&body.top_origin)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        let mut items: Vec<LoginMatch> = logins(session)
            .iter()
            .filter_map(|entry| best_match(entry.item, &page).map(|m| login_match(entry, m)))
            .collect();
        items.sort_by_cached_key(sort_key);
        items.truncate(MAX_MATCHES);
        let warnings = warnings(&page, items.is_empty(), session);
        Ok(ListForOriginResult { items, warnings })
    }

    /// `search`: at most 20 login summaries, no secrets.
    pub(crate) fn browser_search(&self, body: &SearchBody) -> Result<SearchResult, WireError> {
        let query = body.query.trim();
        if query.is_empty() || query.chars().count() > MAX_QUERY_CHARS {
            return Err(WireError::InvalidRequest);
        }
        let page = body.origin.as_deref().map(parse_origin).transpose()?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        let items = session
            .search(query)
            .into_iter()
            .filter(|entry| entry.item.item_type() == Some(ItemType::Login))
            .take(MAX_HITS)
            .map(|entry| SearchHit {
                vault_id: entry.vault_id.to_string(),
                item_id: entry.item_id.to_string(),
                title: non_empty(entry.item.title()).unwrap_or_default(),
                username: username_of(entry.item),
                host: first_host(entry.item),
                has_otp: entry.item.otp().is_some(),
                favorite: entry.item.favorite(),
                fillable: page
                    .as_ref()
                    .and_then(|page| best_match(entry.item, page))
                    .is_some_and(|m| !m.blocked),
            })
            .collect();
        Ok(SearchResult { items })
    }

    /// `fill`: one item's credentials for one frame.
    pub(crate) fn browser_fill(&self, body: &FillBody) -> Result<FillResult, WireError> {
        let page = parse_origin(&body.origin)?;
        let top = parse_origin(&body.top_origin)?;
        match body.frame {
            FrameKind::Top if page != top => return Err(WireError::InvalidRequest),
            FrameKind::SameOriginFrame if page != top => return Err(WireError::CrossOriginFrame),
            _ => {}
        }
        if !body.want.login && !body.want.otp {
            return Err(WireError::InvalidRequest);
        }
        let (vault_id, item_id) = parse_ids(&body.vault_id, &body.item_id)?;
        let mut slot = self.slot().map_err(wire)?;
        let session = slot.unlocked_mut().map_err(wire)?;
        let found = fillable_login(session, vault_id, item_id, &page)?;
        let otp = if body.want.otp {
            match found.otp_kind {
                None | Some(OtpKind::Hotp) if body.want.login => None,
                None => return Err(WireError::NoOtp),
                Some(OtpKind::Hotp) => return Err(WireError::OtpNotFillable),
                Some(_) => {
                    let (code, ..) = current_code(session, vault_id, item_id).map_err(wire)?;
                    Some(Secret::new(code.expose_secret().to_owned()))
                }
            }
        } else {
            None
        };
        let (username, password) = if body.want.login {
            (
                found.username,
                found
                    .password
                    .map(|p| Secret::new(p.expose_secret().to_owned())),
            )
        } else {
            (None, None)
        };
        if body.want.login && username.is_none() && password.is_none() && otp.is_none() {
            return Err(WireError::FieldEmpty);
        }
        self.touch();
        Ok(FillResult {
            username,
            password,
            otp,
        })
    }

    /// `copy`: onto the clipboard, in Rust.
    pub(crate) fn browser_copy(&self, body: &CopyBody) -> Result<CopyResult, WireError> {
        let source = match &body.source {
            CopySource::Generated {} => Source::Generated,
            CopySource::Item {
                vault_id,
                item_id,
                field,
            } => {
                let (vault, item) = parse_ids(vault_id, item_id)?;
                {
                    let slot = self.slot().map_err(wire)?;
                    login_exists(slot.unlocked().map_err(wire)?, vault, item)?;
                }
                Source::Item {
                    vault_id: vault.into(),
                    item_id: item.into(),
                    field: match field {
                        CopyField::Username => DesktopField::Username,
                        CopyField::Password => DesktopField::Password,
                        CopyField::Totp => DesktopField::Totp,
                    },
                }
            }
        };
        let receipt = self
            .copy_to_clipboard(CopyRequest { source })
            .map_err(wire)?;
        Ok(CopyResult {
            clear_after_seconds: receipt.clear_after_seconds,
        })
    }
}

/// What a fill reads from an item before the borrow of the session ends.
struct Found {
    username: Option<String>,
    password: Option<mosaic_core::SecretString>,
    otp_kind: Option<OtpKind>,
}

/// The vault must be open, the item a live login outside the trash, and one
/// of its URLs must allow `page`.
fn fillable_login(
    session: &UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
    page: &PageOrigin,
) -> Result<Found, WireError> {
    login_exists(session, vault_id, item_id)?;
    let item = session
        .item(vault_id, item_id)
        .map_err(|_| WireError::ItemNotFound)?;
    match best_match(item, page) {
        None => Err(WireError::OriginMismatch),
        Some(matched) if matched.blocked => Err(WireError::InsecureOrigin),
        Some(_) => Ok(Found {
            username: username_of(item),
            password: item
                .secret(SecretField::Password)
                .map_err(|_| WireError::Internal)?
                .filter(|p| !p.expose_secret().is_empty()),
            otp_kind: item.otp().map(|config| config.kind),
        }),
    }
}

/// A closed hardware-protected vault is `vaultUnavailable`; anything but a
/// live login outside the trash is `itemNotFound` (the type is not
/// revealed).
fn login_exists(
    session: &UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
) -> Result<(), WireError> {
    if session
        .vault_protection(vault_id)
        .map_err(|_| WireError::ItemNotFound)?
        == (VaultProtection::HardwareKey { open: false })
    {
        return Err(WireError::VaultUnavailable);
    }
    let item = session
        .item(vault_id, item_id)
        .map_err(|_| WireError::ItemNotFound)?;
    if item.is_deleted() || item.trashed_at().is_some() || item.item_type() != Some(ItemType::Login)
    {
        return Err(WireError::ItemNotFound);
    }
    Ok(())
}

/// The `request` of a `generate` op: the desktop's `GenerateRequest`, where
/// `forSite` also carries the password rules the page announced.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum GenerateKind {
    Password {
        length: u32,
        lowercase: bool,
        uppercase: bool,
        digits: bool,
        symbols: bool,
        exclude_ambiguous: bool,
    },
    Passphrase {
        words: u32,
        separator: String,
        capitalize: bool,
        append_digit: bool,
    },
    ForSite {
        host: String,
        #[serde(deserialize_with = "required")]
        rules: Option<String>,
    },
}

fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl Backend {
    /// `generate`: the desktop's generator, keeping the value for a `copy`
    /// with the source `generated`. Not user activity.
    pub(crate) fn browser_generate(&self, request: &RawValue) -> Result<GenerateResult, WireError> {
        let kind: GenerateKind =
            serde_json::from_str(request.get()).map_err(|_| WireError::InvalidRequest)?;
        let (generated, rules_source) = match kind {
            GenerateKind::Password {
                length,
                lowercase,
                uppercase,
                digits,
                symbols,
                exclude_ambiguous,
            } => (
                generate(GenerateRequest::Password {
                    length,
                    lowercase,
                    uppercase,
                    digits,
                    symbols,
                    exclude_ambiguous,
                }),
                None,
            ),
            GenerateKind::Passphrase {
                words,
                separator,
                capitalize,
                append_digit,
            } => (
                generate(GenerateRequest::Passphrase {
                    words,
                    separator,
                    capitalize,
                    append_digit,
                }),
                None,
            ),
            GenerateKind::ForSite { host, rules } => site_password(&host, rules.as_deref())?,
        };
        let generated = generated.map_err(|e| wire(generator_error(e)))?;
        self.keep_generated(&generated).map_err(wire)?;
        let entropy = generated.entropy();
        Ok(GenerateResult {
            value: Secret::new(generated.expose_secret().to_owned()),
            entropy_bits: entropy.bits(),
            strength: serde_json::to_value(strength(entropy.strength()))
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or(WireError::Internal)?,
            rules_source,
        })
    }
}

type SitePassword = (
    Result<mosaic_generator::GeneratedPassword, mosaic_generator::Error>,
    Option<RulesSource>,
);

/// A password for a site: the rules the page announced if they work, else
/// the vendored quirks for the host, else the defaults.
fn site_password(host: &str, rules: Option<&str>) -> Result<SitePassword, WireError> {
    if host.len() > MAX_HOST_BYTES || rules.is_some_and(|r| r.len() > MAX_RULES_BYTES) {
        return Err(WireError::InvalidRequest);
    }
    if let Some(rules) = rules.filter(|r| !r.trim().is_empty())
        && let Ok(password) = PasswordRules::parse(rules).generate_default()
    {
        return Ok((Ok(password), Some(RulesSource::Page)));
    }
    if let Some(site) = rules_for_domain(host)
        && let Ok(password) = site.rules().generate_default()
    {
        return Ok((Ok(password), Some(RulesSource::SiteQuirks)));
    }
    Ok((generate_password(&PasswordOptions::default()), None))
}
