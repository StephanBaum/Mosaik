//! Config file and command line.

use std::path::{Path, PathBuf};

use mosaic_server::cli::{CliError, run};
use mosaic_server::{ApiError, Config, ConfigError, DB_FILE, Db};

fn config_text(dir: &Path, extra: &str) -> String {
    let d = dir.display();
    format!(
        "listen = \"127.0.0.1:8743\"\ncanonical_origin = \"https://sync.example.com\"\n\
         data_dir = '{d}/data'\nsecret_file = '{d}/secret/server.key'\nregistration = \"invite\"\n{extra}"
    )
}

fn write_config(dir: &Path, extra: &str) -> PathBuf {
    let p = dir.join("mosaic-server.toml");
    std::fs::write(&p, config_text(dir, extra)).unwrap();
    p
}

fn cli(args: &[&str]) -> Result<String, CliError> {
    let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    let mut out = Vec::new();
    run(&args, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

#[test]
fn init_refuses_existing() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "");
    let cfg = cfg.to_str().unwrap();

    let out = cli(&["init", "--config", cfg]).unwrap();
    assert!(out.contains("initialised"));
    let secret = dir.path().join("secret/server.key");
    assert!(secret.is_file());
    assert!(dir.path().join("data").join(DB_FILE).is_file());
    assert!(dir.path().join("data/blobs").is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&secret).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    // Neither file is overwritten.
    let before = std::fs::read(&secret).unwrap();
    assert!(matches!(
        cli(&["init", "--config", cfg]),
        Err(CliError::Exists(_))
    ));
    assert_eq!(std::fs::read(&secret).unwrap(), before);
    // Either one alone is enough to refuse.
    std::fs::remove_file(dir.path().join("data").join(DB_FILE)).ok();
    for ext in ["-wal", "-shm"] {
        std::fs::remove_file(dir.path().join("data").join(format!("{DB_FILE}{ext}"))).ok();
    }
    assert!(matches!(
        cli(&["init", "--config", cfg]),
        Err(CliError::Exists("the secret file"))
    ));
    std::fs::remove_file(&secret).unwrap();
    std::fs::write(dir.path().join("data").join(DB_FILE), b"x").unwrap();
    assert!(matches!(
        cli(&["init", "--config", cfg]),
        Err(CliError::Exists("the database"))
    ));
    assert!(!secret.exists(), "nothing was created");
}

#[test]
fn check_config_rejects_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let ok = write_config(dir.path(), "");
    let out = cli(&["check-config", "--config", ok.to_str().unwrap()]).unwrap();
    assert!(out.contains("config ok") && out.contains("https://sync.example.com"));
    assert!(out.contains("not initialised"));

    for (extra, what) in [
        ("bogus = 1\n", "unknown top-level key"),
        (
            "trusted_proxies = [\"127.0.0.1\"]\n[limits]\nbogus = 1\n",
            "unknown limits key",
        ),
        (
            "[limits]\nmax_request_bytes = 10\n",
            "request limit below the minimum",
        ),
        (
            "[limits]\nsession_idle_days = 200\n",
            "idle longer than the maximum",
        ),
    ] {
        let p = write_config(dir.path(), extra);
        let r = cli(&["check-config", "--config", p.to_str().unwrap()]);
        assert!(matches!(r, Err(CliError::Config(_))), "{what}");
    }
    // A bad origin, a missing file and bad arguments.
    let text =
        config_text(dir.path(), "").replace("https://sync.example.com", "http://sync.example.com");
    assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    assert!(matches!(
        cli(&[
            "check-config",
            "--config",
            dir.path().join("missing.toml").to_str().unwrap()
        ]),
        Err(CliError::Config(ConfigError::Read))
    ));
    assert!(matches!(cli(&["check-config"]), Err(CliError::Usage(_))));
    assert!(matches!(
        cli(&["frobnicate", "--config", "x"]),
        Err(CliError::Config(_) | CliError::Usage(_))
    ));
    assert!(matches!(cli(&[]), Err(CliError::Usage(_))));
}

#[test]
fn backup_produces_openable_db() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "");
    let cfg = cfg.to_str().unwrap();
    // Not initialised yet.
    assert!(
        cli(&[
            "backup",
            "--config",
            cfg,
            "--out",
            dir.path().join("b0").to_str().unwrap()
        ])
        .is_err()
    );
    cli(&["init", "--config", cfg]).unwrap();

    // An invite code: 20 Crockford characters, shown once, stored hashed.
    let code = cli(&["invite", "create", "--config", cfg]).unwrap();
    let code = code.trim();
    assert_eq!(code.len(), 20);
    assert!(
        code.bytes()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b)),
        "{code}"
    );
    let other = cli(&["invite", "create", "--config", cfg]).unwrap();
    assert_ne!(other.trim(), code);

    // A blob to back up.
    let blob_dir = dir.path().join("data/blobs/ab");
    std::fs::create_dir_all(&blob_dir).unwrap();
    std::fs::write(
        blob_dir.join("ab000000000000000000000000000000.matt"),
        b"cipher",
    )
    .unwrap();

    let out = dir.path().join("backup.sqlite");
    cli(&[
        "backup",
        "--config",
        cfg,
        "--out",
        out.to_str().unwrap(),
        "--blobs",
    ])
    .unwrap();
    let copy = Db::open(&out).unwrap();
    let n: i64 = copy
        .run(|c| Ok::<_, ApiError>(c.query_row("SELECT COUNT(*) FROM invites", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(n, 2);
    // The stored value is a hash, not the code.
    let stored: Vec<u8> = copy
        .run(|c| {
            Ok::<_, ApiError>(
                c.query_row("SELECT code_hash FROM invites LIMIT 1", [], |r| r.get(0))?,
            )
        })
        .unwrap();
    assert_eq!(stored.len(), 32);
    assert!(
        !std::fs::read(&out)
            .unwrap()
            .windows(20)
            .any(|w| w == code.as_bytes())
    );
    assert_eq!(
        std::fs::read(
            dir.path()
                .join("backup.sqlite.blobs/ab/ab000000000000000000000000000000.matt")
        )
        .unwrap(),
        b"cipher"
    );
    // A second backup to the same target is refused.
    assert!(matches!(
        cli(&["backup", "--config", cfg, "--out", out.to_str().unwrap()]),
        Err(CliError::BackupTargetExists)
    ));
}

#[test]
fn account_delete_unknown_email() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "");
    let cfg = cfg.to_str().unwrap();
    cli(&["init", "--config", cfg]).unwrap();
    assert!(matches!(
        cli(&[
            "account",
            "delete",
            "--config",
            cfg,
            "--email",
            "nobody@example.com"
        ]),
        Err(CliError::NoSuchAccount)
    ));
    assert!(matches!(
        cli(&[
            "account",
            "delete",
            "--config",
            cfg,
            "--email",
            "not an address"
        ]),
        Err(CliError::BadEmail)
    ));
}

