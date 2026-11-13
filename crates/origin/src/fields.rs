//! Which fields of a platform view tree to fill (ADR-0092, brief §4.3).
//!
//! Pure: the platform maps its tree (Android `AssistStructure`) to
//! [`FieldNode`]s. Nodes of another web domain (a cross-origin frame) are
//! never selected, and a focus inside one refuses the whole request.

/// The most nodes looked at; later ones are ignored (fewer fills, never more).
pub const MAX_NODES: usize = 4096;

/// The input type of a node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputKind {
    /// Plain text, or unknown.
    Text,
    /// A password variation (`TYPE_TEXT_VARIATION_PASSWORD` and the like).
    Password,
    /// An email address, phone number or number field.
    Other,
}

/// One view node.
#[derive(Clone, Debug)]
pub struct FieldNode {
    /// The `webDomain` of the node (`None` for a native view).
    pub web_domain: Option<String>,
    /// The platform autofill hints (`username`, `password`, `newPassword`,
    /// `smsOTPCode`, ...).
    pub hints: Vec<String>,
    /// The HTML `autocomplete` attribute (`username`, `current-password`,
    /// `new-password`, `one-time-code`, ...).
    pub autocomplete: Option<String>,
    /// The input type.
    pub input_kind: InputKind,
    /// The `idEntry` or a similar resource name.
    pub id_entry: Option<String>,
    /// Whether the node is visible.
    pub visible: bool,
    /// Whether the node has the input focus.
    pub focused: bool,
}

/// What a field takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldRole {
    /// The user name or email.
    Username,
    /// The current password.
    Password,
    /// A new password (sign-up or change).
    NewPassword,
    /// A one-time code.
    Otp,
}

/// Why nothing is selected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldRefusal {
    /// The focused node is in a frame of another web domain.
    ForeignFrame,
}

/// A selected field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SelectedField {
    /// The index of the node in the slice given to [`select_fill_fields`].
    pub index: usize,
    /// What it takes.
    pub role: FieldRole,
}

/// The result of [`select_fill_fields`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FieldSelection {
    /// The web domain of the root (the first node), if any.
    pub root_domain: Option<String>,
    /// The fields to fill, in tree order.
    pub fields: Vec<SelectedField>,
    /// Set when the request is refused (and `fields` is empty).
    pub refused: Option<FieldRefusal>,
}

const USERNAME_WORDS: &[&str] = &[
    "user", "login", "email", "e-mail", "mail", "account", "benutzer", "nutzer", "konto", "anmeld",
    "kennung",
];
const PASSWORD_WORDS: &[&str] = &["pass", "pwd", "kennwort", "passwort", "secret"];
const NEW_WORDS: &[&str] = &[
    "new",
    "neu",
    "confirm",
    "repeat",
    "retype",
    "wiederhol",
    "bestätig",
];
const OTP_WORDS: &[&str] = &[
    "otp",
    "totp",
    "2fa",
    "one-time",
    "onetime",
    "one_time",
    "verification",
    "verifizier",
    "einmal",
    "sicherheitscode",
    "bestätigungscode",
    "authenticator",
    "mfa",
];

fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

fn by_hint(hint: &str) -> Option<FieldRole> {
    match hint.to_ascii_lowercase().as_str() {
        "username" | "emailaddress" | "newusername" | "email" => Some(FieldRole::Username),
        "password" => Some(FieldRole::Password),
        "newpassword" => Some(FieldRole::NewPassword),
        "smsotpcode" | "onetimecode" | "one-time-code" => Some(FieldRole::Otp),
        _ => None,
    }
}

fn by_autocomplete(value: &str) -> Option<FieldRole> {
    value
        .split_ascii_whitespace()
        .find_map(|token| match token.to_ascii_lowercase().as_str() {
            "username" | "email" => Some(FieldRole::Username),
            "current-password" => Some(FieldRole::Password),
            "new-password" => Some(FieldRole::NewPassword),
            "one-time-code" => Some(FieldRole::Otp),
            _ => None,
        })
}

