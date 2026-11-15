//! Binding data on a program login (ADR-0164): the display data and the
//! submit switch kept in the unknown members of the URL element, how a captured
//! program is matched against the saved bindings of a login, and the key of the
//! block list.
//!
//! The display data is for the picker and the item view only. No matcher reads
//! it; matching is `mosaic_origin::program` on the URL text alone.

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::{LoginUrl, Object, Value};
use mosaic_origin::program::{
    BindingChange, ProgramClass, binding_change, fold_name, program_fill_allowed,
};

use crate::identity::CapturedIdentity;
use crate::policy::link_again_needed;

/// The longest display name or signer, in characters.
pub const MAX_DISPLAY_CHARS: usize = 256;

const PROGRAM_KEY: &str = "program";
const SUBMIT_KEY: &str = "submit";

/// What the picker shows for a bound program.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProgramDisplay {
    /// The program name.
    pub name: String,
    /// The signer, when the program has a trusted one.
    pub signer: Option<String>,
}

fn truncated(s: &str) -> String {
    s.chars().take(MAX_DISPLAY_CHARS).collect()
}

/// The display data of a captured program: the package name (the part of the
/// family before `_`) or the exe name; the signer's organization, else its
/// common name.
#[must_use]
pub fn display_for(identity: &CapturedIdentity) -> ProgramDisplay {
    let name = identity.package_family.as_deref().map_or_else(
        || truncated(&identity.exe_name),
        |family| truncated(family.split('_').next().unwrap_or(family)),
    );
    let signer = identity.signer_display.as_ref().and_then(|s| {
        s.organization
            .as_deref()
            .filter(|o| !o.is_empty())
            .or(s.common_name.as_deref())
            .filter(|n| !n.is_empty())
            .map(truncated)
    });
    ProgramDisplay { name, signer }
}

/// The display data stored on a binding; `None` when absent or malformed.
#[must_use]
pub fn program_display(url: &LoginUrl) -> Option<ProgramDisplay> {
    let object = url.extra().get(PROGRAM_KEY)?.as_object()?;
    let name = object.get("name")?.as_str()?;
    let len = name.chars().count();
    if len == 0 || len > MAX_DISPLAY_CHARS {
        return None;
    }
    let signer = match object.get("signer") {
        None => None,
        Some(v) => {
            let s = v.as_str()?;
            if s.chars().count() > MAX_DISPLAY_CHARS {
                return None;
            }
            Some(s.to_owned())
        }
    };
    Some(ProgramDisplay {
        name: name.to_owned(),
        signer,
    })
}

/// Stores the display data on a binding (both parts cut to
/// [`MAX_DISPLAY_CHARS`]).
pub fn set_program_display(url: &mut LoginUrl, display: &ProgramDisplay) {
    let mut object = Object::new();
    object.insert("name", Value::text(&truncated(&display.name)));
    if let Some(signer) = &display.signer {
        object.insert("signer", Value::text(&truncated(signer)));
    }
    url.extra_mut().insert(PROGRAM_KEY, Value::Object(object));
}

/// Whether Enter is pressed after filling this binding: on unless the binding
/// says `submit = false`.
#[must_use]
pub fn submit_enabled(url: &LoginUrl) -> bool {
    !matches!(url.extra().get(SUBMIT_KEY), Some(Value::Bool(false)))
}

/// Turns Enter after the fill on (removes the member) or off (`false`).
pub fn set_submit(url: &mut LoginUrl, on: bool) {
    if on {
        url.extra_mut().remove(SUBMIT_KEY);
    } else {
        url.extra_mut().insert(SUBMIT_KEY, Value::Bool(false));
    }
}

/// How a captured program relates to the saved bindings of a login.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BindingMatch {
    /// The indices of the bindings that allow this program.
    Matched(Vec<usize>),
    /// None matches, but one was bound to this program before its signer or
    /// file changed.
    LinkAgain(BindingChange),
    /// Nothing relates.
    NoBinding,
}

