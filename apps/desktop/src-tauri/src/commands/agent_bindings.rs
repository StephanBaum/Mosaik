//! Agent bindings in the item IPC (ADR-0159, builder checks B1 and B4).
//!
//! A binding is the policy that lets agents use one secret of an item: the
//! field, the `https` destinations and how the value is sent. It travels with
//! the item (and syncs), and a **device mark** says the user confirmed exactly
//! this binding on this device (B1). Saving a binding in this device's editor
//! counts as the review; a binding that arrived by sync or changed on another
//! device shows "review" until the user confirms it here.
//!
//! No value crosses this module: the checks read the bound field in Rust and
//! drop it at once, and a view lists names, destinations and states only.

use mosaic_agent::binding::{Binding, BindingError, BindingField, check_value};
use mosaic_agent::log::{LogEntry, LogKind};
use mosaic_core::model::{
    AgentBasicUser, AgentBindingRecord, AgentInject, CustomFieldValue, ElementId, Item,
    SecretField, Text, TextField,
};
use mosaic_core::{ItemId, VaultId};
use mosaic_origin::AgentOrigin;
use mosaic_vault::session::UnlockedSession;
use tauri::State;

use super::draft::Prepared;
use crate::contract::agent::{
    AgentBasicUserKind, AgentBasicUserView, AgentBindingChange, AgentBindingConfirmRequest,
    AgentBindingSnapshot, AgentBindingView, AgentInjectKind, AgentInjectView,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::{ItemDetail, SecretFieldKey, SecretRef, TextFieldKey};
use crate::state::{AppState, Backend};

/// How many log entries are read to find what the user last confirmed.
const LAST_CONFIRMED_SCAN: usize = 500;

/// `agent_binding_confirm`.
#[tauri::command]
pub async fn agent_binding_confirm(
    state: State<'_, AppState>,
    request: AgentBindingConfirmRequest,
) -> IpcResult<ItemDetail> {
    state.run(move |b| b.agent_binding_confirm(&request)).await
}

fn error(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}

fn binding_error(e: BindingError) -> IpcError {
    match e {
        BindingError::Origin(_) => error(ErrorCode::AgentOriginInvalid),
        BindingError::ValueTooShort | BindingError::ValueHasControl => {
            error(ErrorCode::AgentValueUnusable)
        }
        _ => error(ErrorCode::AgentBindingInvalid),
    }
}

/// One checked change, not yet applied.
pub(super) enum BindingOp {
    /// A new binding.
    Add(Box<AgentBindingRecord>, BindingField),
    /// Replaces the binding `id`'s field, destinations, injection and
    /// variable name; anything else the stored record holds is kept.
    Update(ElementId, Box<AgentBindingRecord>, BindingField),
    /// Removes the binding.
    Remove(ElementId),
}

/// The model key (`api_key.key_id`) of a text field named on the wire (`apiKeyId`).
fn text_key_to_model(wire: &str) -> Option<String> {
    serde_json::from_value::<TextFieldKey>(serde_json::Value::String(wire.to_owned()))
        .ok()
        .map(|key| key.to_model().key().to_owned())
}

/// The wire name of a text field stored under its model key; a key this build does not know
/// is shown as stored.
fn text_key_to_wire(model: &str) -> String {
    TextField::from_key(model)
        .map(TextFieldKey::from_model)
        .and_then(|key| serde_json::to_value(key).ok())
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| model.to_owned())
}

fn element(id: &str) -> IpcResult<ElementId> {
    ElementId::from_stored(id).map_err(|_| error(ErrorCode::AgentBindingInvalid))
}

fn field_key(field: &SecretRef) -> IpcResult<String> {
    match field {
        SecretRef::Field { field } => Ok(field.to_model().key().to_owned()),
        SecretRef::CustomField { element_id } => {
            element(element_id)?;
            Ok(format!("field:{element_id}"))
        }
        SecretRef::History { .. } => Err(error(ErrorCode::AgentBindingInvalid)),
    }
}

