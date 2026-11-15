//! Where the engine gets bindings and values from (agent access design §5.4, §6).
//!
//! The engine never touches the vault. A [`SecretSource`] shows it the usable bindings (metadata
//! only), resolves a reference, and hands over a value only when the engine asks, after every
//! check passed. The vault-backed implementation is [`crate::vault_source`]; tests use a fake.

use mosaic_core::model::ElementId;
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_vault::refs::RefCandidate;

use crate::binding::Binding;
use crate::error::AgentErrorCode;

/// One agent-usable binding with the metadata the user and the model may see.
#[derive(Clone, Debug)]
pub struct SourceBinding {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// The binding's element ID (the `agent:<id>` of the item).
    pub element_id: ElementId,
    /// The vault's name (its ID when it has none).
    pub vault_name: String,
    /// The item's title.
    pub item_title: String,
    /// A neutral key for core fields (`api_key.secret`) or the custom field's label.
    pub field_label: String,
    /// The validated binding.
    pub binding: Binding,
    /// Whether the user confirmed this exact binding on this device.
    pub confirmed: bool,
}

/// Why a reference did not resolve to a usable binding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ResolveError {
    /// Not a well-formed agent reference.
    Invalid,
    /// Nothing an agent may use (never says whether the field exists).
    NotUsable,
    /// More than one usable item matches.
    Ambiguous(Vec<RefCandidate>),
}

/// The vault side of the engine.
pub trait SecretSource {
    /// Every usable binding, in a stable order.
    fn usable_bindings(&self) -> Vec<SourceBinding>;

    /// Resolves an agent reference (`mosaic://<vault>/<item>/<field>`).
    ///
    /// # Errors
    /// [`ResolveError`].
    fn resolve(&self, reference: &str) -> Result<SourceBinding, ResolveError>;

    /// Reads the bound value, again from the item, and checks it (at least 12 bytes, no CR, LF or
    /// NUL).
    ///
    /// # Errors
    /// [`AgentErrorCode::NotAgentUsable`] when the value is gone or no longer qualifies.
    fn read_value(&self, binding: &SourceBinding) -> Result<SecretString, AgentErrorCode>;

    /// The user name of a Basic binding: a literal, or the item's text field; `None` when the
    /// secret is the user name.
    ///
    /// # Errors
    /// [`AgentErrorCode::NotAgentUsable`] when the named text field is missing.
    fn basic_user(&self, binding: &SourceBinding) -> Result<Option<String>, AgentErrorCode>;
}
