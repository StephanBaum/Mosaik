//! The link policy (ADR-0158): what a login is bound to when the user links a
//! program, which warnings the user sees, and the two separate confirmations an
//! unsigned program needs.

use mosaic_core::model::LoginUrl;
use mosaic_origin::program::{
    BindingChange, ProgramClass, ProgramUrl, binding_change, program_fill_allowed,
    program_lookalike_of,
};

use crate::identity::CapturedIdentity;

/// The least time between the two confirmations of an unsigned program, so a
/// double click is not a link.
pub const SECOND_CONFIRM_MIN_GAP_MS: u64 = 1_000;

/// How many separate confirmations linking needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirmations {
    /// One.
    One,
    /// Two, at least [`SECOND_CONFIRM_MIN_GAP_MS`] apart.
    Two,
}

/// Something the user is told before linking.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LinkWarning {
    /// The program has no trusted signature: it is bound by path and file hash.
    Unsigned,
    /// The exe name is one edit away from an already linked program.
    LookalikeExe {
        /// The linked exe name it resembles.
        similar_to: String,
    },
}

/// What linking would store, and what it takes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProposedBinding {
    /// The URL to save on the login.
    pub url: ProgramUrl,
    /// The confirmations needed.
    pub confirmations: Confirmations,
    /// The warnings to show, `Unsigned` first.
    pub warnings: Vec<LinkWarning>,
}

/// Why a program cannot be linked.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BindRefusal {
    /// A browser, script host, Mosaic, broker or remote-desktop client.
    #[error("excluded program")]
    Excluded(ProgramClass),
    /// The identity has nothing to bind to (no package, signer or file hash).
    #[error("bad identity")]
    BadIdentity,
}

/// What a login would be bound to for this program: its package when it has
/// one (one confirmation), else its trusted signer plus exe name (one), else
/// its path plus file hash (two confirmations, an `Unsigned` warning).
/// `bound_exes` are the exe names of the user's other `signed/` and `path/`
/// bindings, never this identity's own; a one-edit look-alike among them is a
/// warning.
///
/// # Errors
/// [`BindRefusal`].
pub fn propose_binding(
    identity: &CapturedIdentity,
    bound_exes: &[&str],
) -> Result<ProposedBinding, BindRefusal> {
    if identity.class != ProgramClass::Ordinary {
        return Err(BindRefusal::Excluded(identity.class));
    }
    let mut warnings = Vec::new();
    let (url, confirmations) = if let Some(family) = &identity.package_family {
        (
            ProgramUrl::package(family).map_err(|_| BindRefusal::BadIdentity)?,
            Confirmations::One,
        )
    } else if let Some(signer) = identity.signer_subject_sha256 {
        (
            ProgramUrl::signed(&identity.exe_name, signer).map_err(|_| BindRefusal::BadIdentity)?,
            Confirmations::One,
        )
    } else if let Some(hash) = identity.file_sha256 {
        warnings.push(LinkWarning::Unsigned);
        (
            ProgramUrl::path(&identity.final_path, hash).map_err(|_| BindRefusal::BadIdentity)?,
            Confirmations::Two,
        )
    } else {
        return Err(BindRefusal::BadIdentity);
    };
    if let Some(similar) = program_lookalike_of(&identity.exe_name, bound_exes.iter().copied()) {
        warnings.push(LinkWarning::LookalikeExe {
            similar_to: similar.to_owned(),
        });
    }
    Ok(ProposedBinding {
        url,
        confirmations,
        warnings,
    })
}

/// Whether none of the saved URLs matches the program but one of them was bound
/// to it before its signer or file changed: the user is asked to link again.
#[must_use]
pub fn link_again_needed(saved: &[LoginUrl], identity: &CapturedIdentity) -> Option<BindingChange> {
    let observed = identity.as_identity();
    if saved
        .iter()
        .any(|url| program_fill_allowed(url, &observed).is_ok())
    {
        return None;
    }
    saved.iter().find_map(|url| binding_change(url, &observed))
}

/// Where a confirmation stands.
pub enum ConfirmStep {
    /// The first of two confirmations is recorded; confirm again.
    NeedAnother,
    /// The second came too soon (or the clock went back).
    TooSoon,
    /// Confirmed: the URL to save.
    Done(LoginUrl),
}

