//! Picks, links and the second step of a two-step login (ADR-0164). A fill starts only from
//! `picker_choose` or `picker_link_confirm`, both of which answer a click or Enter in Mosaic's own
//! list, never earlier than [`PICK_MIN_DELAY_MS`] after the list appeared, and only while the list
//! is in front, where it was put and not covered. Everything else the service does only shows or
//! hides a window.

use mosaic_core::{ItemId, VaultId};
use mosaic_program_fill::binding::{block_key, display_for};
use mosaic_program_fill::detect::FieldRole;
use mosaic_program_fill::fill::{Continuation, FillAbort, FillOutcome};
use mosaic_program_fill::host::{FieldOffer, FillRequest, Token};
use mosaic_program_fill::identity::CapturedIdentity;
use mosaic_program_fill::policy::{
    ConfirmStep, Confirmations, LinkAgain, LinkConfirmation, LinkWarning, link_again_needed,
    propose_binding, propose_relink,
};
use mosaic_program_fill::verify::{CONTINUATION_TTL_MS, ProbeError, Rect};

use super::offer::{self, Credentials};
use super::{
    Cont, Effect, LINK_MIN_DELAY_MS, LinkState, PICK_MIN_DELAY_MS, PickerLabel, link, program_view,
    stale,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::picker::{
    HashChange, LinkConfirmResult, LinkStep, PickerGenerationRequest, PickerItemRequest,
    PickerLogin, PickerMode, PickerNotice, PickerSearchRequest,
};
use crate::contract::program_fill::BlockedProgram;
use crate::state::Backend;

fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidInput)
}