fn inject_of(view: &AgentInjectView) -> IpcResult<AgentInject> {
    let bad = || error(ErrorCode::AgentBindingInvalid);
    Ok(match view.kind {
        AgentInjectKind::Bearer => AgentInject::Bearer,
        AgentInjectKind::Header => AgentInject::Header {
            name: Text::new(view.header_name.as_deref().ok_or_else(bad)?),
            prefix: Text::new(view.header_prefix.as_deref().unwrap_or_default()),
        },
        AgentInjectKind::Basic => {
            let user = view.basic_user.as_ref().ok_or_else(bad)?;
            let value = user.value.as_deref();
            AgentInject::Basic {
                user: match (user.kind, value) {
                    (AgentBasicUserKind::Literal, Some(v)) => AgentBasicUser::Literal(Text::new(v)),
                    (AgentBasicUserKind::Field, Some(v)) => {
                        AgentBasicUser::Field(Text::new(&text_key_to_model(v).ok_or_else(bad)?))
                    }
                    (AgentBasicUserKind::SecretIsUser, None) => AgentBasicUser::SecretIsUser,
                    _ => return Err(bad()),
                },
            }
        }
        AgentInjectKind::Query => AgentInject::Query {
            name: Text::new(view.query_name.as_deref().ok_or_else(bad)?),
        },
    })
}

fn record_of(
    field: &SecretRef,
    origins: &[String],
    inject: &AgentInjectView,
    env: Option<&str>,
) -> IpcResult<(AgentBindingRecord, BindingField)> {
    let mut canonical = Vec::with_capacity(origins.len());
    for origin in origins {
        let parsed =
            AgentOrigin::from_input(origin).map_err(|_| error(ErrorCode::AgentOriginInvalid))?;
        canonical.push(parsed.as_str().to_owned());
    }
    let refs: Vec<&str> = canonical.iter().map(String::as_str).collect();
    let mut record = AgentBindingRecord::new(&field_key(field)?, &refs, inject_of(inject)?);
    record.env = env.filter(|e| !e.is_empty()).map(Text::new);
    let binding = Binding::from_record(&record).map_err(binding_error)?;
    Ok((record, binding.field))
}

/// Checks the shape of the draft's binding changes (no session needed).
///
/// # Errors
/// `agentOriginInvalid`, `agentBindingInvalid`.
pub(super) fn prepare_bindings(changes: Vec<AgentBindingChange>) -> IpcResult<Vec<BindingOp>> {
    if changes.len() > super::draft::MAX_DRAFT_ENTRIES {
        return Err(error(ErrorCode::InvalidInput));
    }
    changes
        .into_iter()
        .map(|change| match change {
            AgentBindingChange::Add {
                field,
                origins,
                inject,
                env,
            } => {
                let (record, parsed) = record_of(&field, &origins, &inject, env.as_deref())?;
                Ok(BindingOp::Add(Box::new(record), parsed))
            }
            AgentBindingChange::Update {
                element_id,
                field,
                origins,
                inject,
                env,
            } => {
                let (record, parsed) = record_of(&field, &origins, &inject, env.as_deref())?;
                Ok(BindingOp::Update(
                    element(&element_id)?,
                    Box::new(record),
                    parsed,
                ))
            }
            AgentBindingChange::Remove { element_id } => {
                Ok(BindingOp::Remove(element(&element_id)?))
            }
        })
        .collect()
}

/// B4: the vault belongs to this account and is not shared.
fn vault_eligible(session: &UnlockedSession, vault: VaultId) -> bool {
    session.owns_vault(vault).unwrap_or(false) && !session.vault_shared(vault).unwrap_or(true)
}

/// Whether the bound field holds a value an agent may use (12 bytes or more,
/// no CR, LF or NUL). The value is read and dropped.
fn value_qualifies(item: &Item, field: &BindingField) -> bool {
    match field {
        BindingField::Secret(f) => matches!(
            item.secret(*f),
            Ok(Some(value)) if check_value(value.expose_secret()).is_ok()
        ),
        BindingField::Custom(id) => item.custom_fields().iter().any(|(cid, c)| {
            cid == id
                && matches!(&c.value, CustomFieldValue::Hidden(v) if check_value(v.expose_secret()).is_ok())
        }),
    }
}

/// The same check against the value the draft will leave in the field.
fn draft_value_qualifies(
    prepared: &Prepared,
    current: Option<&Item>,
    field: &BindingField,
) -> bool {
    let post = match field {
        BindingField::Secret(f) => prepared.post_secret(*f),
        BindingField::Custom(id) => prepared.post_custom(id),
    };
    match post {
        Some(Some(value)) => check_value(value.expose_secret()).is_ok(),
        Some(None) => false,
        None => current.is_some_and(|item| value_qualifies(item, field)),
    }
}

