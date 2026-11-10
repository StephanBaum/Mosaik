//! Input caps and file marks: the `inject` stdin cap (16 MiB) and, on Windows, the mark of the web
//! on exports. The import file cap (`MAX_FILE_LEN`, 256 MiB) is tested on its reader with a small
//! cap (`cmd/transfer.rs`): a 256 MiB fixture file would cost more memory than the test is worth.

use crate::common::{Fixture, GITHUB_PASSWORD};

const MAX_TEMPLATE: usize = 16 * 1024 * 1024;

#[test]
fn inject_stdin_cap() {
    let f = Fixture::new();
    // One byte over the cap (the harness adds a newline): refused before anything is unlocked.
    let over = "a".repeat(MAX_TEMPLATE);
    let out = f
        .cmd(&["--json", "--no-input", "inject"])
        .stdin(&over)
        .run();
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(out.error_json()["error"]["code"], "templateTooLarge");
    assert_eq!(out.stdout.len(), 0);
}

#[test]
fn inject_stdin_at_the_cap_is_read() {
    let f = Fixture::new();
    f.unlock("5m");
    let exact = "a".repeat(MAX_TEMPLATE - 1);
    let out = f.cmd(&["--no-input", "inject"]).stdin(&exact).run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.len(), MAX_TEMPLATE);
    assert!(!out.stdout.contains(GITHUB_PASSWORD));
}

#[test]
fn inject_file_cap() {
    let f = Fixture::new();
    let template = f.tmp.path().join("big.tpl");
    std::fs::write(&template, "b".repeat(MAX_TEMPLATE + 1)).unwrap();
    let out = f
        .cmd(&[
            "--json",
            "--no-input",
            "inject",
            "--in",
            template.to_str().unwrap(),
        ])
        .run();
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(out.error_json()["error"]["code"], "templateTooLarge");
}

#[cfg(windows)]
#[test]
fn export_is_marked_as_downloaded() {
    let f = Fixture::new();
    f.unlock("5m");
    let file = f.tmp.path().join("backup.mosaicx");
    let out = f
        .cmd(&["--json", "export", file.to_str().unwrap()])
        .prompts(&["a passphrase of its own", "a passphrase of its own"])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let mut stream = file.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let mark = std::fs::read_to_string(&stream).unwrap();
    assert!(mark.contains("[ZoneTransfer]"), "{mark}");
    assert!(mark.contains("ZoneId=3"), "{mark}");
}