/// Matches a captured program against the saved URLs of one login.
#[must_use]
pub fn match_bindings(saved: &[LoginUrl], identity: &CapturedIdentity) -> BindingMatch {
    let observed = identity.as_identity();
    let matched: Vec<usize> = saved
        .iter()
        .enumerate()
        .filter(|(_, url)| program_fill_allowed(url, &observed).is_ok())
        .map(|(i, _)| i)
        .collect();
    if !matched.is_empty() {
        return BindingMatch::Matched(matched);
    }
    link_again_needed(saved, identity, None).map_or(BindingMatch::NoBinding, |a| {
        BindingMatch::LinkAgain(a.change)
    })
}

/// The indices of the bindings a new link of this program replaces: those
/// whose signer or file changed (`binding_change`), so no dead binding stays
/// next to the new one.
#[must_use]
pub fn superseded(saved: &[LoginUrl], identity: &CapturedIdentity) -> Vec<usize> {
    let observed = identity.as_identity();
    saved
        .iter()
        .enumerate()
        .filter(|(_, url)| binding_change(url, &observed).is_some())
        .map(|(i, _)| i)
        .collect()
}

/// The key of "Don't offer for this program": `package:<family>`,
/// `signed:<exe>:<signer base64url>` or `path:<folded path>` (the file hash is
/// left out, so an update of an unsigned program stays blocked). `None` for an
/// excluded class.
#[must_use]
pub fn block_key(identity: &CapturedIdentity) -> Option<String> {
    if identity.class != ProgramClass::Ordinary {
        return None;
    }
    if let Some(family) = &identity.package_family {
        return Some(format!("package:{family}"));
    }
    if let Some(signer) = &identity.signer_id {
        return Some(format!(
            "signed:{}:{}",
            fold_name(&identity.exe_name),
            Base64UrlUnpadded::encode_string(signer)
        ));
    }
    Some(format!("path:{}", fold_name(&identity.final_path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SignerDisplay;
    use mosaic_origin::program::ProgramUrl;

    const SIGNER: [u8; 32] = [0x11; 32];
    const OLD: [u8; 32] = [0x22; 32];

    fn ident(exe: &str) -> CapturedIdentity {
        CapturedIdentity {
            pid: 1,
            final_path: format!(r"C:\Tools\{exe}"),
            exe_name: exe.to_owned(),
            package_family: None,
            signer_id: None,
            signer_display: None,
            file_sha256: None,
            class: ProgramClass::Ordinary,
        }
    }

    fn signed_ident(exe: &str, signer: [u8; 32]) -> CapturedIdentity {
        CapturedIdentity {
            signer_id: Some(signer),
            ..ident(exe)
        }
    }

    fn signed_url(exe: &str, signer: [u8; 32]) -> LoginUrl {
        LoginUrl::new(
            &ProgramUrl::Signed {
                exe: exe.to_owned(),
                signer,
            }
            .canonical(),
        )
    }

    #[test]
    fn display_comes_from_package_name_or_exe_and_prefers_the_organization() {
        let mut p = ident("calc.exe");
        p.package_family = Some("microsoft.windowscalculator_8wekyb3d8bbwe".to_owned());
        assert_eq!(display_for(&p).name, "microsoft.windowscalculator");
        let mut s = ident("slack.exe");
        s.signer_display = Some(SignerDisplay {
            common_name: Some("Slack CN".to_owned()),
            organization: Some("Slack Technologies".to_owned()),
        });
        let d = display_for(&s);
        assert_eq!(d.name, "slack.exe");
        assert_eq!(d.signer.as_deref(), Some("Slack Technologies"));
        s.signer_display = Some(SignerDisplay {
            common_name: Some("Slack CN".to_owned()),
            organization: None,
        });
        assert_eq!(display_for(&s).signer.as_deref(), Some("Slack CN"));
        s.signer_display = Some(SignerDisplay {
            common_name: None,
            organization: Some("o".repeat(300)),
        });
        assert_eq!(display_for(&s).signer.unwrap().chars().count(), 256);
    }

    #[test]
    fn program_display_round_trips_and_rejects_malformed_members() {
        let mut url = signed_url("slack.exe", SIGNER);
        assert_eq!(program_display(&url), None);
        let d = ProgramDisplay {
            name: "Slack".to_owned(),
            signer: Some("Slack Technologies".to_owned()),
        };
        set_program_display(&mut url, &d);
        assert_eq!(program_display(&url), Some(d));
        for name in [String::new(), "n".repeat(257)] {
            let mut o = Object::new();
            o.insert("name", Value::text(&name));
            url.extra_mut().insert("program", Value::Object(o));
            assert_eq!(program_display(&url), None);
        }
        let mut o = Object::new();
        o.insert("name", Value::int(5));
        url.extra_mut().insert("program", Value::Object(o));
        assert_eq!(program_display(&url), None);
        let mut o = Object::new();
        o.insert("name", Value::text("x"));
        o.insert("signer", Value::int(1));
        url.extra_mut().insert("program", Value::Object(o));
        assert_eq!(program_display(&url), None);
    }

    #[test]
    fn submit_is_on_unless_explicitly_false() {
        let mut url = signed_url("slack.exe", SIGNER);
        assert!(submit_enabled(&url));
        set_submit(&mut url, false);
        assert!(!submit_enabled(&url));
        set_submit(&mut url, true);
        assert!(submit_enabled(&url));
        assert!(url.extra().get("submit").is_none());
        url.extra_mut().insert("submit", Value::text("no"));
        assert!(submit_enabled(&url));
    }

    #[test]
    fn match_bindings_reports_matches_then_link_again() {
        let saved = vec![
            LoginUrl::new("https://example.com"),
            signed_url("slack.exe", SIGNER),
            signed_url("slack.exe", SIGNER),
        ];
        assert_eq!(
            match_bindings(&saved, &signed_ident("slack.exe", SIGNER)),
            BindingMatch::Matched(vec![1, 2])
        );
        assert_eq!(
            match_bindings(&saved, &signed_ident("slack.exe", OLD)),
            BindingMatch::LinkAgain(BindingChange::SignerChanged)
        );
        assert_eq!(
            match_bindings(&saved, &signed_ident("other.exe", SIGNER)),
            BindingMatch::NoBinding
        );
    }

    #[test]
    fn superseded_lists_only_changed_bindings_of_this_program() {
        let saved = vec![
            signed_url("slack.exe", OLD),
            signed_url("other.exe", OLD),
            signed_url("slack.exe", SIGNER),
        ];
        assert_eq!(
            superseded(&saved, &signed_ident("slack.exe", SIGNER)),
            vec![0]
        );
    }

    #[test]
    fn block_keys_ignore_the_unsigned_file_hash() {
        let mut first = ident("t.exe");
        first.final_path = r"C:\Tools\T.exe".to_owned();
        first.file_sha256 = Some([1; 32]);
        let mut second = ident("t.exe");
        second.final_path = r"c:\tools\t.exe".to_owned();
        second.file_sha256 = Some([2; 32]);
        assert_eq!(block_key(&first), block_key(&second));
        assert_eq!(block_key(&first).as_deref(), Some(r"path:c:\tools\t.exe"));
        let mut chrome = ident("chrome.exe");
        chrome.class = ProgramClass::Browser;
        assert_eq!(block_key(&chrome), None);
        let mut packaged = ident("a.exe");
        packaged.package_family = Some("pub.app_8wekyb3d8bbwe".to_owned());
        assert_eq!(
            block_key(&packaged).as_deref(),
            Some("package:pub.app_8wekyb3d8bbwe")
        );
        let signed = block_key(&signed_ident("slack.exe", SIGNER)).unwrap();
        assert!(signed.starts_with("signed:slack.exe:"));
    }
}