fn by_keywords(node: &FieldNode) -> Option<FieldRole> {
    let mut text = node.id_entry.clone().unwrap_or_default();
    for hint in &node.hints {
        text.push(' ');
        text.push_str(hint);
    }
    let text = text.to_lowercase();
    if contains_any(&text, OTP_WORDS) {
        Some(FieldRole::Otp)
    } else if contains_any(&text, PASSWORD_WORDS) {
        Some(if contains_any(&text, NEW_WORDS) {
            FieldRole::NewPassword
        } else {
            FieldRole::Password
        })
    } else if contains_any(&text, USERNAME_WORDS) {
        Some(FieldRole::Username)
    } else {
        None
    }
}

fn role_of(node: &FieldNode) -> Option<FieldRole> {
    node.hints
        .iter()
        .find_map(|h| by_hint(h))
        .or_else(|| node.autocomplete.as_deref().and_then(by_autocomplete))
        .or_else(|| (node.input_kind == InputKind::Password).then_some(FieldRole::Password))
        .or_else(|| by_keywords(node))
}

/// Chooses the fields to fill.
///
/// 1. The root web domain is the first node's.
/// 2. A node whose `web_domain` differs is a cross-origin frame and is
///    excluded; if the **focused** node is one, the result is refused.
/// 3. Roles come from autofill hints, then HTML `autocomplete`, then the input
///    type (a password variation), and only then from `id_entry` and hint
///    keywords (English and German).
/// 4. Invisible nodes are excluded.
#[must_use]
pub fn select_fill_fields(nodes: &[FieldNode]) -> FieldSelection {
    let nodes = nodes.get(..MAX_NODES).unwrap_or(nodes);
    let root = nodes.first().and_then(|n| n.web_domain.clone());
    let mut selection = FieldSelection {
        root_domain: root.clone(),
        ..FieldSelection::default()
    };
    if nodes.iter().any(|n| n.focused && n.web_domain != root) {
        selection.refused = Some(FieldRefusal::ForeignFrame);
        return selection;
    }
    for (index, node) in nodes.iter().enumerate() {
        if node.web_domain != root || !node.visible {
            continue;
        }
        if let Some(role) = role_of(node) {
            selection.fields.push(SelectedField { index, role });
        }
    }
    selection
}

/// Whose fill a selection serves (ADR-0092). The platform tree is the same
/// for both; what may be filled is not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldScope {
    /// A page in an allowlisted browser. The browser's own screen before the
    /// page (address bar, tab strip) has no `web_domain` and is dropped: the
    /// page root is the first node that has one.
    Web,
    /// An app, which includes a browser that is not allowlisted. Only native
    /// nodes can be filled: a node with a `web_domain`, own or inherited, is
    /// web content of someone else and is excluded, and a focus in one refuses
    /// the request (M10-G2 round 2, N1).
    App,
}

/// [`select_fill_fields`] for a [`FieldScope`]. `index` always points into the
/// `nodes` given.
#[must_use]
pub fn select_scoped_fill_fields(nodes: &[FieldNode], scope: FieldScope) -> FieldSelection {
    let nodes = nodes.get(..MAX_NODES).unwrap_or(nodes);
    match scope {
        FieldScope::Web => {
            let start = nodes
                .iter()
                .position(|n| n.web_domain.is_some())
                .unwrap_or(0);
            let mut selection = select_fill_fields(nodes.get(start..).unwrap_or_default());
            for field in &mut selection.fields {
                field.index = field.index.saturating_add(start);
            }
            selection
        }
        FieldScope::App => {
            let mut selection = FieldSelection::default();
            if nodes.iter().any(|n| n.focused && n.web_domain.is_some()) {
                selection.refused = Some(FieldRefusal::ForeignFrame);
                return selection;
            }
            for (index, node) in nodes.iter().enumerate() {
                if node.web_domain.is_some() || !node.visible {
                    continue;
                }
                if let Some(role) = role_of(node) {
                    selection.fields.push(SelectedField { index, role });
                }
            }
            selection
        }
    }
}