/// The checks of a draft's binding changes that need the session: agent
/// access is on, the vault is eligible (B4), the bound value qualifies and an
/// updated or removed binding exists.
///
/// # Errors
/// `agentAccessOff`, `agentVaultShared`, `agentValueUnusable`,
/// `agentBindingInvalid`.
pub(super) fn check_ops(
    enabled: bool,
    session: &UnlockedSession,
    vault: VaultId,
    current: Option<&Item>,
    prepared: &Prepared,
    ops: &[BindingOp],
) -> IpcResult<()> {
    if ops.is_empty() {
        return Ok(());
    }
    if !enabled {
        return Err(error(ErrorCode::AgentAccessOff));
    }
    if !vault_eligible(session, vault) {
        return Err(error(ErrorCode::AgentVaultShared));
    }
    let existing: Vec<ElementId> = current
        .map(|item| {
            item.agent_bindings()
                .into_iter()
                .map(|(id, _)| id)
                .collect()
        })
        .unwrap_or_default();
    for op in ops {
        match op {
            BindingOp::Add(_, field) => {
                if !draft_value_qualifies(prepared, current, field) {
                    return Err(error(ErrorCode::AgentValueUnusable));
                }
            }
            BindingOp::Update(id, _, field) => {
                if !existing.contains(id) {
                    return Err(error(ErrorCode::AgentBindingInvalid));
                }
                if !draft_value_qualifies(prepared, current, field) {
                    return Err(error(ErrorCode::AgentValueUnusable));
                }
            }
            BindingOp::Remove(id) => {
                if !existing.contains(id) {
                    return Err(error(ErrorCode::AgentBindingInvalid));
                }
            }
        }
    }
    Ok(())
}

/// Applies the checked changes to the item being edited; returns the
/// bindings the user just saved (they are confirmed on this device).
///
/// # Errors
/// The model's errors.
pub(super) fn apply_ops(
    ops: Vec<BindingOp>,
    edit: &mut mosaic_core::model::ItemEdit<'_>,
    current: &[(ElementId, AgentBindingRecord)],
    touched: &mut Vec<ElementId>,
) -> mosaic_core::Result<()> {
    for op in ops {
        match op {
            BindingOp::Add(record, _) => touched.push(edit.add_agent_binding(&record)?),
            BindingOp::Update(id, new, _) => {
                let mut record = current
                    .iter()
                    .find(|(cid, _)| *cid == id)
                    .map(|(_, r)| r.dup())
                    .ok_or(mosaic_core::Error::Malformed)?;
                record.field = Text::new(new.field.as_str());
                record.origins = new.origins.iter().map(|o| Text::new(o.as_str())).collect();
                record.inject = new.inject;
                record.env = new.env.as_ref().map(|e| Text::new(e.as_str()));
                // Phase 1: commands never receive the value.
                record.run = false;
                edit.set_agent_binding(&id, &record)?;
                touched.push(id);
            }
            BindingOp::Remove(id) => edit.remove_agent_binding(&id)?,
        }
    }
    Ok(())
}

fn label_of(item: &Item, record: &AgentBindingRecord) -> String {
    match record.field.as_str().strip_prefix("field:") {
        Some(id) => item
            .custom_fields()
            .into_iter()
            .find(|(cid, _)| cid.as_str() == id)
            .map_or_else(
                || record.field.as_str().to_owned(),
                |(_, c)| c.label.as_str().to_owned(),
            ),
        None => record.field.as_str().to_owned(),
    }
}

fn inject_kind(inject: &AgentInject) -> mosaic_agent::binding::InjectKind {
    use mosaic_agent::binding::InjectKind;
    match inject {
        AgentInject::Bearer => InjectKind::Bearer,
        AgentInject::Header { .. } => InjectKind::Header,
        AgentInject::Basic { .. } => InjectKind::Basic,
        AgentInject::Query { .. } => InjectKind::Query,
    }
}

