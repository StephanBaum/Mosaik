//! The vault side of the guided password change (M7b, ADR-0066 to ADR-0068): which attempts a
//! page may see, the two values a change form gets, and what each of the three dialogs decides.
//!
//! Every origin decision goes through `mosaic-origin` and is **exact and https only**: an eTLD+1
//! match is `originMismatch` here, and the per-item HTTP override does not apply. The extension
//! never writes: `rotationBegin`, `rotationResult` and `rotationAddOrigin` only open a dialog,
//! and the user's answer in it writes the vault. `fillChange` and `fillPending` hand over the
//! values of **one** item and mark the attempt delivered before they answer.

use mosaic_core::model::{
    LoginUrl, MAX_OPEN_ATTEMPTS, RotationAttemptId, RotationStage, RulesSource, SecretField,
    UrlMatch,
};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_generator::{PasswordOptions, generate_password};
use mosaic_insights::change_url;
use mosaic_native_messaging::bodies::RulesSource as WireRulesSource;
use mosaic_native_messaging::bodies::{
    AddOriginOutcome, AttemptStage, BeginOutcome, ChangeShape, FillChangeBody, FillChangeResult,
    FillPendingBody, FillPendingResult, FrameKind, ListForOriginBody, ReportedOutcome,
    ResultOutcome, RotationAddOriginBody, RotationAddOriginResult, RotationBeginBody,
    RotationBeginResult, RotationForOriginResult, RotationMatch, RotationResultBody,
    RotationResultResult,
};
use mosaic_native_messaging::protocol::{ErrorCode as WireError, Secret};
use mosaic_origin::{MatchKind, PageOrigin, idn_unicode, registrable_domain, saved_origin};
use mosaic_vault::session::{RotationOutcome, UnlockedSession};

use super::fill::{
    MAX_RULES_BYTES, best_match, login_exists, non_empty, parse_ids, parse_origin, site_password,
    username_of, wire,
};
use super::rotation_prompt::{RotationAnswer, RotationPlan, RotationPlanKind};
use super::save::check_writable;
use crate::contract::browser::BrowserRotationDecision;
use crate::contract::credential_rotation::RotationOutcomeKey;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::Backend;

/// The longest attempt ID (the key suffix), in bytes.
const MAX_ATTEMPT_ID_BYTES: usize = 64;
/// The longest title or user name sent to the extension, in characters.
const MAX_SHOWN_CHARS: usize = 128;
/// A generated password below this many bits of entropy (a page's rules that allow almost
/// nothing) is replaced by the default generator's.
const MIN_ENTROPY_BITS: f64 = 64.0;

/// Parses an attempt ID: the key suffix of the item field, at most 64 ASCII characters.
fn attempt_id(text: &str) -> Result<RotationAttemptId, WireError> {
    if text.len() > MAX_ATTEMPT_ID_BYTES || !text.is_ascii() {
        return Err(WireError::InvalidRequest);
    }
    RotationAttemptId::from_stored(text).map_err(|_| WireError::InvalidRequest)
}

fn clip(text: Option<String>) -> Option<String> {
    text.map(|t| t.chars().take(MAX_SHOWN_CHARS).collect())
}

/// The frame rules of `fill`; a request without a frame field comes from the top frame only.
fn check_frame(
    page: &PageOrigin,
    top: &PageOrigin,
    frame: Option<FrameKind>,
) -> Result<(), WireError> {
    if page == top {
        return Ok(());
    }
    match frame {
        Some(FrameKind::Top) => Err(WireError::InvalidRequest),
        Some(FrameKind::SameOriginFrame) | None => Err(WireError::CrossOriginFrame),
    }
}

/// The page is https and the login (a live login outside the trash) has a saved URL at exactly
/// this origin.
fn exact_login(
    session: &UnlockedSession,
    vault: VaultId,
    item: ItemId,
    page: &PageOrigin,
) -> Result<(), WireError> {
    if page.is_http() {
        return Err(WireError::InsecureOrigin);
    }
    login_exists(session, vault, item)?;
    let found = session
        .item(vault, item)
        .map_err(|_| WireError::ItemNotFound)?;
    match best_match(found, page) {
        Some(m) if m.kind == MatchKind::ExactOrigin && !m.blocked => Ok(()),
        _ => Err(WireError::OriginMismatch),
    }
}

