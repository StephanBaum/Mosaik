//! The committed man page and shell completions equal what the binary generates from the command
//! table (spec §3.9). `UPDATE_DOCS=1 cargo test -p mosaic-cli --features test-util docs_are_current`
//! rewrites them.

use std::path::Path;
use std::process::Command;

fn generate(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env_clear()
        .env("MOSAIC_LANG", "en")
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "mosaic {args:?} failed");
    String::from_utf8(out.stdout).unwrap().replace("\r\n", "\n")
}

#[test]
fn docs_are_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files: [(&str, Vec<&str>); 5] = [
        ("man/mosaic.1", vec!["help", "--man"]),
        ("completions/mosaic.bash", vec!["completions", "bash"]),
        ("completions/_mosaic", vec!["completions", "zsh"]),
        ("completions/mosaic.fish", vec!["completions", "fish"]),
        ("completions/mosaic.ps1", vec!["completions", "powershell"]),
    ];
    let update = std::env::var_os("UPDATE_DOCS").is_some();
    let mut stale = Vec::new();
    for (path, args) in files {
        let generated = generate(&args);
        assert!(!generated.trim().is_empty(), "{path}: nothing generated");
        let file = root.join(path);
        if update {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, &generated).unwrap();
        }
        let committed = std::fs::read_to_string(&file)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if committed != generated {
            stale.push(path);
        }
    }
    assert!(
        stale.is_empty(),
        "stale (run with UPDATE_DOCS=1 to regenerate): {stale:?}"
    );
}

#[test]
fn every_command_is_in_the_man_page_and_completions() {
    let man = generate(&["help", "--man"]);
    let bash = generate(&["completions", "bash"]);
    let help = generate(&["help"]);
    // Every command line of `help` (two leading spaces, then the name) appears in both.
    let mut seen = 0;
    for line in help.lines() {
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        let name = rest.split_whitespace().next().unwrap_or_default();
        if name.is_empty() || name.starts_with('-') || name.starts_with('<') {
            continue;
        }
        seen += 1;
        assert!(man.contains(name), "{name} missing from the man page");
        assert!(
            bash.contains(name),
            "{name} missing from the bash completion"
        );
    }
    assert!(seen >= 10, "help lists the commands ({seen})");
}
