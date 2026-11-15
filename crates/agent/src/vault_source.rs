//! The vault-backed secret source (agent access design §6, builder checks B1 and B4).
//!
//! A binding is usable when its vault is available, this account owns it and it is not shared
//! (B4: a member's write could change a binding), the item is neither trashed nor deleted, the
//! binding validates, and its field is a secret field with a value or a hidden custom field of the
//! item. Two usable bindings of one item that name the same field conflict and neither is usable.
//!
//! A binding is *confirmed* when this device holds its mark (B1). The source only reads: it never
//! writes a mark, touches an item or counts as activity.

use std::collections::HashSet;

use mosaic_core::model::{AgentBindingRecord, CustomFieldValue, ElementId, Item, SecretField};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_vault::refs::{RefCandidate, RefError, resolve_agent};
use mosaic_vault::session::{ItemFilter, TrashFilter, UnlockedSession};

use crate::binding::{BasicUser, Binding, BindingField, Inject, check_value};
use crate::error::AgentErrorCode;
use crate::source::{ResolveError, SecretSource, SourceBinding};

/// The bindings of one unlocked session.
pub struct VaultSource<'a> {
    session: &'a UnlockedSession,
}

impl<'a> VaultSource<'a> {
    /// A source over `session`.
    #[must_use]
    pub const fn new(session: &'a UnlockedSession) -> Self {
        Self { session }
    }

    /// The vaults whose bindings agents may use, with their display names.
    fn eligible_vaults(&self) -> Vec<(VaultId, String)> {
        self.session
            .vaults()
            .into_iter()
            .filter(|v| v.available)
            .filter(|v| self.is_eligible(v.vault_id))
            .map(|v| {
                (
                    v.vault_id,
                    v.name.map_or_else(|| v.vault_id.to_string(), str::to_owned),
                )
            })
            .collect()
    }

    /// B4: owned by this account and not shared.
    fn is_eligible(&self, vault: VaultId) -> bool {
        self.session.owns_vault(vault).unwrap_or(false)
            && !self.session.vault_shared(vault).unwrap_or(true)
    }

    fn bindings_of(
        &self,
        vault_id: VaultId,
        vault_name: &str,
        item_id: ItemId,
        item: &Item,
    ) -> Vec<SourceBinding> {
        let title = item.title().unwrap_or_default().to_owned();
        let customs = item.custom_fields();
        let mut found: Vec<SourceBinding> = Vec::new();
        for (element_id, record) in item.agent_bindings() {
            let Ok(binding) = Binding::from_record(&record) else {
                continue;
            };
            let Some(field_label) = field_label(item, &customs, &binding.field) else {
                continue;
            };
            let Some(confirmed) = self.confirmed(vault_id, item_id, &element_id, &record) else {
                continue;
            };
            found.push(SourceBinding {
                vault_id,
                item_id,
                element_id,
                vault_name: vault_name.to_owned(),
                item_title: title.clone(),
                field_label,
                binding,
                confirmed,
            });
        }
        // Two bindings on one field conflict: neither is usable until the user fixes it.
        let fields: Vec<BindingField> = found.iter().map(|b| b.binding.field.clone()).collect();
        found.retain(|b| fields.iter().filter(|f| **f == b.binding.field).count() == 1);
        found
    }

    /// `None` when the binding has no canonical form (it is unusable).
    fn confirmed(
        &self,
        vault: VaultId,
        item: ItemId,
        element: &ElementId,
        record: &AgentBindingRecord,
    ) -> Option<bool> {
        let canonical = record.canonical_json().ok()?;
        Some(
            self.session
                .agent_mark_confirmed(vault, item, element.as_str(), canonical.expose_secret())
                .unwrap_or(false),
        )
    }

    fn all(&self) -> Vec<SourceBinding> {
        let mut out = Vec::new();
        for (vault_id, vault_name) in self.eligible_vaults() {
            let mut filter = ItemFilter::default();
            filter.vault = Some(vault_id);
            filter.trash = TrashFilter::Exclude;
            let mut items = self.session.items(&filter);
            items.sort_by_key(|r| r.item_id.to_string());
            for r in items {
                out.extend(self.bindings_of(vault_id, &vault_name, r.item_id, r.item));
            }
        }
        out
    }
}

/// `Some(label)` when the bound field exists on the item: a secret field with a value, or a
/// hidden custom field.
fn field_label(
    item: &Item,
    customs: &[(ElementId, mosaic_core::model::CustomField)],
    field: &BindingField,
) -> Option<String> {
    match field {
        BindingField::Secret(f) => {
            matches!(item.secret(*f), Ok(Some(_))).then(|| f.key().to_owned())
        }
        BindingField::Custom(id) => customs
            .iter()
            .find(|(cid, _)| cid == id)
            .filter(|(_, c)| matches!(c.value, CustomFieldValue::Hidden(_)))
            .map(|(_, c)| c.label.as_str().to_owned()),
    }
}

