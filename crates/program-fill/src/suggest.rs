//! Search terms for linking a program to a login (ADR-0164): words from the
//! program's own identity, matched against login titles and hosts. A
//! suggestion is only a pre-selection in the link list; it never links.

use mosaic_origin::program::fold_name;

use crate::identity::CapturedIdentity;

/// The shortest term kept, in characters.
pub const MIN_TERM_CHARS: usize = 3;
/// The most suggestions shown.
pub const MAX_SUGGESTIONS: usize = 5;
/// Words too common in publisher names to point at a program.
pub const STOP_WORDS: &[&str] = &[
    "microsoft",
    "corporation",
    "corp",
    "inc",
    "llc",
    "ltd",
    "gmbh",
    "windows",
    "app",
    "software",
    "the",
];

fn add_words(out: &mut Vec<String>, text: &str, separators: impl Fn(char) -> bool) {
    for word in text.split(separators) {
        let folded = fold_name(word);
        if folded.chars().count() >= MIN_TERM_CHARS
            && !STOP_WORDS.contains(&folded.as_str())
            && !out.contains(&folded)
        {
            out.push(folded);
        }
    }
}

/// The terms of a program: the exe stem, the package name split on `.`, the
/// signer's organization and common name split on non-alphanumerics; folded,
/// at least [`MIN_TERM_CHARS`] long, without [`STOP_WORDS`], deduplicated in
/// that order.
#[must_use]
pub fn suggestion_terms(identity: &CapturedIdentity) -> Vec<String> {
    let mut out = Vec::new();
    let exe = fold_name(&identity.exe_name);
    let stem = exe.strip_suffix(".exe").unwrap_or(&exe);
    add_words(&mut out, stem, |_| false);
    if let Some(family) = &identity.package_family {
        let name = family.split('_').next().unwrap_or(family);
        add_words(&mut out, name, |c| c == '.');
    }
    if let Some(signer) = &identity.signer_display {
        for text in [&signer.organization, &signer.common_name]
            .into_iter()
            .flatten()
        {
            add_words(&mut out, text, |c| !c.is_alphanumeric());
        }
    }
    out
}

/// Whether the title or any host contains any term (case-insensitive).
#[must_use]
pub fn suggests(terms: &[String], title: &str, hosts: &[&str]) -> bool {
    let title = fold_name(title);
    let hosts: Vec<String> = hosts.iter().map(|h| fold_name(h)).collect();
    terms
        .iter()
        .any(|t| title.contains(t.as_str()) || hosts.iter().any(|h| h.contains(t.as_str())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SignerDisplay;
    use mosaic_origin::program::ProgramClass;

    fn ident(exe: &str, org: Option<&str>) -> CapturedIdentity {
        CapturedIdentity {
            pid: 1,
            final_path: format!(r"C:\Apps\{exe}"),
            exe_name: exe.to_owned(),
            package_family: None,
            signer_id: None,
            signer_display: org.map(|o| SignerDisplay {
                common_name: None,
                organization: Some(o.to_owned()),
            }),
            file_sha256: None,
            class: ProgramClass::Ordinary,
        }
    }

    #[test]
    fn terms_come_from_exe_package_and_signer_without_stop_words() {
        let t = suggestion_terms(&ident("Slack.exe", Some("Slack Technologies, LLC")));
        assert_eq!(t, vec!["slack", "technologies"]);
        let t = suggestion_terms(&ident("calc.exe", Some("Microsoft Corporation")));
        assert_eq!(t, vec!["calc"]);
        let mut p = ident("x.exe", None);
        p.package_family = Some("microsoft.windowscalculator_8wekyb3d8bbwe".to_owned());
        assert_eq!(suggestion_terms(&p), vec!["windowscalculator"]);
    }

    #[test]
    fn suggestions_match_title_or_host_case_insensitively() {
        let t = vec!["slack".to_owned()];
        assert!(suggests(&t, "SLACK work", &[]));
        assert!(suggests(&t, "Work", &["app.slack.com"]));
        assert!(!suggests(&t, "Slick", &["example.com"]));
    }
}
