//! Login-field roles (ADR-0164): from the edit controls of one container, which
//! role the focused one plays. Pure; the Windows side supplies the controls in
//! UI Automation tree order.

use crate::verify::{Rect, RuntimeId, probe_points};

/// How many levels above the focused edit the container is taken.
pub const CONTAINER_LEVELS: u8 = 3;
/// The most edit controls read from a container or window.
pub const MAX_EDITS_SCANNED: usize = 64;

/// What the probe read about one edit control.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EditInfo {
    /// The element's runtime ID.
    pub runtime_id: RuntimeId,
    /// Whether it is a password field.
    pub is_password: bool,
    /// Whether it is enabled.
    pub enabled: bool,
    /// Whether it is off screen.
    pub offscreen: bool,
    /// Its screen rectangle.
    pub rect: Rect,
}

/// The part the focused edit plays in a login.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldRole {
    /// A password field and the index of its user-name field, if any.
    Password {
        /// The nearest usable text field before it.
        username: Option<usize>,
    },
    /// The user-name field of the password field at `password`.
    Username {
        /// The index of the password field.
        password: usize,
    },
    /// A text field with no password field in its container: a candidate for
    /// the first step of a two-step login.
    Text,
}

/// Whether the edit can be filled: enabled, on screen, and large enough to be
/// tested for occlusion.
#[must_use]
pub fn usable(edit: &EditInfo) -> bool {
    edit.enabled && !edit.offscreen && probe_points(edit.rect).is_some()
}

fn username_before(edits: &[EditInfo], password: usize) -> Option<usize> {
    edits
        .get(..password)?
        .iter()
        .rposition(|e| usable(e) && !e.is_password)
}

/// The role of `edits[focused]`; `edits` come from one container in tree order.
///
/// - A password field takes the nearest usable text field before it.
/// - A text field is the user name only when the nearest usable password field
///   after it names it as its user name.
/// - A text field with no usable password field in the container is
///   [`FieldRole::Text`].
/// - Anything else, a disabled or hidden field, a field after the password:
///   `None`.
#[must_use]
pub fn classify(edits: &[EditInfo], focused: usize) -> Option<FieldRole> {
    let edit = edits.get(focused).filter(|e| usable(e))?;
    if edit.is_password {
        return Some(FieldRole::Password {
            username: username_before(edits, focused),
        });
    }
    let after = focused + 1;
    let password = edits
        .get(after..)?
        .iter()
        .position(|e| usable(e) && e.is_password)
        .map(|offset| after + offset);
    match password {
        Some(k) if username_before(edits, k) == Some(focused) => {
            Some(FieldRole::Username { password: k })
        }
        Some(_) => None,
        None if edits.iter().any(|e| usable(e) && e.is_password) => None,
        None => Some(FieldRole::Text),
    }
}

/// Whether the window holds exactly one usable edit and it is `focused`.
#[must_use]
pub fn lone_text(window_edits: &[EditInfo], focused: &RuntimeId) -> bool {
    let mut usable_edits = window_edits.iter().filter(|e| usable(e));
    match (usable_edits.next(), usable_edits.next()) {
        (Some(only), None) => only.runtime_id == *focused,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: i32, pw: bool) -> EditInfo {
        EditInfo {
            runtime_id: RuntimeId(vec![42, id]),
            is_password: pw,
            enabled: true,
            offscreen: false,
            rect: Rect {
                left: 10,
                top: id * 40,
                right: 210,
                bottom: id * 40 + 30,
            },
        }
    }

    #[test]
    fn a_password_field_takes_the_nearest_text_field_before_it() {
        let edits = [e(0, false), e(1, true)];
        assert_eq!(
            classify(&edits, 1),
            Some(FieldRole::Password { username: Some(0) })
        );
        assert_eq!(
            classify(&edits, 0),
            Some(FieldRole::Username { password: 1 })
        );
    }

    #[test]
    fn only_the_nearest_text_field_before_a_password_is_a_username() {
        let edits = [e(0, false), e(1, false), e(2, true)];
        assert_eq!(
            classify(&edits, 1),
            Some(FieldRole::Username { password: 2 })
        );
        assert_eq!(classify(&edits, 0), None);
        assert_eq!(
            classify(&edits, 2),
            Some(FieldRole::Password { username: Some(1) })
        );
    }

    #[test]
    fn disabled_hidden_or_tiny_fields_are_skipped() {
        let mut user = e(0, false);
        user.enabled = false;
        let edits = [user, e(1, true)];
        assert_eq!(
            classify(&edits, 1),
            Some(FieldRole::Password { username: None })
        );
        assert_eq!(classify(&edits, 0), None);
        let mut hidden = e(0, false);
        hidden.offscreen = true;
        assert_eq!(classify(&[hidden, e(1, true)], 0), None);
        let mut tiny = e(0, false);
        tiny.rect.right = tiny.rect.left + 3;
        assert_eq!(
            classify(&[tiny, e(1, true)], 1),
            Some(FieldRole::Password { username: None })
        );
        assert_eq!(classify(&edits, 9), None);
    }

    #[test]
    fn a_text_field_without_any_password_field_is_a_text_candidate() {
        assert_eq!(classify(&[e(0, false)], 0), Some(FieldRole::Text));
        assert_eq!(
            classify(&[e(0, false), e(1, false)], 1),
            Some(FieldRole::Text)
        );
    }

    #[test]
    fn a_text_field_after_the_password_is_not_a_login_field() {
        let edits = [e(0, false), e(1, true), e(2, false)];
        assert_eq!(classify(&edits, 2), None);
    }

    #[test]
    fn lone_text_needs_exactly_one_usable_edit_in_the_window() {
        let focused = RuntimeId(vec![42, 0]);
        assert!(lone_text(&[e(0, false)], &focused));
        assert!(!lone_text(&[e(0, false), e(1, false)], &focused));
        let mut disabled = e(1, false);
        disabled.enabled = false;
        assert!(lone_text(&[e(0, false), disabled], &focused));
        assert!(!lone_text(&[e(1, false)], &focused));
        assert!(!lone_text(&[], &focused));
    }
}