/// This account may change items of the vault, and the vault is open and holds logins.
fn writable(session: &UnlockedSession, vault: VaultId) -> Result<(), WireError> {
    use mosaic_core::model::Role;
    if !matches!(
        session.vault_own_role(vault),
        Ok(Role::Owner | Role::Manage | Role::Write)
    ) {
        return Err(WireError::VaultUnavailable);
    }
    check_writable(session, vault).map_err(wire)
}

fn ipc(error: mosaic_vault::session::Error) -> WireError {
    wire(IpcError::from(error))
}

const fn stage_of(stage: RotationStage) -> AttemptStage {
    match stage {
        RotationStage::Prepared => AttemptStage::Prepared,
        RotationStage::Delivered => AttemptStage::Delivered,
    }
}

/// The change page of a login, if it stays on the page's registrable domain (https only): the
/// extension re-checks the same rule.
fn change_url_for(item: &mosaic_core::model::Item, page: &PageOrigin) -> Option<String> {
    let url = change_url::resolve(item)?.url;
    let target = saved_origin(&url)?;
    let same_site = !target.is_http()
        && registrable_domain(&target.host_str())
            .zip(registrable_domain(&page.host_str()))
            .is_some_and(|(a, b)| a == b);
    same_site.then_some(url)
}

/// A password for a page: its rules if they work and are not weak, else the vendored quirks, else
/// the defaults. A page cannot talk the desktop into a weak password.
pub(super) fn rotation_password(
    host: &str,
    rules: Option<&str>,
) -> Result<(SecretString, Option<RulesSource>), WireError> {
    let (generated, source) = site_password(host, rules)?;
    let generated = generated.map_err(|_| WireError::Internal)?;
    let (generated, source) = if generated.entropy().bits() >= MIN_ENTROPY_BITS {
        (generated, source)
    } else {
        (
            generate_password(&PasswordOptions::default()).map_err(|_| WireError::Internal)?,
            None,
        )
    };
    let secret =
        SecretString::from_str_copy(generated.expose_secret()).map_err(|_| WireError::Internal)?;
    Ok((
        secret,
        source.map(|s| match s {
            WireRulesSource::Page => RulesSource::Page,
            WireRulesSource::SiteQuirks => RulesSource::SiteQuirks,
        }),
    ))
}

/// Replaces the generated password of a `begin` plan (a decision that does not fit is refused
/// before).
pub(super) fn regenerate(kind: &mut RotationPlanKind, host: &str) -> Result<(), WireError> {
    let RotationPlanKind::Begin {
        generated,
        rules,
        rules_source,
    } = kind
    else {
        return Err(WireError::InvalidRequest);
    };
    let (fresh, source) = rotation_password(host, rules.as_deref())?;
    *generated = fresh;
    *rules_source = source;
    Ok(())
}

/// Whether `decision` is one the open request can take (`cancel` fits all).
pub(super) const fn valid_decision(
    kind: &RotationPlanKind,
    decision: BrowserRotationDecision,
) -> bool {
    use BrowserRotationDecision as D;
    match (kind, decision) {
        (_, D::Cancel {})
        | (RotationPlanKind::Begin { .. }, D::Allow {} | D::Regenerate {})
        | (RotationPlanKind::Result { .. }, D::KeepOld {})
        | (RotationPlanKind::AddOrigin { .. }, D::AddOrigin {}) => true,
        (RotationPlanKind::Result { reported, .. }, D::Confirm {}) => {
            matches!(reported, RotationOutcomeKey::Succeeded)
        }
        _ => false,
    }
}

/// The two values of one open attempt, copied out of the session before it is borrowed again.
struct Values {
    current: Option<SecretString>,
    pending: SecretString,
    username: Option<String>,
}

fn values_of(
    session: &UnlockedSession,
    vault: VaultId,
    item: ItemId,
    attempt: RotationAttemptId,
) -> Result<Values, WireError> {
    let found = session
        .item(vault, item)
        .map_err(|_| WireError::ItemNotFound)?;
    let pending = found
        .rotation_attempts()
        .into_iter()
        .find(|a| a.id == attempt)
        .ok_or(WireError::RotationNotFound)?
        .pending;
    let current = found
        .secret(SecretField::Password)
        .map_err(|_| WireError::Internal)?
        .filter(|p| !p.expose_secret().is_empty());
    Ok(Values {
        current,
        pending,
        username: username_of(found),
    })
}