fn wire_kind(kind: mosaic_agent::binding::InjectKind) -> AgentInjectKind {
    use mosaic_agent::binding::InjectKind;
    match kind {
        InjectKind::Bearer => AgentInjectKind::Bearer,
        InjectKind::Header => AgentInjectKind::Header,
        InjectKind::Basic => AgentInjectKind::Basic,
        InjectKind::Query => AgentInjectKind::Query,
    }
}

fn inject_view(inject: &AgentInject) -> AgentInjectView {
    let mut view = AgentInjectView {
        kind: wire_kind(inject_kind(inject)),
        header_name: None,
        header_prefix: None,
        basic_user: None,
        query_name: None,
    };
    match inject {
        AgentInject::Bearer => {}
        AgentInject::Header { name, prefix } => {
            view.header_name = Some(name.as_str().to_owned());
            view.header_prefix = Some(prefix.as_str().to_owned());
        }
        AgentInject::Basic { user } => {
            view.basic_user = Some(match user {
                AgentBasicUser::Literal(t) => AgentBasicUserView {
                    kind: AgentBasicUserKind::Literal,
                    value: Some(t.as_str().to_owned()),
                },
                AgentBasicUser::Field(t) => AgentBasicUserView {
                    kind: AgentBasicUserKind::Field,
                    value: Some(text_key_to_wire(t.as_str())),
                },
                AgentBasicUser::SecretIsUser => AgentBasicUserView {
                    kind: AgentBasicUserKind::SecretIsUser,
                    value: None,
                },
            });
        }
        AgentInject::Query { name } => view.query_name = Some(name.as_str().to_owned()),
    }
    view
}

fn field_ref(record: &AgentBindingRecord) -> Option<SecretRef> {
    let key = record.field.as_str();
    match key.strip_prefix("field:") {
        Some(id) => Some(SecretRef::CustomField {
            element_id: id.to_owned(),
        }),
        None => SecretField::from_key(key).map(|field| SecretRef::Field {
            field: SecretFieldKey::from_model(field),
        }),
    }
}

/// The binding's destinations and injection at its newest `BindingConfirmed`
/// log entry for this item and field (`None`: never confirmed here, or the
/// entry is beyond the newest [`LAST_CONFIRMED_SCAN`]).
fn last_confirmed(
    session: &UnlockedSession,
    item_id: ItemId,
    label: &str,
) -> Option<AgentBindingSnapshot> {
    let page = session.agent_log_page(None, LAST_CONFIRMED_SCAN).ok()?;
    let item = item_id.to_string();
    page.entries.iter().find_map(|(_, plain)| {
        let entry = LogEntry::from_json(plain.expose_secret())?;
        (entry.kind == LogKind::BindingConfirmed
            && entry.item_id.as_deref() == Some(item.as_str())
            && entry.field_label.as_deref() == Some(label))
        .then(|| AgentBindingSnapshot {
            origins: entry.origins.clone().unwrap_or_default(),
            inject: entry.inject.map_or(AgentInjectKind::Bearer, wire_kind),
        })
    })
}

/// The canonical form the mark covers; `None` for a record without one.
fn canonical(record: &AgentBindingRecord) -> Option<Vec<u8>> {
    record
        .canonical_json()
        .ok()
        .map(|c| c.expose_secret().to_vec())
}

/// The views of an item's bindings.
pub(super) fn views(
    session: &UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
    item: &Item,
) -> Vec<AgentBindingView> {
    let eligible = vault_eligible(session, vault_id);
    item.agent_bindings()
        .into_iter()
        .filter_map(|(id, record)| {
            let field = field_ref(&record)?;
            let canon = canonical(&record);
            let confirmed = canon.as_ref().is_some_and(|c| {
                session
                    .agent_mark_confirmed(vault_id, item_id, id.as_str(), c)
                    .unwrap_or(false)
            });
            let parsed = Binding::from_record(&record).ok();
            let usable = eligible
                && confirmed
                && parsed
                    .as_ref()
                    .is_some_and(|b| value_qualifies(item, &b.field));
            let last = (!confirmed)
                .then(|| last_confirmed(session, item_id, &label_of(item, &record)))
                .flatten();
            Some(AgentBindingView {
                element_id: id.as_str().to_owned(),
                field,
                origins: record
                    .origins
                    .iter()
                    .map(|o| o.as_str().to_owned())
                    .collect(),
                inject: inject_view(&record.inject),
                env: record.env.as_ref().map(|e| e.as_str().to_owned()),
                confirmed,
                last_confirmed: last,
                usable,
            })
        })
        .collect()
}