/// The first eight bytes of a hash as 16 lowercase hex digits.
fn hex_prefix(hash: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    hash.iter().take(8).fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// What a fill needs from the model once the checks passed.
struct Pick {
    offer: FieldOffer,
    identity: CapturedIdentity,
    /// The generation the checks passed for: only one fill can claim it.
    generation: u64,
}

impl Backend {
    /// The checks every fill starts with, in this order: the generation, then (a pick only) the
    /// mode, that the login is on the list, and the guard time; then that the list is in front,
    /// where it was put, and not covered.
    fn pick_checks(
        &self,
        generation: u64,
        login: Option<(VaultId, ItemId)>,
        guard_ms: Option<(u64, u64)>,
    ) -> IpcResult<Pick> {
        let svc = &self.program_fill;
        let now = svc.now();
        let (offer, list_at) = {
            let m = svc.model();
            if m.generation != generation {
                return Err(stale());
            }
            if m.mode != PickerMode::List || m.fill_running {
                return Err(stale());
            }
            if let Some((vault, item)) = login
                && !m
                    .data
                    .matches
                    .iter()
                    .any(|x| x.login.vault_id.get() == vault && x.login.item_id.get() == item)
            {
                return Err(invalid());
            }
            if let Some((since, min)) = guard_ms
                && now.saturating_sub(since) < min
            {
                return Err(IpcError::new(ErrorCode::PickerTooSoon));
            }
            (
                m.offer.clone().ok_or_else(stale)?,
                m.list_at.ok_or_else(stale)?,
            )
        };
        let windows = svc.windows();
        if !windows.is_foreground(PickerLabel::List)
            || windows.placed(PickerLabel::List) != Some(list_at)
        {
            return Err(stale());
        }
        let hwnd = windows.hwnd(PickerLabel::List).ok_or_else(stale)?;
        let clear = svc.host().map(|h| h.window_clear(hwnd, list_at));
        if clear != Some(Ok(true)) {
            return Err(stale());
        }
        let identity = offer.identity.clone();
        Ok(Pick {
            offer,
            identity,
            generation,
        })
    }

    /// A click or Enter on a login of the list: fills it.
    ///
    /// # Errors
    /// `pickerStale` (the generation changed, or the list is not in front, not where it was put
    /// or covered), `pickerTooSoon` (within 300 ms of the list appearing), `invalidInput` (the
    /// login is not on the list), `itemNotFound`, `locked`.
    pub fn picker_choose(&self, request: &PickerItemRequest) -> IpcResult<()> {
        let svc = &self.program_fill;
        let (vault, item) = (request.vault_id.get(), request.item_id.get());
        let shown_at = svc.model().list_shown_at;
        let pick = self.pick_checks(
            request.generation,
            Some((vault, item)),
            Some((shown_at, PICK_MIN_DELAY_MS)),
        )?;
        self.touch();
        let credentials = {
            let slot = self.slot()?;
            offer::credentials(slot.unlocked()?, vault, item, Some(&pick.identity))?
        };
        self.run_fill(&pick, vault, item, credentials)
    }

    /// Claims the pick, hides both windows, fills, and shows what came of it. Of two picks made
    /// for the same generation (a double Enter) only the first one claims it.
    fn run_fill(
        &self,
        pick: &Pick,
        vault: VaultId,
        item: ItemId,
        credentials: Credentials,
    ) -> IpcResult<()> {
        let svc = &self.program_fill;
        let Some(host) = svc.host() else {
            return Err(IpcError::new(ErrorCode::ProgramFillUnavailable));
        };
        let settings = self.settings_snapshot();
        let has_password = credentials.password.is_some();
        let submit = settings.program_fill_submit && credentials.submit && has_password;
        let role_text = matches!(pick.offer.role, FieldRole::Text);
        let picked_at = svc.now();
        let effects = {
            let mut m = svc.model();
            if m.generation != pick.generation || m.fill_running {
                return Err(stale());
            }
            m.fill_running = true;
            m.mode = PickerMode::Hidden;
            m.generation += 1;
            m.link = None;
            m.list_at = None;
            m.chip_at = None;
            m.chip_visible = false;
            vec![
                Effect::Hide(PickerLabel::Chip),
                Effect::Hide(PickerLabel::List),
            ]
        };
        svc.apply(&effects);
        // A lone text field is the first step of a two-step login: only the user name is written,
        // and the password is not handed over at all.
        let password = if role_text {
            None
        } else {
            credentials.password
        };
        let result = host.fill(FillRequest {
            token: pick.offer.token,
            username: credentials.username,
            password,
            submit,
            picked_at_ms: picked_at,
            continuation: false,
        });
        let mut notice = None;
        match result {
            Ok(FillOutcome::Done { .. }) if role_text => {
                // Only `picked_at_ms` is read: the host takes the window from its capture.
                host.arm_continuation(
                    pick.offer.token,
                    Continuation {
                        root: 0,
                        window_pid: 0,
                        element_pid: 0,
                        picked_at_ms: picked_at,
                        used: false,
                    },
                );
                svc.model().cont = Some(Cont {
                    vault,
                    item,
                    identity: pick.identity.clone(),
                    picked_at,
                    used: false,
                });
            }
            Ok(FillOutcome::Done { .. } | FillOutcome::NothingToFill) => {}
            Ok(FillOutcome::Aborted {
                reason, cleared, ..
            }) => {
                notice = Some(match reason {
                    FillAbort::ModifiersHeld => PickerNotice::ModifiersHeld,
                    FillAbort::Probe(ProbeError::Timeout) => PickerNotice::NotResponding,
                    _ if cleared => PickerNotice::AbortedCleared,
                    _ => PickerNotice::Aborted,
                });
            }
            Err(ProbeError::Timeout) => notice = Some(PickerNotice::NotResponding),
            Err(_) => notice = Some(PickerNotice::Aborted),
        }
        svc.model().fill_running = false;
        if let Some(notice) = notice {
            self.program_fill_notice(notice, Some(pick.offer.rect));
        }
        Ok(())
    }

    /// The second step of a two-step login: the host found the password field of the same
    /// window. Fills it once, within a minute of the first step; anything else is ignored.
    pub(crate) fn program_fill_second_step(&self, token: Token, near: Rect) {
        let svc = &self.program_fill;
        let now = svc.now();
        let cont = {
            let mut m = svc.model();
            let live = m.cont.as_mut().filter(|c| {
                !c.used && now >= c.picked_at && now - c.picked_at < CONTINUATION_TTL_MS
            });
            let Some(live) = live else { return };
            live.used = true;
            live.clone()
        };
        let Some(host) = svc.host() else { return };
        let credentials = self.slot().ok().and_then(|slot| {
            slot.unlocked().ok().and_then(|s| {
                offer::credentials(s, cont.vault, cont.item, Some(&cont.identity)).ok()
            })
        });
        let Some(credentials) = credentials else {
            return;
        };
        let Some(password) = credentials.password else {
            return;
        };
        let submit = self.settings_snapshot().program_fill_submit && credentials.submit;
        svc.model().fill_running = true;
        let result = host.fill(FillRequest {
            token,
            username: None,
            password: Some(password),
            submit,
            picked_at_ms: cont.picked_at,
            continuation: true,
        });
        svc.model().fill_running = false;
        let notice = match result {
            Ok(FillOutcome::Aborted { reason, .. }) => Some(match reason {
                FillAbort::ModifiersHeld => PickerNotice::ModifiersHeld,
                FillAbort::Probe(ProbeError::Timeout) => PickerNotice::NotResponding,
                _ => PickerNotice::Aborted,
            }),
            Err(ProbeError::Timeout) => Some(PickerNotice::NotResponding),
            Err(_) => Some(PickerNotice::Aborted),
            Ok(_) => None,
        };
        if let Some(notice) = notice {
            // After a timeout the host has a new worker that does not know the token: the
            // rectangle of the event is the fallback.
            let near = host.field_rect(token).ok().flatten().unwrap_or(near);
            self.program_fill_notice(notice, Some(near));
        }
    }

    /// The list's search box: logins whose title, user name or web host contains the text.
    ///
    /// # Errors
    /// `pickerStale`, `invalidInput` (more than 128 characters), `locked`.
    pub fn picker_search(&self, request: &PickerSearchRequest) -> IpcResult<Vec<PickerLogin>> {
        {
            let m = self.program_fill.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            if m.mode != PickerMode::List {
                return Err(stale());
            }
        }
        let slot = self.slot()?;
        offer::search(slot.unlocked()?, &request.query)
    }

    /// "Link a login…": opens the link step for a login of the user's choice.
    ///
    /// # Errors
    /// `pickerStale`, `invalidInput` (not a live login in a vault this account may change, or a
    /// program that cannot be linked), `locked`.
    pub fn picker_link_begin(&self, request: &PickerItemRequest) -> IpcResult<LinkStep> {
        let svc = &self.program_fill;
        let (vault, item) = (request.vault_id.get(), request.item_id.get());
        let now = svc.now();
        let identity = {
            let m = svc.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            if m.mode != PickerMode::List {
                return Err(invalid());
            }
            m.offer.as_ref().ok_or_else(invalid)?.identity.clone()
        };
        let (login, proposed, old_hash) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let context = offer::link_context(session, vault, item, &identity)?;
            let bound: Vec<&str> = context.bound_exes.iter().map(String::as_str).collect();
            let again: Option<LinkAgain> = link_again_needed(&context.saved, &identity, None);
            let proposed = match &again {
                Some(again) => propose_relink(&identity, &bound, again),
                None => propose_binding(&identity, &bound),
            }
            .map_err(|_| invalid())?;
            let login = offer::login_of(session, vault, item).ok_or_else(invalid)?;
            (login, proposed, context.old_hash)
        };
        let confirmations = match proposed.confirmations {
            Confirmations::One => 1,
            Confirmations::Two => 2,
        };
        let lookalike_of = proposed.warnings.iter().find_map(|w| match w {
            LinkWarning::LookalikeExe { similar_to } => Some(similar_to.clone()),
            LinkWarning::Unsigned => None,
        });
        let hash_change = old_hash
            .zip(identity.file_sha256)
            .map(|(old, new)| HashChange {
                old_prefix: hex_prefix(&old),
                new_prefix: hex_prefix(&new),
            });
        let view = LinkStep {
            login,
            program: program_view(&identity),
            confirmations,
            confirmed: 0,
            lookalike_of,
            hash_change,
            submit: self.settings_snapshot().program_fill_submit,
        };
        let effects = {
            let mut m = svc.model();
            if m.generation != request.generation || m.mode != PickerMode::List {
                return Err(stale());
            }
            m.link = Some(LinkState {
                view: view.clone(),
                confirmation: LinkConfirmation::new(proposed),
                vault,
                item,
                shown_at: now,
            });
            m.generation += 1;
            vec![Effect::Changed(PickerLabel::List, m.generation)]
        };
        svc.apply(&effects);
        Ok(view)
    }

    /// The confirmation of the link step; the last one links and fills.
    ///
    /// # Errors
    /// `pickerStale` (also the checks of a pick), `invalidInput` (no link step), `locked`,
    /// `sharingRoleForbidden` and the vault's other codes.
    pub fn picker_link_confirm(
        &self,
        request: &PickerGenerationRequest,
    ) -> IpcResult<LinkConfirmResult> {
        let svc = &self.program_fill;
        let now = svc.now();
        let shown_at = {
            let m = svc.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            m.link.as_ref().ok_or_else(invalid)?.shown_at
        };
        if now.saturating_sub(shown_at) < LINK_MIN_DELAY_MS {
            return Err(IpcError::new(ErrorCode::PickerTooSoon));
        }
        let pick = self.pick_checks(request.generation, None, None)?;
        let step = {
            let mut m = svc.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            let link = m.link.as_mut().ok_or_else(invalid)?;
            (link.confirmation.confirm(now), link.vault, link.item)
        };
        let (step, vault, item) = step;
        match step {
            ConfirmStep::NeedAnother => {
                let effects = {
                    let mut m = svc.model();
                    if let Some(link) = m.link.as_mut() {
                        link.view.confirmed = 1;
                    }
                    m.generation += 1;
                    vec![Effect::Changed(PickerLabel::List, m.generation)]
                };
                svc.apply(&effects);
                Ok(LinkConfirmResult::NeedAnother)
            }
            ConfirmStep::TooSoon => Ok(LinkConfirmResult::TooSoon),
            ConfirmStep::Done(url) => {
                self.touch();
                let credentials = {
                    let mut slot = self.slot()?;
                    let session = slot.unlocked_mut()?;
                    link::write(session, vault, item, &pick.identity, url)?;
                    offer::credentials(session, vault, item, None)?
                };
                self.run_fill(&pick, vault, item, credentials)?;
                Ok(LinkConfirmResult::Linked)
            }
        }
    }

    /// "Don't offer for this program": remembers it and closes both windows.
    ///
    /// # Errors
    /// `pickerStale`, `invalidInput` (a program that is never offered), `storeUnavailable`.
    pub fn picker_block(&self, request: &PickerGenerationRequest) -> IpcResult<()> {
        let svc = &self.program_fill;
        let identity = {
            let m = svc.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            m.offer.as_ref().ok_or_else(stale)?.identity.clone()
        };
        let key = block_key(&identity).ok_or_else(invalid)?;
        svc.blocklist().add(BlockedProgram {
            key,
            label: display_for(&identity).name,
        })?;
        let effects = svc.model().clear_view();
        svc.apply(&effects);
        Ok(())
    }

    /// Escape, or the notice's Close: closes the list and puts the focus back into the field.
    ///
    /// # Errors
    /// `pickerStale`.
    pub fn picker_dismiss(&self, request: &PickerGenerationRequest) -> IpcResult<()> {
        let svc = &self.program_fill;
        let (effects, token) = {
            let mut m = svc.model();
            if m.generation != request.generation {
                return Err(stale());
            }
            if m.fill_running {
                return Ok(());
            }
            let token = m.offer.as_ref().map(|o| o.token);
            (m.clear_view(), token)
        };
        svc.apply(&effects);
        if let (Some(token), Some(host)) = (token, svc.host()) {
            let _ = host.refocus(token);
        }
        Ok(())
    }
}