fn to_wire(secret: &SecretString) -> Secret {
    Secret::new(secret.expose_secret().to_owned())
}

impl Backend {
    /// `rotationForOrigin`: the open attempts of the logins saved for exactly this https origin
    /// (at most 5, newest first, no secret).
    pub(crate) fn browser_rotation_for_origin(
        &self,
        body: &ListForOriginBody,
    ) -> Result<RotationForOriginResult, WireError> {
        let page = parse_origin(&body.origin)?;
        parse_origin(&body.top_origin)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        let attempts = if page.is_http() {
            Vec::new()
        } else {
            session
                .rotation_attempts_for_origin(&page)
                .into_iter()
                .map(|r| RotationMatch {
                    change_url: session
                        .item(r.vault_id, r.item_id)
                        .ok()
                        .and_then(|item| change_url_for(item, &page)),
                    vault_id: r.vault_id.to_string(),
                    item_id: r.item_id.to_string(),
                    attempt_id: r.attempt_id.to_string(),
                    title: clip(non_empty(r.title.as_deref())),
                    username: clip(non_empty(r.username.as_deref())),
                    host: page.host_str(),
                    stage: stage_of(r.stage),
                    started_at: r.started_at.as_unix_millis(),
                })
                .collect()
        };
        Ok(RotationForOriginResult { attempts })
    }

    /// The checks every value-bearing op makes, then the values and the delivered mark.
    fn deliver(
        &self,
        ids: (&str, &str, &str),
        origins: (&str, &str),
        frame: FrameKind,
    ) -> Result<Values, WireError> {
        let page = parse_origin(origins.0)?;
        let top = parse_origin(origins.1)?;
        check_frame(&page, &top, Some(frame))?;
        let (vault, item) = parse_ids(ids.0, ids.1)?;
        let attempt = attempt_id(ids.2)?;
        let mut slot = self.slot().map_err(wire)?;
        let session = slot.unlocked_mut().map_err(wire)?;
        exact_login(session, vault, item, &page)?;
        writable(session, vault)?;
        let values = values_of(session, vault, item, attempt)?;
        // Marked before the values leave: a crash after the fill finds the attempt delivered.
        session
            .mark_rotation_delivered(vault, item, attempt)
            .map_err(ipc)?;
        self.items_changed(session);
        self.touch();
        Ok(values)
    }

    /// `fillChange`: the current and the new password of one attempt (`newOnly`: the new one).
    pub(crate) fn browser_fill_change(
        &self,
        body: &FillChangeBody,
    ) -> Result<FillChangeResult, WireError> {
        let values = self.deliver(
            (&body.vault_id, &body.item_id, &body.attempt_id),
            (&body.origin, &body.top_origin),
            body.frame,
        )?;
        let current = match body.shape {
            ChangeShape::CurrentAndNew => {
                Some(values.current.as_ref().ok_or(WireError::FieldEmpty)?)
            }
            ChangeShape::NewOnly => None,
        };
        Ok(FillChangeResult {
            current: current.map(to_wire),
            new: to_wire(&values.pending),
        })
    }

    /// `fillPending`: the new password of one attempt and the user name, to sign in after a
    /// change the site accepted but the user has not confirmed.
    pub(crate) fn browser_fill_pending(
        &self,
        body: &FillPendingBody,
    ) -> Result<FillPendingResult, WireError> {
        let values = self.deliver(
            (&body.vault_id, &body.item_id, &body.attempt_id),
            (&body.origin, &body.top_origin),
            body.frame,
        )?;
        Ok(FillPendingResult {
            username: values.username,
            password: to_wire(&values.pending),
        })
    }

