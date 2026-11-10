//! Import, encrypted export and sync.

use std::sync::Arc;

use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::OwnedSession;
use mosaic_sync::folder::{self, FolderConfig, FolderTransport};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};

use crate::common::{Fixture, GITHUB_PASSWORD, PASSWORD};

const PASSPHRASE: &str = "an export passphrase of its own";

fn exported() -> (Fixture, std::path::PathBuf) {
    let f = Fixture::new();
    f.unlock("5m");
    let file = f.tmp.path().join("backup.mosaicx");
    let out = f
        .cmd(&["--json", "export", file.to_str().unwrap()])
        .prompts(&[PASSPHRASE, PASSPHRASE])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let report = out.json();
    assert_eq!(report["data"]["items"], 6);
    assert_eq!(report["data"]["vaults"], 2);
    (f, file)
}

#[test]
fn export_import_round_trip_cli() {
    let (_source, file) = exported();
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..4], b"MOSX");
    // No plaintext in the file.
    assert!(!String::from_utf8_lossy(&bytes).contains("GitHub"));
    let target = Fixture::fresh(&["Imported"]);
    let preview = target
        .cmd(&[
            "--json",
            "import",
            "mosaicx",
            file.to_str().unwrap(),
            "--dry-run",
        ])
        .prompts(&[PASSPHRASE, PASSWORD])
        .run();
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    assert_eq!(preview.json()["data"]["count"], 6);
    let listed = target
        .cmd(&["--json", "list"])
        .prompts(&[PASSWORD])
        .run()
        .json();
    assert_eq!(
        listed["data"].as_array().unwrap().len(),
        0,
        "a dry run imports nothing"
    );
    let done = target
        .cmd(&["--json", "import", "mosaicx", file.to_str().unwrap()])
        .prompts(&[PASSPHRASE, PASSWORD, "y"])
        .run();
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(done.json()["data"]["imported"], 6);
    let after = target
        .cmd(&["--json", "list"])
        .prompts(&[PASSWORD])
        .run()
        .json();
    assert_eq!(after["data"].as_array().unwrap().len(), 6);
    let get = target
        .cmd(&[
            "get",
            "mosaic://Imported/Mail",
            "--field",
            "password",
            "--reveal",
        ])
        .prompts(&[PASSWORD])
        .run();
    assert_eq!(get.stdout, "mail-pw-xyz\n");
}

#[test]
fn export_wrong_passphrase_exit_3() {
    let (_source, file) = exported();
    let target = Fixture::fresh(&["Imported"]);
    let out = target
        .cmd(&["--json", "import", "mosaicx", file.to_str().unwrap()])
        .prompts(&["the wrong passphrase", PASSWORD, "y"])
        .run();
    assert_eq!(out.code, 3);
    assert_eq!(out.error_json()["error"]["code"], "exportUnreadable");
}

#[test]
fn export_truncated_exit_3() {
    let (_source, file) = exported();
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(&file, &bytes[..bytes.len() - 20]).unwrap();
    let target = Fixture::fresh(&["Imported"]);
    let out = target
        .cmd(&["--json", "import", "mosaicx", file.to_str().unwrap()])
        .prompts(&[PASSPHRASE, PASSWORD, "y"])
        .run();
    assert_eq!(out.code, 3);
    assert_eq!(out.error_json()["error"]["code"], "exportUnreadable");
}

#[test]
fn export_checks_the_passphrase_and_never_overwrites() {
    let f = Fixture::new();
    f.unlock("5m");
    let file = f.tmp.path().join("x.mosaicx");
    let mismatch = f
        .cmd(&["--json", "export", file.to_str().unwrap()])
        .prompts(&[PASSPHRASE, "a different passphrase!"])
        .run();
    assert_eq!(mismatch.code, 3);
    assert_eq!(mismatch.error_json()["error"]["code"], "passphraseMismatch");
    let weak = f
        .cmd(&["--json", "export", file.to_str().unwrap()])
        .prompts(&["short", "short"])
        .run();
    assert_eq!(weak.error_json()["error"]["code"], "passphraseWeak");
    assert!(!file.exists());
    std::fs::write(&file, "x").unwrap();
    let exists = f
        .cmd(&["--json", "export", file.to_str().unwrap()])
        .prompts(&[PASSPHRASE, PASSPHRASE])
        .run();
    assert_eq!(exists.error_json()["error"]["code"], "fileExists");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "x");
    // The passphrase never comes from an option.
    assert_eq!(f.cmd(&["export", "--passphrase", "x", "y"]).run().code, 2);
}

fn csv_file(f: &Fixture) -> std::path::PathBuf {
    let path = f.tmp.path().join("logins.csv");
    std::fs::write(&path, "title,username,password,url\nBank,carol,bank-secret-1,https://bank.example\nForum,dave,forum-secret-2,https://forum.example\n").unwrap();
    path
}

#[test]
fn import_dry_run_previews() {
    let f = Fixture::fresh(&["Main"]);
    let csv = csv_file(&f);
    let out = f
        .cmd(&[
            "--json",
            "import",
            "csv",
            csv.to_str().unwrap(),
            "--dry-run",
        ])
        .prompts(&[PASSWORD])
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc = out.json();
    assert_eq!(doc["data"]["count"], 2);
    assert_eq!(doc["data"]["counts"]["login"], 2);
    assert!(!out.stdout.contains("bank-secret-1"));
    let listed = f.cmd(&["--json", "list"]).prompts(&[PASSWORD]).run().json();
    assert_eq!(listed["data"].as_array().unwrap().len(), 0);
}

