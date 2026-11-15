//! The picker windows' contract (ADR-0164): what the chip and the list of program fill show and
//! send. Neither window ever receives a secret: logins travel as ID, title, user name, vault
//! name and host; programs as kind, names and (for an unsigned one) its path. The password goes
//! from the vault into the shim inside Rust.

use serde::{Deserialize, Serialize};

use super::ids::{WireItemId, WireVaultId};
use super::macros::wire_enum;
use super::settings::{Language, Theme};

wire_enum!(
    /// What the picker shows now.
    pub enum PickerMode {
        /// Nothing.
        Hidden,
        /// The small button at the field.
        Chip,
        /// The list of logins.
        List,
        /// A short notice.
        Notice,
    }
);

wire_enum!(
    /// How a program is identified.
    pub enum ProgramKind {
        /// A packaged (Store) app.
        Package,
        /// A program with a signature Windows trusts.
        Signed,
        /// A program without one: bound by its path and file hash.
        Unsigned,
    }
);

wire_enum!(
    /// Why the user is asked to link a program again.
    pub enum LinkAgainReason {
        /// The same program is now signed by someone else.
        SignerChanged,
        /// The same path now holds another file.
        FileChanged,
    }
);

wire_enum!(
    /// A notice the picker shows.
    pub enum PickerNotice {
        /// The program runs as administrator.
        Elevated,
        /// No login field at the focus.
        NoField,
        /// The fill stopped because the window changed.
        Aborted,
        /// The fill stopped and what it typed was removed again.
        AbortedCleared,
        /// Shift, Ctrl, Alt or Windows were still held.
        ModifiersHeld,
        /// The program did not answer in time.
        NotResponding,
    }
);

/// A program, for display.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramView {
    /// How it is identified.
    pub kind: ProgramKind,
    /// The exe file name.
    pub exe: String,
    /// The name to show.
    pub name: String,
    /// The signer's name, when it has a trusted one.
    pub signer: Option<String>,
    /// The file's path, for an unsigned program.
    pub path: Option<String>,
}

/// A login, for the list. No password, no one-time code.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerLogin {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The title.
    pub title: String,
    /// The user name.
    pub username: Option<String>,
    /// The vault's name.
    pub vault_name: Option<String>,
    /// The host of the login's first web address.
    pub host: Option<String>,
}

/// The first and the second look at a changed file: 16 hex digits of each hash.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashChange {
    /// The linked file's hash prefix.
    pub old_prefix: String,
    /// This file's hash prefix.
    pub new_prefix: String,
}

/// The step "link this login to this program".
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStep {
    /// The login.
    pub login: PickerLogin,
    /// The program.
    pub program: ProgramView,
    /// How many confirmations it takes (1 or 2).
    pub confirmations: u8,
    /// How many were given.
    pub confirmed: u8,
    /// An exe name this one resembles among the linked programs.
    pub lookalike_of: Option<String>,
    /// For a program that changed on disk.
    pub hash_change: Option<HashChange>,
    /// Whether Enter is pressed after the fill.
    pub submit: bool,
}

/// What a picker window shows.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerState {
    /// Rises with every change of what a picker shows; a request with another one is `pickerStale`.
    pub generation: u64,
    /// What to show.
    pub mode: PickerMode,
    /// The vault is locked: the chip says "unlock".
    pub locked: bool,
    /// The UI language.
    pub language: Language,
    /// The color theme.
    pub theme: Theme,
    /// The program at the field.
    pub program: Option<ProgramView>,
    /// The logins linked to it.
    pub matches: Vec<PickerLogin>,
    /// The index in `matches` to preselect (exactly one match).
    pub preselected: Option<u32>,
    /// Logins that might belong to it (only when nothing is linked).
    pub suggestions: Vec<PickerLogin>,
    /// The program was linked before its signer or file changed.
    pub link_again: Option<LinkAgainReason>,
    /// The link step, if one is open.
    pub link: Option<LinkStep>,
    /// The notice, in `Notice` mode.
    pub notice: Option<PickerNotice>,
}

/// A request that carries the generation it was made for.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickerGenerationRequest {
    /// The generation of the state the user saw.
    pub generation: u64,
}

/// `picker_search`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickerSearchRequest {
    /// The generation of the state the user saw.
    pub generation: u64,
    /// The text (at most 128 characters).
    pub query: String,
}

/// `picker_choose` and `picker_link_begin`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickerItemRequest {
    /// The generation of the state the user saw.
    pub generation: u64,
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
}

/// The answer to `picker_link_confirm`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LinkConfirmResult {
    /// The first of two confirmations is recorded.
    NeedAnother,
    /// Too soon after the step (or the first confirmation) appeared.
    TooSoon,
    /// Linked, and the fill ran.
    Linked,
}