    /// What `rotationBegin` shows: the checks, then a password generated for the page.
    pub(crate) fn browser_plan_rotation_begin(
        &self,
        body: &RotationBeginBody,
    ) -> Result<RotationPlan, WireError> {
        if body
            .rules
            .as_deref()
            .is_some_and(|r| r.len() > MAX_RULES_BYTES)
        {
            return Err(WireError::InvalidRequest);
        }
        let page = parse_origin(&body.origin)?;
        let top = parse_origin(&body.top_origin)?;
        check_frame(&page, &top, None)?;
        let (vault, item) = parse_ids(&body.vault_id, &body.item_id)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        exact_login(session, vault, item, &page)?;
        writable(session, vault)?;
        let found = session
            .item(vault, item)
            .map_err(|_| WireError::ItemNotFound)?;
        if found.rotation_attempts().len() >= MAX_OPEN_ATTEMPTS {
            return Err(WireError::RotationLimit);
        }
        let host = page.host_str();
        let (generated, rules_source) = rotation_password(&host, body.rules.as_deref())?;
        Ok(plan(
            RotationPlanKind::Begin {
                generated,
                rules: body.rules.clone(),
                rules_source,
            },
            (vault, item),
            &page,
            clip(non_empty(found.title())),
        ))
    }

    /// What `rotationResult` shows: the attempt must be open.
    pub(crate) fn browser_plan_rotation_result(
        &self,
        body: &RotationResultBody,
    ) -> Result<RotationPlan, WireError> {
        let page = parse_origin(&body.origin)?;
        let top = parse_origin(&body.top_origin)?;
        check_frame(&page, &top, None)?;
        let (vault, item) = parse_ids(&body.vault_id, &body.item_id)?;
        let attempt = attempt_id(&body.attempt_id)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        exact_login(session, vault, item, &page)?;
        writable(session, vault)?;
        let found = session
            .item(vault, item)
            .map_err(|_| WireError::ItemNotFound)?;
        if !found.rotation_attempts().iter().any(|a| a.id == attempt) {
            return Err(WireError::RotationNotFound);
        }
        let reported = match body.outcome {
            ReportedOutcome::Succeeded => RotationOutcomeKey::Succeeded,
            ReportedOutcome::Failed => RotationOutcomeKey::Failed,
        };
        Ok(plan(
            RotationPlanKind::Result { attempt, reported },
            (vault, item),
            &page,
            clip(non_empty(found.title())),
        ))
    }

    /// What `rotationAddOrigin` shows. Only an https, non-loopback origin whose registrable
    /// domain is that of an https URL of the login and of `fromOrigin` (and not a public suffix).
    pub(crate) fn browser_plan_rotation_add_origin(
        &self,
        body: &RotationAddOriginBody,
    ) -> Result<RotationPlan, WireError> {
        let page = parse_origin(&body.origin)?;
        let top = parse_origin(&body.top_origin)?;
        let from = parse_origin(&body.from_origin)?;
        check_frame(&page, &top, None)?;
        let (vault, item) = parse_ids(&body.vault_id, &body.item_id)?;
        let attempt = attempt_id(&body.attempt_id)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        login_exists(session, vault, item)?;
        writable(session, vault)?;
        let found = session
            .item(vault, item)
            .map_err(|_| WireError::ItemNotFound)?;
        if !found.rotation_attempts().iter().any(|a| a.id == attempt) {
            return Err(WireError::RotationNotFound);
        }
        let reg = |origin: &PageOrigin| registrable_domain(&origin.host_str());
        let related = !page.is_http()
            && !page.is_loopback()
            && !from.is_http()
            && reg(&page).is_some_and(|domain| {
                reg(&from) == Some(domain.clone())
                    && found.urls().iter().any(|(_, url)| {
                        saved_origin(url.url.as_str())
                            .is_some_and(|o| !o.is_http() && reg(&o) == Some(domain.clone()))
                    })
            });
        if !related {
            return Err(WireError::OriginNotRelated);
        }
        let saved_exactly = found
            .urls()
            .iter()
            .any(|(_, url)| saved_origin(url.url.as_str()).as_ref() == Some(&page));
        if saved_exactly {
            return Err(WireError::InvalidRequest);
        }
        Ok(plan(
            RotationPlanKind::AddOrigin { attempt },
            (vault, item),
            &page,
            clip(non_empty(found.title())),
        ))
    }

