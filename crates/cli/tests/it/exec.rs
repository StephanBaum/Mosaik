//! `run`, `inject` and the clipboard helper.

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::common::{Fixture, GITHUB_PASSWORD};

fn unlocked() -> Fixture {
    let f = Fixture::new();
    f.unlock("5m");
    f
}

/// A command line that prints the value of environment variable `name`.
fn print_env(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".into(), "/C".into(), format!("echo [%{name}%]")]
    } else {
        vec![
            "sh".into(),
            "-c".into(),
            format!("printf '[%s]' \"${name}\""),
        ]
    }
}

fn run_args<'a>(head: &[&'a str], child: &'a [String]) -> Vec<&'a str> {
    let mut args: Vec<&str> = head.to_vec();
    args.push("--");
    args.extend(child.iter().map(String::as_str));
    args
}

#[test]
fn run_injects_only_requested_vars() {
    let f = unlocked();
    let child = print_env("GH_PW");
    let args = run_args(
        &[
            "--no-input",
            "run",
            "--env",
            "GH_PW=mosaic://Personal/GitHub/password",
        ],
        &child,
    );
    let out = f.cmd(&args).run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.contains(&format!("[{GITHUB_PASSWORD}]")),
        "{}",
        out.stdout
    );
    // Another variable of the same item is not injected.
    let other = print_env("GH_USER");
    let args = run_args(
        &[
            "--no-input",
            "run",
            "--env",
            "GH_PW=mosaic://Personal/GitHub/password",
        ],
        &other,
    );
    let out = f.cmd(&args).run();
    assert!(
        !out.stdout.contains("alice") && !out.stdout.contains(GITHUB_PASSWORD),
        "{}",
        out.stdout
    );
    // A bad name is a usage error.
    let args = run_args(
        &["--no-input", "run", "--env", "1BAD=mosaic://Personal/Mail"],
        &child,
    );
    assert_eq!(f.cmd(&args).run().code, 2);
}

#[test]
fn run_strips_mosaic_vars() {
    let f = unlocked();
    let child = print_env("MOSAIC_LEAK");
    let args = run_args(&["--no-input", "run"], &child);
    // Windows variable names are case-insensitive: a mixed-case name goes too.
    let out = f
        .cmd(&args)
        .env("MOSAIC_LEAK", "leaked-value")
        .env("Mosaic_Other", "leaked-value")
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(!out.stdout.contains("leaked-value"), "{}", out.stdout);
}

#[test]
fn run_propagates_exit_code() {
    let f = unlocked();
    let child: Vec<String> = if cfg!(windows) {
        vec!["cmd".into(), "/C".into(), "exit 7".into()]
    } else {
        vec!["sh".into(), "-c".into(), "exit 7".into()]
    };
    assert_eq!(
        f.cmd(&run_args(&["--no-input", "run"], &child)).run().code,
        7
    );
    let missing = vec!["mosaic-no-such-program".to_owned()];
    let out = f.cmd(&run_args(&["--no-input", "run"], &missing)).run();
    assert_eq!(out.code, 125, "{}", out.stderr);
}

#[test]
fn run_releases_store_lock_before_spawn() {
    let f = unlocked();
    // The child opens the same store: it would be exit 5 if `run` still held the lock.
    let store = f.store.to_str().unwrap().to_owned();
    let child = vec![
        env!("CARGO_BIN_EXE_mosaic").to_owned(),
        "--store".to_owned(),
        store,
        "status".to_owned(),
    ];
    let out = f.cmd(&run_args(&["--no-input", "run"], &child)).run();
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    assert!(out.stdout.contains("Store:"), "{}", out.stdout);
}

#[test]
fn inject_template_to_file_0600() {
    let f = unlocked();
    let template = f.tmp.path().join("app.env.tpl");
    std::fs::write(
        &template,
        "PASSWORD={{ mosaic://Personal/GitHub/password }}\nUSER={{mosaic://Personal/GitHub/username}}\nKEEP={{ other }}\n",
    )
    .unwrap();
    let dest = f.tmp.path().join("app.env");
    let out = f
        .cmd(&[
            "--no-input",
            "inject",
            "--in",
            template.to_str().unwrap(),
            "--out",
            dest.to_str().unwrap(),
        ])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        std::fs::read_to_string(&dest).unwrap(),
        format!("PASSWORD={GITHUB_PASSWORD}\nUSER=alice\nKEEP={{{{ other }}}}\n")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // It never overwrites.
    let again = f
        .cmd(&[
            "--no-input",
            "inject",
            "--in",
            template.to_str().unwrap(),
            "--out",
            dest.to_str().unwrap(),
        ])
        .run();
    assert_eq!(again.code, 1);
    // To a pipe the result goes to stdout.
    let piped = f
        .cmd(&["--no-input", "inject", "--in", template.to_str().unwrap()])
        .run();
    assert!(piped.stdout.contains(GITHUB_PASSWORD));
}

fn wait_for(f: &Fixture, want: &str, secs: u64) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if f.clipboard() == want {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn spawn_helper(f: &Fixture, value: &str, clear_after: &str) -> std::process::Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .args(["__clipboard-hold", "--clear-after", clear_after])
        .env("MOSAIC_TEST_DIR", &f.test_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(value.as_bytes())
        .unwrap();
    child
}

#[test]
fn copy_spawns_helper_secret_not_in_argv() {
    let f = unlocked();
    let out = f
        .cmd(&[
            "--no-input",
            "get",
            "mosaic://Personal/GitHub",
            "--field",
            "password",
            "--copy",
            "--clear-after",
            "30s",
        ])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        !out.stdout.contains(GITHUB_PASSWORD),
        "--copy does not print the value"
    );
    assert!(
        wait_for(&f, GITHUB_PASSWORD, 10),
        "the helper set the clipboard"
    );
    let argv = std::fs::read_to_string(f.test_dir.join("helper-argv.txt")).unwrap();
    assert!(argv.contains("__clipboard-hold") && argv.contains("30"));
    assert!(
        !argv.contains(GITHUB_PASSWORD),
        "the value is not on the helper's command line"
    );
    assert!(
        out.stderr.contains("Clears in 30 seconds"),
        "{}",
        out.stderr
    );
}

#[test]
fn helper_clears_after_ttl_if_unchanged() {
    let f = Fixture::new();
    let mut helper = spawn_helper(&f, "value-to-clear", "1");
    assert!(wait_for(&f, "value-to-clear", 10));
    assert!(wait_for(&f, "", 10), "cleared after the TTL");
    helper.wait().unwrap();
}

#[test]
fn helper_keeps_foreign_clipboard() {
    let f = Fixture::new();
    let mut helper = spawn_helper(&f, "value-of-ours", "1");
    assert!(wait_for(&f, "value-of-ours", 10));
    std::fs::write(
        f.test_dir.join("clipboard.txt"),
        "something the user copied",
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(2500));
    assert_eq!(
        f.clipboard(),
        "something the user copied",
        "not ours any more: left alone"
    );
    helper.wait().unwrap();
}

#[test]
fn clear_after_out_of_range_is_a_usage_error() {
    let f = unlocked();
    let out = f
        .cmd(&[
            "--no-input",
            "get",
            "mosaic://Personal/Mail",
            "--field",
            "password",
            "--copy",
            "--clear-after",
            "9999s",
        ])
        .run();
    assert_eq!(out.code, 2);
}
