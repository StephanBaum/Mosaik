//! Agent access (ADR-0159, ADR-0160): the bindings that mark a secret as
//! usable by AI agents, the paired agent clients, the pairing and access
//! dialogs, grants and the activity log.
//!
//! No value crosses this contract. A binding names a field and says where and
//! how the value may be sent; a grant names an item and a client; the log
//! holds facts about a use, never a value, a header, a body or a query string.
//! The only text from outside that reaches the webview is the client's name
//! and the agent's reason, and the UI shows both as plain, labelled text.
//!
//! This module imports only `serde` and contract types, because the desktop
//! fuzz crate includes `src/contract/` by path.

use serde::{Deserialize, Serialize};

use super::ids::{WireItemId, WireVaultId};
use super::items::SecretRef;
use super::macros::wire_enum;

wire_enum!(
    /// How a value is placed in a request.
    pub enum AgentInjectKind {
        /// `Authorization: Bearer <value>`.
        Bearer,
        /// A named header with an optional prefix.
        Header,
        /// `Authorization: Basic`, with a user name.
        Basic,
        /// A query parameter (the destination may log URLs).
        Query,
    }
);

wire_enum!(
    /// Where the user name of a Basic binding comes from.
    pub enum AgentBasicUserKind {
        /// A fixed user name (`value`).
        Literal,
        /// A text field of the item (`value` is its key).
        Field,
        /// The value is the user name and the password is empty.
        SecretIsUser,
    }
);

/// The user name source of a Basic binding.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentBasicUserView {
    /// The source.
    pub kind: AgentBasicUserKind,
    /// The literal or the text field key; `null` for `secretIsUser`.
    pub value: Option<String>,
}

/// How a binding places the value. Only the members of `kind` are set.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentInjectView {
    /// The kind.
    pub kind: AgentInjectKind,
    /// `header`: the header name.
    pub header_name: Option<String>,
    /// `header`: the text before the value.
    pub header_prefix: Option<String>,
    /// `basic`: the user name.
    pub basic_user: Option<AgentBasicUserView>,
    /// `query`: the parameter name.
    pub query_name: Option<String>,
}

/// What a binding looked like when the user last confirmed it on this device.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBindingSnapshot {
    /// The destinations then.
    pub origins: Vec<String>,
    /// How the value was sent.
    pub inject: AgentInjectKind,
}

/// One binding of an item, as the detail shows it. Never a value.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBindingView {
    /// The binding.
    pub element_id: String,
    /// The bound field (`field` or `customField`).
    pub field: SecretRef,
    /// The destinations, canonical.
    pub origins: Vec<String>,
    /// How the value is sent.
    pub inject: AgentInjectView,
    /// The environment variable name for commands.
    pub env: Option<String>,
    /// The user confirmed exactly this binding on this device.
    pub confirmed: bool,
    /// The last confirmed state (`null` if never confirmed here).
    pub last_confirmed: Option<AgentBindingSnapshot>,
    /// An agent could use it now (confirmed, the value qualifies, the vault
    /// is owned and not shared).
    pub usable: bool,
}

/// A change to the item's agent bindings, applied with the draft.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "op",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AgentBindingChange {
    /// Mark a field as usable by agents.
    Add {
        /// The field (`field` or `customField`).
        field: SecretRef,
        /// The destinations (any form of an `https` URL; stored canonical).
        origins: Vec<String>,
        /// How the value is sent.
        inject: AgentInjectView,
        /// The environment variable name for commands.
        env: Option<String>,
    },
    /// Replace a binding.
    Update {
        /// The binding.
        element_id: String,
        /// The field.
        field: SecretRef,
        /// The destinations.
        origins: Vec<String>,
        /// How the value is sent.
        inject: AgentInjectView,
        /// The environment variable name.
        env: Option<String>,
    },
    /// Stop agents from using the field.
    Remove {
        /// The binding.
        element_id: String,
    },
}

/// `agent_binding_confirm`: the user reviewed a binding that arrived by sync.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentBindingConfirmRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The binding.
    pub element_id: String,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::items::SecretFieldKey;

    pub(crate) fn bearer() -> AgentInjectView {
        AgentInjectView {
            kind: AgentInjectKind::Bearer,
            header_name: None,
            header_prefix: None,
            basic_user: None,
            query_name: None,
        }
    }

    pub(crate) fn header() -> AgentInjectView {
        AgentInjectView {
            kind: AgentInjectKind::Header,
            header_name: Some("X-Api-Key".into()),
            header_prefix: Some(String::new()),
            basic_user: None,
            query_name: None,
        }
    }

    pub(crate) fn basic() -> AgentInjectView {
        AgentInjectView {
            kind: AgentInjectKind::Basic,
            header_name: None,
            header_prefix: None,
            basic_user: Some(AgentBasicUserView {
                kind: AgentBasicUserKind::Literal,
                value: Some("api".into()),
            }),
            query_name: None,
        }
    }

    pub(crate) fn query() -> AgentInjectView {
        AgentInjectView {
            kind: AgentInjectKind::Query,
            header_name: None,
            header_prefix: None,
            basic_user: None,
            query_name: Some("key".into()),
        }
    }

    pub(crate) fn field() -> SecretRef {
        SecretRef::Field {
            field: SecretFieldKey::ApiKeySecret,
        }
    }

    pub(crate) fn views() -> Vec<AgentBindingView> {
        vec![
            AgentBindingView {
                element_id: "b1".into(),
                field: field(),
                origins: vec!["https://api.example.com".into()],
                inject: bearer(),
                env: Some("EXAMPLE_API_KEY".into()),
                confirmed: true,
                last_confirmed: None,
                usable: true,
            },
            AgentBindingView {
                element_id: "b2".into(),
                field: SecretRef::CustomField {
                    element_id: "c1".into(),
                },
                origins: vec![
                    "https://api.example.com".into(),
                    "https://eu.example.com:8443".into(),
                ],
                inject: header(),
                env: None,
                confirmed: false,
                last_confirmed: Some(AgentBindingSnapshot {
                    origins: vec!["https://api.example.com".into()],
                    inject: AgentInjectKind::Bearer,
                }),
                usable: false,
            },
        ]
    }

    pub(crate) fn changes() -> Vec<AgentBindingChange> {
        vec![
            AgentBindingChange::Add {
                field: field(),
                origins: vec!["api.example.com".into()],
                inject: bearer(),
                env: None,
            },
            AgentBindingChange::Update {
                element_id: "b1".into(),
                field: field(),
                origins: vec!["https://api.example.com".into()],
                inject: query(),
                env: Some("EXAMPLE_API_KEY".into()),
            },
            AgentBindingChange::Remove {
                element_id: "b1".into(),
            },
        ]
    }

    pub(crate) fn basic_change() -> AgentBindingChange {
        AgentBindingChange::Add {
            field: field(),
            origins: vec!["https://api.example.com".into()],
            inject: basic(),
            env: None,
        }
    }
}