#[test]
fn state_opens_from_an_initialised_directory() {
    use std::sync::Arc;

    use mosaic_server::{AppState, SystemClock};

    let dir = tempfile::tempdir().unwrap();
    let path = write_config(dir.path(), "");
    cli(&["init", "--config", path.to_str().unwrap()]).unwrap();
    let config = Config::load(&path).unwrap();
    let state = AppState::open(&config, Arc::new(SystemClock)).unwrap();
    state.maintain().unwrap();
    // A damaged secret file is refused.
    std::fs::write(&config.secret_file, b"not a secret file").unwrap();
    assert!(AppState::open(&config, Arc::new(SystemClock)).is_err());
}

#[tokio::test(start_paused = true)]
async fn account_delete_command_removes_the_account() {
    use axum::http::StatusCode;
    use mosaic_sync::wire::json::RegistrationMode;

    use crate::common::TestApp;

    // The command works on the database file of a running server.
    let app = TestApp::with_file_db(RegistrationMode::Open);
    let (u, token) = app.user("alice").await;
    let d = app.dir.path().display();
    let cfg = app.dir.path().join("cli.toml");
    std::fs::write(
        &cfg,
        format!(
            "listen = \"127.0.0.1:8743\"\ncanonical_origin = \"https://sync.example.com\"\n\
             data_dir = '{d}'\nsecret_file = '{d}/server.key'\nregistration = \"open\"\n"
        ),
    )
    .unwrap();
    let out = cli(&[
        "account",
        "delete",
        "--config",
        cfg.to_str().unwrap(),
        "--email",
        &u.email.to_uppercase(),
    ])
    .unwrap();
    assert!(out.contains("account deleted"));
    assert_eq!(
        app.get("/v1/account", Some(&token)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn new_limits_are_validated() {
    let dir = tempfile::tempdir().unwrap();
    for (extra, what) in [
        ("[limits]\nmax_concurrent_requests = 0\n", "no concurrency"),
        (
            "[limits]\nmax_concurrent_requests = 1000000\n",
            "absurd concurrency",
        ),
        ("[limits]\nmax_key_generations = 0\n", "no key generations"),
    ] {
        let p = write_config(dir.path(), extra);
        assert!(
            matches!(Config::load(&p), Err(ConfigError::Invalid(_))),
            "{what}"
        );
    }
    let ok = Config::load(&write_config(dir.path(), "")).unwrap();
    assert_eq!(ok.limits.max_concurrent_requests, 512);
    assert_eq!(ok.limits.max_key_generations, 10_000);
}

#[cfg(unix)]
#[test]
fn files_are_private_and_a_readable_secret_file_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    use mosaic_server::{AppState, SecretFileError, ServerKeys, StartError, SystemClock};

    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(dir.path(), "");
    cli(&["init", "--config", path.to_str().unwrap()]).unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(mode(&config.data_dir), 0o700);
    assert_eq!(mode(&config.secret_file), 0o600);
    let db = config.data_dir.join(DB_FILE);
    assert_eq!(mode(&db), 0o600);

    // The WAL and shared-memory files take the mode of the database file.
    let state = AppState::open(&config, Arc::new(SystemClock)).unwrap();
    state
        .db()
        .run(|c| {
            c.execute("CREATE TABLE IF NOT EXISTS probe (x)", [])?;
            Ok::<_, ApiError>(())
        })
        .unwrap();
    for ext in ["-wal", "-shm"] {
        let f = config.data_dir.join(format!("{DB_FILE}{ext}"));
        if f.exists() {
            assert_eq!(mode(&f), 0o600, "{ext}");
        }
    }

    // A secret file that others can read is refused, by the loader and at start.
    std::fs::set_permissions(&config.secret_file, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert!(matches!(
        ServerKeys::load(&config.secret_file),
        Err(SecretFileError::Permissions)
    ));
    assert!(matches!(
        AppState::open(&config, Arc::new(SystemClock)),
        Err(StartError::Secret(SecretFileError::Permissions))
    ));
    std::fs::set_permissions(&config.secret_file, std::fs::Permissions::from_mode(0o604)).unwrap();
    assert!(ServerKeys::load(&config.secret_file).is_err());
    std::fs::set_permissions(&config.secret_file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(ServerKeys::load(&config.secret_file).is_ok());
}
