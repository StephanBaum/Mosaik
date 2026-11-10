//! Reading: list, search, get, references, TOTP, the generator.

use crate::common::{Fixture, GITHUB_PASSWORD, PASSWORD, golden};

fn unlocked() -> Fixture {
    let f = Fixture::new();
    f.unlock("5m");
    f
}

#[test]
fn list_and_search_json_golden() {
    let f = unlocked();
    let list = f.cmd(&["--json", "--no-input", "list"]).run();
    assert_eq!(list.code, 0, "{}", list.stderr);
    golden("list.json", &list.stdout);
    let search = f.cmd(&["--json", "--no-input", "search", "mail"]).run();
    assert_eq!(search.code, 0, "{}", search.stderr);
    golden("search.json", &search.stdout);
    // No secret value in a listing.
    assert!(!list.stdout.contains(GITHUB_PASSWORD) && !list.stdout.contains("mail-pw-xyz"));
}

#[test]
fn get_conceals_secret_fields() {
    let f = unlocked();
    let out = f
        .cmd(&["--json", "--no-input", "get", "mosaic://Personal/GitHub"])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc = out.json();
    assert_eq!(doc["data"]["fields"]["login.password"]["concealed"], true);
    assert_eq!(doc["data"]["fields"]["login.username"]["value"], "alice");
    assert_eq!(doc["data"]["fields"]["field:recovery"]["concealed"], true);
    assert_eq!(doc["data"]["otp"]["concealed"], true);
    assert!(doc["data"]["otp"]["secondsRemaining"].is_number());
    assert!(!out.stdout.contains(GITHUB_PASSWORD) && !out.stdout.contains("recov-123"));
    golden("get.json", &out.stdout);
    let text = f
        .cmd(&["--no-input", "get", "mosaic://Personal/GitHub"])
        .run();
    assert!(
        text.stdout.contains("login.password: ••••••••"),
        "{}",
        text.stdout
    );
    assert!(!text.stdout.contains(GITHUB_PASSWORD));
}

#[test]
fn get_reveal_prints() {
    let f = unlocked();
    let out = f
        .cmd(&[
            "--no-input",
            "get",
            "mosaic://Personal/GitHub",
            "--field",
            "password",
            "--reveal",
        ])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout, format!("{GITHUB_PASSWORD}\n"));
    let json = f
        .cmd(&[
            "--json",
            "--no-input",
            "get",
            "mosaic://Personal/GitHub",
            "--reveal",
        ])
        .run()
        .json();
    assert_eq!(
        json["data"]["fields"]["login.password"]["value"],
        GITHUB_PASSWORD
    );
    assert_eq!(
        json["data"]["fields"]["field:recovery"]["value"],
        "recov-123"
    );
}

#[test]
fn secret_field_without_reveal_exit_9() {
    let f = unlocked();
    for reference in [
        "mosaic://Personal/GitHub/password",
        "mosaic://Personal/GitHub/otp",
    ] {
        let out = f.cmd(&["--json", "--no-input", "get", reference]).run();
        assert_eq!(out.code, 9, "{reference}");
        assert_eq!(out.error_json()["error"]["code"], "revealRequired");
        assert!(!out.stdout.contains(GITHUB_PASSWORD));
    }
    // A non-secret field needs no flag.
    let out = f
        .cmd(&["--no-input", "get", "mosaic://Personal/GitHub/username"])
        .run();
    assert_eq!((out.code, out.stdout.as_str()), (0, "alice\n"));
}

