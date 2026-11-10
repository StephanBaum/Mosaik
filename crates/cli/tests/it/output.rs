//! The output contract: JSON shape, errors on stderr, exit codes, languages, help.

use crate::common::Fixture;

#[test]
fn json_errors_on_stderr() {
    let f = Fixture::new();
    let out = f
        .cmd(&["--json", "--no-input", "get", "mosaic://Nope/Nothing"])
        .run();
    assert_eq!(out.stdout, "", "stdout stays empty on an error");
    let err = out.error_json();
    assert_eq!(err["v"], 1);
    assert_eq!(err["error"]["code"], "inputRequired");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--no-input")
    );
    // One JSON document on stdout for a result.
    let ok = f.cmd(&["--json", "--version"]).run();
    assert_eq!(ok.json()["v"], 1);
    assert!(ok.json()["data"]["version"].is_string());
}

#[test]
fn exit_codes_table() {
    let f = Fixture::new();
    f.unlock("5m");
    let code = |args: &[&str]| f.cmd(&[&["--no-input"], args].concat()).run().code;
    assert_eq!(code(&["list"]), 0);
    assert_eq!(code(&["nonsense"]), 2);
    assert_eq!(code(&["get"]), 2);
    assert_eq!(code(&["get", "mosaic://Personal/Missing"]), 4);
    assert_eq!(code(&["get", "GitHub"]), 7);
    assert_eq!(code(&["get", "mosaic://Personal/GitHub/password"]), 9);
    assert_eq!(code(&["list", "--vault", "Nope"]), 4);
    assert_eq!(code(&["--profile", "weird", "list"]), 2);
    let missing = Fixture::new();
    let gone = missing.tmp.path().join("nowhere.sqlite3");
    let out = missing
        .cmd(&["--no-input", "--store", gone.to_str().unwrap(), "list"])
        .no_store_flag()
        .run();
    assert_eq!(out.code, 4);
}

#[test]
fn messages_in_german() {
    let f = Fixture::new();
    let de = f.cmd(&["--lang", "de", "--no-input", "list"]).run();
    assert_eq!(de.code, 3);
    assert!(
        de.stderr.contains("Eine Eingabe ist nötig"),
        "{}",
        de.stderr
    );
    let by_env = f
        .cmd(&["--no-input", "list"])
        .env("MOSAIC_LANG", "de_DE.UTF-8")
        .run();
    assert!(
        by_env.stderr.contains("Eine Eingabe ist nötig"),
        "{}",
        by_env.stderr
    );
    let en = f
        .cmd(&["--lang", "en", "--no-input", "list"])
        .env("MOSAIC_LANG", "de")
        .run();
    assert!(en.stderr.contains("Input is needed"), "{}", en.stderr);
    // Error codes and JSON keys stay English; only the message changes.
    let json = f
        .cmd(&["--lang", "de", "--json", "--no-input", "list"])
        .run();
    assert_eq!(json.error_json()["error"]["code"], "inputRequired");
    // Usage errors too.
    let usage = f.cmd(&["--lang", "de", "frobnicate"]).run();
    assert!(
        usage.stderr.contains("Unbekannter Befehl"),
        "{}",
        usage.stderr
    );
}

#[test]
fn help_lists_every_command_but_hidden() {
    let f = Fixture::new();
    let help = f.cmd(&["help"]).run();
    assert_eq!(help.code, 0);
    for name in [
        "unlock",
        "lock",
        "status",
        "list",
        "search",
        "get",
        "totp",
        "generate",
        "run",
        "inject",
        "ssh",
        "import",
        "export",
        "sync",
        "setup",
        "recover",
        "completions",
        "help",
    ] {
        assert!(
            help.stdout.contains(&format!("  {name} ")),
            "{name} missing in help"
        );
    }
    assert!(!help.stdout.contains("__clipboard-hold"));
    let one = f.cmd(&["get", "--help"]).run();
    assert!(one.stdout.contains("--reveal") && one.stdout.contains("mosaic get REF"));
    assert_eq!(f.cmd(&["help", "__clipboard-hold"]).run().code, 2);
    let man = f.cmd(&["help", "--man"]).run();
    assert!(man.stdout.starts_with(".TH MOSAIC 1"));
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let completion = f.cmd(&["completions", shell]).run();
        assert_eq!(completion.code, 0, "{shell}");
        assert!(completion.stdout.contains("unlock"), "{shell}");
        assert!(!completion.stdout.contains("__clipboard-hold"), "{shell}");
    }
}