#[test]
fn import_requires_confirmation() {
    let f = Fixture::fresh(&["Main"]);
    let csv = csv_file(&f);
    // `--no-input` refuses: imports are not scriptable blind.
    f.unlock("5m");
    let refused = f
        .cmd(&[
            "--json",
            "--no-input",
            "import",
            "csv",
            csv.to_str().unwrap(),
        ])
        .run();
    assert_eq!(refused.code, 3);
    assert_eq!(refused.error_json()["error"]["code"], "inputRequired");
    // Anything but yes cancels.
    let no = f
        .cmd(&["import", "csv", csv.to_str().unwrap()])
        .prompts(&["n"])
        .run();
    assert_eq!(no.code, 0);
    assert!(no.stdout.contains("Nothing was imported"), "{}", no.stdout);
    let yes = f
        .cmd(&["--json", "import", "csv", csv.to_str().unwrap()])
        .prompts(&["y"])
        .run();
    assert_eq!(yes.json()["data"]["imported"], 2);
    // The same file again is all duplicates.
    let dup = f
        .cmd(&["--json", "import", "csv", csv.to_str().unwrap()])
        .prompts(&["y"])
        .run();
    assert_eq!(dup.json()["data"]["imported"], 0);
}

#[test]
fn sync_folder_and_own_store_join() {
    let f = Fixture::new();
    // A sync folder for the desktop's store.
    let root = f.tmp.path().join("cloud");
    std::fs::create_dir_all(&root).unwrap();
    folder::init(&root).unwrap();
    {
        let store = mosaic_vault::storage::Store::open(&f.store).unwrap();
        let mut session = mosaic_vault::session::LockedSession::open(store)
            .unlock(
                &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::SecretKey::parse(&f.secret_key).unwrap(),
            )
            .map_err(|e| e.error)
            .unwrap();
        session
            .set_sync_config(Some(&mosaic_vault::storage::SyncConfig {
                kind: mosaic_vault::storage::SyncKind::Folder,
                server_url: None,
                email: None,
                folder_path: Some(root.clone()),
                device_name: "desktop".to_owned(),
            }))
            .unwrap();
        // The desktop writes the account file when it sets a folder up (`folder_flow`).
        let bootstrap = session.account_bootstrap().unwrap();
        let (account_id, install_id) = (session.account_id(), session.install_id());
        let owned = OwnedSession::new(session);
        let mut transport = FolderTransport::open(FolderConfig {
            root: root.clone(),
            account_id,
            install_id,
            crypto: SessionCrypto::new(owned.clone()).unwrap(),
            clock: Arc::new(SystemClock),
            device: DeviceInfo {
                created_at_ms: 1,
                name: "desktop".to_owned(),
                platform: Platform::Linux,
            },
        })
        .unwrap();
        transport
            .write_account_file(&bootstrap.params, &bootstrap.wrapped_ak, 1)
            .unwrap();
    }
    f.unlock("5m");
    let status = f.cmd(&["--json", "sync", "--status"]).run();
    assert_eq!(status.code, 0, "{}", status.stderr);
    assert_eq!(status.json()["data"]["backend"], "folder");
    for _ in 0..2 {
        let out = f.cmd(&["--json", "--no-input", "sync"]).run();
        assert_eq!(out.code, 0, "{}", out.stderr);
    }
    // The CLI joins as its own device over the folder and receives the items.
    let own = f.tmp.path().join("own").join("vault.sqlite3");
    let setup = f
        .cmd(&[
            "setup",
            "own-store",
            "--folder",
            root.to_str().unwrap(),
            "--store",
            own.to_str().unwrap(),
        ])
        .no_store_flag()
        .prompts(&[PASSWORD])
        .run();
    assert_eq!(setup.code, 0, "{}", setup.stderr);
    let again = f
        .cmd(&[
            "setup",
            "own-store",
            "--folder",
            root.to_str().unwrap(),
            "--store",
            own.to_str().unwrap(),
        ])
        .no_store_flag()
        .prompts(&[PASSWORD])
        .run();
    assert_eq!(again.code, 1, "an existing own store is never replaced");
    for _ in 0..3 {
        let out = f
            .cmd(&["--json", "sync", "--store", own.to_str().unwrap()])
            .no_store_flag()
            .prompts(&[PASSWORD])
            .run();
        assert_eq!(out.code, 0, "{}", out.stderr);
        // The own store has another install ID and its own session slot: using it neither
        // refuses nor deletes the shared store's session.
        let desktop = f.cmd(&["--json", "--no-input", "sync"]).run();
        assert_eq!(desktop.code, 0, "{}", desktop.stderr);
    }
    let listed = f
        .cmd(&[
            "--json",
            "get",
            "mosaic://Personal/GitHub",
            "--field",
            "password",
            "--reveal",
            "--store",
            own.to_str().unwrap(),
        ])
        .no_store_flag()
        .prompts(&[PASSWORD])
        .run();
    assert_eq!(
        listed.json()["data"]["fields"]["login.password"]["value"],
        GITHUB_PASSWORD
    );
}

#[test]
fn sync_without_a_backend_and_unsupported_commands() {
    let f = Fixture::new();
    f.unlock("5m");
    let out = f.cmd(&["--json", "--no-input", "sync"]).run();
    assert_eq!(out.code, 1);
    assert_eq!(out.error_json()["error"]["code"], "syncNotConfigured");
    let recover = f.cmd(&["--json", "recover"]).run();
    assert_eq!(recover.error_json()["error"]["code"], "syncNotConfigured");
}