/// Confirms the bindings `touched` of the item on this device, drops the
/// marks of bindings that are gone or changed, and logs each confirmation
/// (origins and injection kind only).
///
/// # Errors
/// A store error (the binding then stays unconfirmed, which is the safe
/// state).
pub(super) fn confirm_touched(
    session: &mut UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
    touched: &[ElementId],
    now_ms: u64,
) -> IpcResult<()> {
    let (current, entries) = {
        let item = session.item(vault_id, item_id)?;
        let bindings = item.agent_bindings();
        let mut current: Vec<(String, Vec<u8>)> = Vec::new();
        let mut entries: Vec<(String, Vec<u8>, LogEntry)> = Vec::new();
        let title = item.title().unwrap_or_default().to_owned();
        for (id, record) in &bindings {
            let Some(canon) = canonical(record) else {
                continue;
            };
            if touched.contains(id) {
                let mut entry = LogEntry::new(LogKind::BindingConfirmed, now_ms);
                entry.vault_id = Some(vault_id.to_string());
                entry.item_id = Some(item_id.to_string());
                entry.item_title = Some(title.clone());
                entry.field_label = Some(label_of(item, record));
                entry.origins = Some(
                    record
                        .origins
                        .iter()
                        .map(|o| o.as_str().to_owned())
                        .collect(),
                );
                entry.inject = Some(inject_kind(&record.inject));
                entries.push((id.as_str().to_owned(), canon.clone(), entry));
            }
            current.push((id.as_str().to_owned(), canon));
        }
        (current, entries)
    };
    for (id, canon, entry) in &entries {
        session.confirm_agent_mark(vault_id, item_id, id, canon)?;
        session.append_agent_log(now_ms, &entry.to_json())?;
    }
    let keep: Vec<(&str, &[u8])> = current
        .iter()
        .map(|(id, canon)| (id.as_str(), canon.as_slice()))
        .collect();
    session.retain_agent_marks(vault_id, item_id, &keep)?;
    Ok(())
}

impl Backend {
    /// Whether the user switched agent access on (the setting reads `false`
    /// while locked).
    #[must_use]
    pub(crate) fn agent_access_on(&self) -> bool {
        self.stored.lock().is_ok_and(|s| s.agent_access_enabled)
    }

    /// The detail of an item with its agent bindings (empty unless agent
    /// access is on).
    pub(super) fn detail_in(
        &self,
        session: &UnlockedSession,
        vault_id: VaultId,
        item_id: ItemId,
    ) -> IpcResult<ItemDetail> {
        let item = session.item(vault_id, item_id)?;
        let mut detail = super::items::detail(session.store(), vault_id, item_id, item)?;
        if self.agent_access_on() {
            detail.agent_bindings = views(session, vault_id, item_id, item);
        }
        Ok(detail)
    }

    /// The user reviewed a binding on this device: it becomes usable.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `agentAccessOff`, `agentVaultShared`,
    /// `agentBindingInvalid`, `agentOriginInvalid`, `agentValueUnusable`.
    pub fn agent_binding_confirm(
        &self,
        request: &AgentBindingConfirmRequest,
    ) -> IpcResult<ItemDetail> {
        self.touch();
        if !self.agent_access_on() {
            return Err(error(ErrorCode::AgentAccessOff));
        }
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let id = element(&request.element_id)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        if !vault_eligible(session, vault_id) {
            return Err(error(ErrorCode::AgentVaultShared));
        }
        {
            let item = session.item(vault_id, item_id)?;
            let record = item
                .agent_bindings()
                .into_iter()
                .find(|(bid, _)| *bid == id)
                .map(|(_, r)| r)
                .ok_or_else(|| error(ErrorCode::AgentBindingInvalid))?;
            let binding = Binding::from_record(&record).map_err(binding_error)?;
            if !value_qualifies(item, &binding.field) {
                return Err(error(ErrorCode::AgentValueUnusable));
            }
        }
        confirm_touched(
            session,
            vault_id,
            item_id,
            &[id],
            u64::try_from(crate::browser::unix_millis()).unwrap_or(0),
        )?;
        self.detail_in(session, vault_id, item_id)
    }
}