#[test]
fn ambiguous_ref_exit_7_lists_no_secrets() {
    let f = unlocked();
    let out = f.cmd(&["--json", "--no-input", "get", "GitHub"]).run();
    assert_eq!(out.code, 7);
    let err = out.error_json();
    assert_eq!(err["error"]["code"], "ambiguousRef");
    let candidates = err["error"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    let all = out.stderr.clone();
    assert!(!all.contains(GITHUB_PASSWORD) && !all.contains("work-gh-pw"));
    assert!(
        candidates
            .iter()
            .any(|c| c["vault"] == "Work" && c["title"] == "GitHub")
    );
    // Text mode names the candidates too.
    let text = f.cmd(&["--no-input", "get", "GitHub"]).run();
    assert!(text.stderr.contains("Work / GitHub"), "{}", text.stderr);
}

#[test]
fn ref_by_name_and_id() {
    let f = unlocked();
    let listed = f.cmd(&["--json", "--no-input", "list"]).run().json();
    let mail = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["title"] == "Mail")
        .unwrap();
    let id = mail["id"].as_str().unwrap();
    let by = |reference: &str| {
        f.cmd(&["--no-input", "get", reference, "--field", "username"])
            .run()
    };
    assert_eq!(by("mosaic://Personal/Mail").stdout, "bob\n");
    assert_eq!(by("mosaic://Personal/Mail/username").stdout, "bob\n");
    assert_eq!(by(id).stdout, "bob\n");
    assert_eq!(by(&id.replace('-', "")).stdout, "bob\n");
    assert_eq!(by(&format!("mosaic://Personal/{id}")).stdout, "bob\n");
    assert_eq!(by("Mail").stdout, "bob\n");
    assert_eq!(by("mosaic://Nope/Mail").code, 4);
    assert_eq!(by("mosaic://Personal/Missing").code, 4);
    assert_eq!(by("mosaic://Personal").code, 2);
    // A name matches exactly, not by substring.
    assert_eq!(by("mosaic://Personal/Mai").code, 4);
}

#[test]
fn bidi_title_escaped() {
    let f = unlocked();
    let text = f.cmd(&["--no-input", "list"]).run();
    assert!(
        text.stdout.contains("evil\\u{202e}title"),
        "{}",
        text.stdout
    );
    assert!(!text.stdout.contains('\u{202e}'));
}

#[test]
fn totp_code_matches_core() {
    let f = Fixture::new();
    let unlock = f
        .cmd(&["unlock", "--password-stdin"])
        .stdin(PASSWORD)
        .at(59_000)
        .run();
    assert_eq!(unlock.code, 0, "{}", unlock.stderr);
    // RFC 6238 test vector, SHA-1, T = 59 s: 94287082, six digits 287082.
    let out = f
        .cmd(&["--no-input", "totp", "mosaic://Personal/GitHub"])
        .at(59_000)
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout, "287082\n");
    let json = f
        .cmd(&["--json", "--no-input", "totp", "mosaic://Personal/GitHub"])
        .at(59_000)
        .run()
        .json();
    assert_eq!(json["data"]["code"], "287082");
    assert_eq!(json["data"]["secondsRemaining"], 1);
}

#[test]
fn hotp_counter_advances() {
    let f = unlocked();
    let code = || {
        f.cmd(&["--no-input", "totp", "mosaic://Personal/HotpThing"])
            .run()
            .stdout
    };
    // RFC 4226 test vectors for counters 0 and 1.
    assert_eq!(code(), "755224\n");
    assert_eq!(code(), "287082\n");
}

#[test]
fn generate_lengths_and_words() {
    let f = Fixture::new();
    let out = f.cmd(&["generate", "--length", "32"]).run();
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout.trim_end().chars().count(), 32);
    let digits = f
        .cmd(&[
            "generate",
            "--length",
            "24",
            "--no-upper",
            "--no-lower",
            "--no-symbols",
        ])
        .run();
    assert!(
        digits.stdout.trim_end().chars().all(|c| c.is_ascii_digit()),
        "{}",
        digits.stdout
    );
    let words = f
        .cmd(&["generate", "--words", "4", "--separator", "."])
        .run();
    assert_eq!(words.stdout.trim_end().split('.').count(), 4);
    assert_eq!(f.cmd(&["generate", "--length", "1"]).run().code, 2);
    let json = f.cmd(&["--json", "generate"]).run().json();
    assert!(json["data"]["value"].as_str().unwrap().len() >= 8);
}

#[test]
fn generate_site_rules() {
    let f = Fixture::new();
    // A site the rule table knows, and one it does not: both generate a password.
    for site in ["apple.com", "no-such-site.invalid"] {
        let out = f.cmd(&["generate", "--site", site]).run();
        assert_eq!(out.code, 0, "{site}: {}", out.stderr);
        assert!(out.stdout.trim_end().len() >= 8);
    }
}

#[test]
fn ssh_commands() {
    let f = unlocked();
    let list = f.cmd(&["--json", "--no-input", "ssh", "list"]).run();
    assert_eq!(list.code, 0, "{}", list.stderr);
    assert_eq!(list.json()["data"].as_array().unwrap().len(), 0);
    let status = f.cmd(&["ssh", "agent-status"]).run();
    assert_eq!(status.code, 0);
    assert!(status.stdout.contains("SSH agent"));
}
