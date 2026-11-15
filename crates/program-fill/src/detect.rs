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

/// Whether the container's edits already show that the window holds more than
/// one usable edit, so a lone text field is out and the window need not be
/// searched at all (a tree-wide search is the costly call).
#[must_use]
pub fn container_rules_out_lone_text(container_edits: &[EditInfo]) -> bool {
    container_edits.iter().filter(|e| usable(e)).take(2).count() > 1
}

/// Whether the focused element belongs to web content: a browser engine
/// inside the program (Electron, CEF, `WebView2`: framework `Chrome`, `WebView`,
/// `Edge`, `Gecko`), or an element owned by another process than the window's.
/// Such a program draws its own fields and may show another page under the
/// same root window, so a second step is never written without a new pick.
#[must_use]
pub fn is_web_content(
    framework_id: &str,
    element_pid: u32,
    window_pid: u32,
    webview_host: bool,
) -> bool {
    let framework = framework_id.to_ascii_lowercase();
    webview_host
        || element_pid != window_pid
        || [
            "chrome", "webview", "edge", "gecko", "mozilla", "cef", "electron",
        ]
        .iter()
        .any(|name| framework.contains(name))
}

/// Whether the second step of a two-step login is written without a new pick:
/// the program is not web content and the new password field is the only usable
/// password edit of its root window. `window_edits` is `None` when the window
/// could not be searched (then never).
#[must_use]
pub fn second_step_is_automatic(
    web_content: bool,
    window_edits: Option<&[EditInfo]>,
    focused: &RuntimeId,
) -> bool {
    if web_content {
        return false;
    }
    let Some(edits) = window_edits else {
        return false;
    };
    let mut passwords = edits.iter().filter(|e| usable(e) && e.is_password);
    match (passwords.next(), passwords.next()) {
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

    #[test]
    fn the_container_alone_can_rule_out_a_lone_text_field() {
        assert!(!container_rules_out_lone_text(&[e(0, false)]));
        let mut disabled = e(1, false);
        disabled.enabled = false;
        assert!(!container_rules_out_lone_text(&[e(0, false), disabled]));
        assert!(container_rules_out_lone_text(&[e(0, false), e(1, false)]));
        assert!(!container_rules_out_lone_text(&[]));
    }

    #[test]
    fn web_content_is_told_by_framework_process_or_webview_host() {
        assert!(!is_web_content("WPF", 5, 5, false));
        assert!(!is_web_content("Win32", 5, 5, false));
        assert!(is_web_content("Chrome", 5, 5, false));
        assert!(is_web_content("WebView2", 5, 5, false));
        assert!(is_web_content("Win32", 6, 5, false));
        assert!(is_web_content("Win32", 5, 5, true));
    }

    #[test]
    fn a_second_step_is_automatic_for_a_native_window_with_one_password_field() {
        let focused = RuntimeId(vec![42, 1]);
        let edits = [e(0, false), e(1, true)];
        assert!(second_step_is_automatic(false, Some(&edits), &focused));
        // Web content never continues on its own.
        assert!(!second_step_is_automatic(true, Some(&edits), &focused));
        // Two password fields (a "confirm" field, or a hidden one): not automatic.
        let two = [e(0, false), e(1, true), e(2, true)];
        assert!(!second_step_is_automatic(false, Some(&two), &focused));
        // The only password field is not the focused one.
        assert!(!second_step_is_automatic(
            false,
            Some(&[e(2, true)]),
            &focused
        ));
        // An unusable second password field does not count.
        let mut hidden = e(2, true);
        hidden.offscreen = true;
        assert!(second_step_is_automatic(
            false,
            Some(&[e(1, true), hidden]),
            &focused
        ));
        // The window could not be searched.
        assert!(!second_step_is_automatic(false, None, &focused));
    }
}