    /// Writes the user's answer to a rotation request. The origin and the attempt are checked
    /// again: the vault may have changed while the dialog was open.
    ///
    /// # Errors
    /// `itemNotFound`, `vaultUnavailable`, `rotationNotFound`, `rotationLimit`,
    /// `sharingRoleForbidden`, `invalidInput`, `locked`.
    pub(crate) fn browser_apply_rotation(
        &self,
        plan: &RotationPlan,
        decision: BrowserRotationDecision,
    ) -> IpcResult<RotationAnswer> {
        let (vault, item) = (plan.vault, plan.item);
        let page = PageOrigin::parse(&plan.origin).map_err(|_| IpcError::INTERNAL)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let gone = || IpcError::new(ErrorCode::ItemNotFound);
        let answer = match (&plan.kind, decision) {
            (
                RotationPlanKind::Begin {
                    generated,
                    rules_source,
                    ..
                },
                BrowserRotationDecision::Allow {},
            ) => {
                exact_login(session, vault, item, &page).map_err(|_| gone())?;
                let id = session.begin_rotation(vault, item, generated, *rules_source)?;
                RotationAnswer::Begin(RotationBeginResult {
                    outcome: BeginOutcome::Started,
                    attempt_id: Some(id.to_string()),
                })
            }
            (
                RotationPlanKind::Result { attempt, .. },
                BrowserRotationDecision::Confirm {} | BrowserRotationDecision::KeepOld {},
            ) => {
                exact_login(session, vault, item, &page).map_err(|_| gone())?;
                let confirm = matches!(decision, BrowserRotationDecision::Confirm {});
                session.close_rotation(
                    vault,
                    item,
                    *attempt,
                    if confirm {
                        RotationOutcome::Succeeded
                    } else {
                        RotationOutcome::Failed
                    },
                )?;
                RotationAnswer::Result(RotationResultResult {
                    outcome: if confirm {
                        ResultOutcome::Confirmed
                    } else {
                        ResultOutcome::KeptOld
                    },
                })
            }
            (RotationPlanKind::AddOrigin { attempt }, BrowserRotationDecision::AddOrigin {}) => {
                login_exists(session, vault, item).map_err(|_| gone())?;
                let open = session
                    .item(vault, item)?
                    .rotation_attempts()
                    .iter()
                    .any(|a| a.id == *attempt);
                if !open {
                    return Err(IpcError::new(ErrorCode::RotationNotFound));
                }
                let mut url = LoginUrl::new(&plan.origin);
                url.match_mode = UrlMatch::ExactOrigin;
                session.update_item(vault, item, |e| e.add_url(&url).map(|_| ()))?;
                RotationAnswer::AddOrigin(RotationAddOriginResult {
                    outcome: AddOriginOutcome::Added,
                })
            }
            _ => return Err(IpcError::new(ErrorCode::InvalidInput)),
        };
        self.items_changed(session);
        Ok(answer)
    }
}

fn plan(
    kind: RotationPlanKind,
    (vault, item): (VaultId, ItemId),
    page: &PageOrigin,
    title: Option<String>,
) -> RotationPlan {
    let host = page.host_str();
    RotationPlan {
        kind,
        vault,
        item,
        origin: page.serialize(),
        title,
        unicode_host: idn_unicode(&host),
        host,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_ids_are_short_ascii() {
        assert!(attempt_id(&"a".repeat(65)).is_err());
        assert!(attempt_id("ä").is_err());
        assert!(attempt_id("").is_err());
    }

    #[test]
    fn frames_follow_the_fill_rules() {
        let a = PageOrigin::parse("https://a.example").unwrap();
        let b = PageOrigin::parse("https://b.example").unwrap();
        assert!(check_frame(&a, &a, None).is_ok());
        assert_eq!(check_frame(&a, &b, None), Err(WireError::CrossOriginFrame));
        assert_eq!(
            check_frame(&a, &b, Some(FrameKind::SameOriginFrame)),
            Err(WireError::CrossOriginFrame)
        );
        assert_eq!(
            check_frame(&a, &b, Some(FrameKind::Top)),
            Err(WireError::InvalidRequest)
        );
    }

    #[test]
    fn a_weak_rule_set_falls_back_to_a_strong_password() {
        let (secret, source) = rotation_password(
            "example.com",
            Some("minlength: 4; maxlength: 4; allowed: digit;"),
        )
        .unwrap();
        assert!(
            secret.expose_secret().chars().count() >= 12,
            "page rules cannot weaken it"
        );
        assert_eq!(source, None);
    }

    #[test]
    fn titles_are_clipped() {
        assert_eq!(clip(Some("x".repeat(300))).unwrap().chars().count(), 128);
        assert_eq!(clip(None), None);
    }
}