fn candidate(b: &SourceBinding) -> RefCandidate {
    RefCandidate {
        vault: b.vault_name.clone(),
        title: b.item_title.clone(),
        id: b.item_id,
    }
}

/// The binding of `cands` (all of one item) that `field` names.
fn match_field(cands: &[&SourceBinding], field: &str) -> Result<SourceBinding, ResolveError> {
    let pick = |hits: Vec<&&SourceBinding>| match hits.as_slice() {
        [] => Err(ResolveError::NotUsable),
        [one] => Ok((**one).clone()),
        many => Err(ResolveError::Ambiguous(
            many.iter().map(|b| candidate(b)).collect(),
        )),
    };
    if let Some(rest) = field.strip_prefix("field:") {
        let by_id: Vec<_> = cands
            .iter()
            .filter(|b| matches!(&b.binding.field, BindingField::Custom(id) if id.as_str() == rest))
            .collect();
        if !by_id.is_empty() {
            return pick(by_id);
        }
        let by_label: Vec<_> = cands
            .iter()
            .filter(|b| matches!(b.binding.field, BindingField::Custom(_)) && b.field_label == rest)
            .collect();
        return pick(by_label);
    }
    // The `password` alias of the CLI names the login password.
    let wanted = if field == "password" {
        Some(SecretField::Password)
    } else {
        SecretField::from_key(field)
    };
    let Some(wanted) = wanted else {
        return Err(ResolveError::NotUsable);
    };
    pick(
        cands
            .iter()
            .filter(|b| b.binding.field == BindingField::Secret(wanted))
            .collect(),
    )
}

impl SecretSource for VaultSource<'_> {
    fn usable_bindings(&self) -> Vec<SourceBinding> {
        self.all()
    }

    fn resolve(&self, reference: &str) -> Result<SourceBinding, ResolveError> {
        let all = self.all();
        let usable_items: HashSet<(VaultId, ItemId)> =
            all.iter().map(|b| (b.vault_id, b.item_id)).collect();
        let target = resolve_agent(self.session, reference, &|v, i| {
            usable_items.contains(&(v, i))
        })
        .map_err(|e| match e {
            RefError::Invalid => ResolveError::Invalid,
            RefError::NotFound | RefError::VaultUnavailable => ResolveError::NotUsable,
            RefError::Ambiguous(candidates) => ResolveError::Ambiguous(candidates),
        })?;
        let field = target.field.as_deref().ok_or(ResolveError::Invalid)?;
        let cands: Vec<&SourceBinding> = all
            .iter()
            .filter(|b| b.vault_id == target.vault && b.item_id == target.item)
            .collect();
        match_field(&cands, field)
    }

    fn read_value(&self, binding: &SourceBinding) -> Result<SecretString, AgentErrorCode> {
        let usable = AgentErrorCode::NotAgentUsable;
        // Everything is looked at again: the item may have changed since the reference resolved.
        if !self.is_eligible(binding.vault_id) {
            return Err(usable);
        }
        let item = self
            .session
            .item(binding.vault_id, binding.item_id)
            .map_err(|_| usable)?;
        if item.is_deleted() || item.trashed_at().is_some() {
            return Err(usable);
        }
        let current = item
            .agent_bindings()
            .into_iter()
            .find(|(id, _)| *id == binding.element_id)
            .and_then(|(_, rec)| Binding::from_record(&rec).ok())
            .ok_or(usable)?;
        if current.digest != binding.binding.digest {
            return Err(usable);
        }
        let value = match &current.field {
            BindingField::Secret(f) => item.secret(*f).ok().flatten(),
            BindingField::Custom(id) => item
                .custom_fields()
                .into_iter()
                .find(|(cid, _)| cid == id)
                .and_then(|(_, c)| match c.value {
                    CustomFieldValue::Hidden(s) => Some(s),
                    _ => None,
                }),
        }
        .ok_or(usable)?;
        check_value(value.expose_secret()).map_err(|_| usable)?;
        Ok(value)
    }

    fn basic_user(&self, binding: &SourceBinding) -> Result<Option<String>, AgentErrorCode> {
        let Inject::Basic(user) = &binding.binding.inject else {
            return Ok(None);
        };
        match user {
            BasicUser::SecretIsUser => Ok(None),
            BasicUser::Literal(text) => Ok(Some(text.clone())),
            BasicUser::Field(field) => {
                let item = self
                    .session
                    .item(binding.vault_id, binding.item_id)
                    .map_err(|_| AgentErrorCode::NotAgentUsable)?;
                item.text(*field)
                    .filter(|t| {
                        !t.is_empty() && !t.contains(':') && !t.chars().any(char::is_control)
                    })
                    .map(|t| Some(t.to_owned()))
                    .ok_or(AgentErrorCode::NotAgentUsable)
            }
        }
    }
}