// `LoginUrl` has no `Debug` (a login's URLs are shown deliberately); the URL is
// public data here, but the step prints only its kind.
impl core::fmt::Debug for ConfirmStep {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NeedAnother => "NeedAnother",
            Self::TooSoon => "TooSoon",
            Self::Done(_) => "Done",
        })
    }
}

/// The confirmations of one link attempt.
#[derive(Clone, Debug)]
pub struct LinkConfirmation {
    proposed: ProposedBinding,
    first_at_ms: Option<u64>,
}

impl LinkConfirmation {
    /// A link attempt for the proposed binding.
    #[must_use]
    pub const fn new(proposed: ProposedBinding) -> Self {
        Self {
            proposed,
            first_at_ms: None,
        }
    }

    /// The user confirmed at `now_ms` (a monotonic clock). With one
    /// confirmation needed the first call is `Done`. With two, the first call
    /// is `NeedAnother` and records its time; a later call before
    /// [`SECOND_CONFIRM_MIN_GAP_MS`] has passed (or with an earlier clock value)
    /// is `TooSoon` and keeps the first time.
    pub fn confirm(&mut self, now_ms: u64) -> ConfirmStep {
        let done = || ConfirmStep::Done(LoginUrl::new(&self.proposed.url.canonical()));
        if self.proposed.confirmations == Confirmations::One {
            return done();
        }
        match self.first_at_ms {
            None => {
                self.first_at_ms = Some(now_ms);
                ConfirmStep::NeedAnother
            }
            Some(first) if now_ms >= first && now_ms - first >= SECOND_CONFIRM_MIN_GAP_MS => done(),
            Some(_) => ConfirmStep::TooSoon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_origin::Refusal;

    const SIGNER: [u8; 32] = [0x11; 32];
    const OTHER: [u8; 32] = [0x22; 32];
    const HASH: [u8; 32] = [0x33; 32];
    const PATH: &str = r"C:\Users\a\AppData\Local\Tool\tool.exe";

    fn identity(exe: &str) -> CapturedIdentity {
        CapturedIdentity {
            pid: 11,
            final_path: format!(r"C:\Apps\{exe}"),
            exe_name: exe.to_owned(),
            package_family: None,
            signer_subject_sha256: None,
            signer_display: None,
            file_sha256: None,
            class: ProgramClass::Ordinary,
        }
    }

    fn signed(exe: &str) -> CapturedIdentity {
        CapturedIdentity {
            signer_subject_sha256: Some(SIGNER),
            ..identity(exe)
        }
    }

    fn unsigned(exe: &str, path: &str) -> CapturedIdentity {
        CapturedIdentity {
            final_path: path.to_owned(),
            file_sha256: Some(HASH),
            ..identity(exe)
        }
    }

    #[test]
    fn packaged_and_signed_programs_need_one_confirmation() {
        let mut packaged = signed("notepad.exe");
        packaged.package_family = Some("microsoft.windowsnotepad_8wekyb3d8bbwe".to_owned());
        let p = propose_binding(&packaged, &[]).unwrap();
        assert_eq!(
            p.url,
            ProgramUrl::package("microsoft.windowsnotepad_8wekyb3d8bbwe").unwrap()
        );
        assert_eq!(p.confirmations, Confirmations::One);
        assert_eq!(p.warnings, Vec::new());

        let p = propose_binding(&signed("discord.exe"), &[]).unwrap();
        assert_eq!(p.url, ProgramUrl::signed("discord.exe", SIGNER).unwrap());
        assert_eq!(p.confirmations, Confirmations::One);
        assert!(!p.warnings.contains(&LinkWarning::Unsigned));
    }

    #[test]
    fn unsigned_program_binds_path_and_hash_with_two_confirmations() {
        let p = propose_binding(&unsigned("tool.exe", PATH), &[]).unwrap();
        assert_eq!(p.url, ProgramUrl::path(PATH, HASH).unwrap());
        assert_eq!(p.confirmations, Confirmations::Two);
        assert_eq!(p.warnings.first(), Some(&LinkWarning::Unsigned));
        // Nothing to bind to: no package, no signer and no hash.
        assert_eq!(
            propose_binding(&identity("tool.exe"), &[]),
            Err(BindRefusal::BadIdentity)
        );
    }

    #[test]
    fn double_click_does_not_link_an_unsigned_program() {
        let p = propose_binding(&unsigned("tool.exe", PATH), &[]).unwrap();
        let canonical = p.url.canonical();
        let done = |step: ConfirmStep| match step {
            ConfirmStep::Done(url) => url.url.as_str() == canonical,
            _ => false,
        };
        let mut c = LinkConfirmation::new(p.clone());
        assert!(matches!(c.confirm(1_000), ConfirmStep::NeedAnother));
        assert!(matches!(c.confirm(1_200), ConfirmStep::TooSoon));
        assert!(matches!(c.confirm(1_999), ConfirmStep::TooSoon));
        assert!(done(c.confirm(2_000)));
        // The clock going back is never a second confirmation.
        let mut c = LinkConfirmation::new(p);
        assert!(matches!(c.confirm(5_000), ConfirmStep::NeedAnother));
        assert!(matches!(c.confirm(500), ConfirmStep::TooSoon));
        assert!(done(c.confirm(6_000)));
        // One confirmation is enough for a signed program.
        let p = propose_binding(&signed("discord.exe"), &[]).unwrap();
        assert!(matches!(
            LinkConfirmation::new(p).confirm(0),
            ConfirmStep::Done(_)
        ));
    }

    #[test]
    fn lookalike_exe_is_a_warning_never_a_match() {
        let lookalike = signed("dlscord.exe");
        let p = propose_binding(&lookalike, &["discord.exe", "slack.exe"]).unwrap();
        assert!(p.warnings.contains(&LinkWarning::LookalikeExe {
            similar_to: "discord.exe".to_owned()
        }));
        let discord = LoginUrl::new(
            &ProgramUrl::signed("discord.exe", SIGNER)
                .unwrap()
                .canonical(),
        );
        assert_eq!(
            program_fill_allowed(&discord, &lookalike.as_identity()),
            Err(Refusal::NoMatch)
        );
        // A Cyrillic o is the same warning.
        let homoglyph = signed("disc\u{043e}rd.exe");
        let p = propose_binding(&homoglyph, &["discord.exe"]).unwrap();
        assert_ne!(p.warnings, Vec::new());
        assert_eq!(
            program_fill_allowed(&discord, &homoglyph.as_identity()),
            Err(Refusal::NoMatch)
        );
        // No similar bound program, no warning.
        assert_eq!(
            propose_binding(&signed("slack.exe"), &["discord.exe"])
                .unwrap()
                .warnings,
            Vec::new()
        );
    }

    #[test]
    fn excluded_programs_cannot_be_bound() {
        let mut browser = signed("chrome.exe");
        browser.class = ProgramClass::Browser;
        assert_eq!(
            propose_binding(&browser, &[]),
            Err(BindRefusal::Excluded(ProgramClass::Browser))
        );
    }

    #[test]
    fn changed_signer_or_file_asks_to_link_again() {
        let stale = || LoginUrl::new(&ProgramUrl::signed("tool.exe", OTHER).unwrap().canonical());
        let web = LoginUrl::new("https://example.com");
        let path_url = LoginUrl::new(&ProgramUrl::path(PATH, OTHER).unwrap().canonical());
        let saved = [web, stale(), path_url];
        assert_eq!(
            link_again_needed(&saved, &signed("tool.exe")),
            Some(BindingChange::SignerChanged)
        );
        assert_eq!(
            link_again_needed(&saved, &unsigned("tool.exe", PATH)),
            Some(BindingChange::FileChanged)
        );
        assert_eq!(link_again_needed(&saved, &signed("other.exe")), None);
        // A binding that still matches means nothing to link again.
        let good = LoginUrl::new(&ProgramUrl::signed("tool.exe", SIGNER).unwrap().canonical());
        assert_eq!(
            link_again_needed(&[stale(), good], &signed("tool.exe")),
            None
        );
    }
}