/// What a picker window is told.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PickerEvent {
    /// The state changed: read it again.
    Changed {
        /// The new generation.
        generation: u64,
    },
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn login(n: u8) -> PickerLogin {
        PickerLogin {
            vault_id: ids::vault(1),
            item_id: ids::item(n),
            title: "Slack".into(),
            username: Some("alex@example.com".into()),
            vault_name: Some("Personal".into()),
            host: Some("slack.com".into()),
        }
    }

    pub(crate) fn program() -> ProgramView {
        ProgramView {
            kind: ProgramKind::Signed,
            exe: "slack.exe".into(),
            name: "Slack".into(),
            signer: Some("Slack Technologies".into()),
            path: None,
        }
    }

    pub(crate) fn states() -> Vec<PickerState> {
        let base = PickerState {
            generation: 7,
            mode: PickerMode::List,
            locked: false,
            language: Language::En,
            theme: Theme::Dark,
            program: Some(program()),
            matches: vec![login(1)],
            preselected: Some(0),
            suggestions: vec![],
            link_again: None,
            link: None,
            notice: None,
        };
        vec![
            base.clone(),
            PickerState {
                mode: PickerMode::Chip,
                locked: true,
                matches: vec![],
                preselected: None,
                ..base.clone()
            },
            PickerState {
                matches: vec![],
                preselected: None,
                suggestions: vec![login(2)],
                link_again: Some(LinkAgainReason::FileChanged),
                link: Some(LinkStep {
                    login: login(2),
                    program: ProgramView {
                        kind: ProgramKind::Unsigned,
                        exe: "tool.exe".into(),
                        name: "tool.exe".into(),
                        signer: None,
                        path: Some("C:/Tools/tool.exe".into()),
                    },
                    confirmations: 2,
                    confirmed: 1,
                    lookalike_of: Some("tools.exe".into()),
                    hash_change: Some(HashChange {
                        old_prefix: "0011223344556677".into(),
                        new_prefix: "8899aabbccddeeff".into(),
                    }),
                    submit: false,
                }),
                ..base.clone()
            },
            PickerState {
                mode: PickerMode::Notice,
                program: None,
                matches: vec![],
                preselected: None,
                notice: Some(PickerNotice::ModifiersHeld),
                ..base
            },
        ]
    }

    pub(crate) fn generation_requests() -> Vec<PickerGenerationRequest> {
        vec![PickerGenerationRequest { generation: 7 }]
    }

    pub(crate) fn search_requests() -> Vec<PickerSearchRequest> {
        vec![PickerSearchRequest {
            generation: 7,
            query: "sla".into(),
        }]
    }

    pub(crate) fn item_requests() -> Vec<PickerItemRequest> {
        vec![PickerItemRequest {
            generation: 7,
            vault_id: ids::vault(1),
            item_id: ids::item(1),
        }]
    }

    pub(crate) fn confirm_results() -> Vec<LinkConfirmResult> {
        vec![
            LinkConfirmResult::NeedAnother,
            LinkConfirmResult::TooSoon,
            LinkConfirmResult::Linked,
        ]
    }

    pub(crate) fn events() -> Vec<PickerEvent> {
        vec![PickerEvent::Changed { generation: 8 }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire shape of every picker type: camelCase, tags, nulls and no unknown keys.
    #[test]
    fn picker_types_have_the_documented_wire_shape() {
        let states = samples::states();
        assert_eq!(states.len(), 4);
        let list = serde_json::to_value(&states[0]).unwrap();
        assert_eq!(list["mode"], "list");
        assert_eq!(list["language"], "en");
        assert_eq!(list["preselected"], 0);
        assert_eq!(list["matches"][0]["vaultName"], "Personal");
        assert_eq!(list["program"]["kind"], "signed");
        assert!(list["link"].is_null() && list["notice"].is_null());
        let link = serde_json::to_value(&states[2]).unwrap();
        assert_eq!(link["linkAgain"], "fileChanged");
        assert_eq!(link["link"]["hashChange"]["oldPrefix"], "0011223344556677");
        assert_eq!(link["link"]["program"]["kind"], "unsigned");
        let results: Vec<String> = samples::confirm_results()
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        assert_eq!(
            results,
            [
                r#"{"kind":"needAnother"}"#,
                r#"{"kind":"tooSoon"}"#,
                r#"{"kind":"linked"}"#
            ]
        );
        assert_eq!(
            serde_json::to_string(&samples::events()[0]).unwrap(),
            r#"{"type":"changed","generation":8}"#
        );
        // Requests refuse unknown keys.
        assert!(
            serde_json::from_str::<PickerGenerationRequest>(r#"{"generation":1,"extra":true}"#)
                .is_err()
        );
        let request = serde_json::to_string(&samples::item_requests()[0]).unwrap();
        assert!(serde_json::from_str::<PickerItemRequest>(&request).is_ok());
        let request = serde_json::to_string(&samples::search_requests()[0]).unwrap();
        assert!(serde_json::from_str::<PickerSearchRequest>(&request).is_ok());
        let request = serde_json::to_string(&samples::generation_requests()[0]).unwrap();
        assert!(serde_json::from_str::<PickerGenerationRequest>(&request).is_ok());
    }
}
