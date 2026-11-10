//! Store states the CLI must report: a halted sync (exit 6) and its typed acknowledgement, an open
//! rotation job (exit 10) and a hardware-protected vault (exit 8). The states are set up by
//! editing the fixture store with `mosaic-vault` before the binary runs.

use mosaic_core::unlock::PrfOutput;
use mosaic_core::{MasterPassword, SecretBytes, SecretKey};
use mosaic_vault::session::{
    DEFAULT_RP_ID, LockedSession, NewHardwareKey, RotationKind, UnlockedSession,
};
use mosaic_vault::storage::{Store, SyncConfig, SyncHalt, SyncKind, VaultFlag};

use crate::common::{Fixture, PASSWORD};

/// Opens the fixture store, applies `change`, and closes it again.
fn edit(f: &Fixture, change: impl FnOnce(&mut UnlockedSession)) {
    let mut session = LockedSession::open(Store::open(&f.store).unwrap())
        .unlock(
            &MasterPassword::new(PASSWORD.to_owned()),
            &SecretKey::parse(&f.secret_key).unwrap(),
        )
        .map_err(|e| e.error)
        .unwrap();
    change(&mut session);
}

fn with_folder_backend(f: &Fixture) {
    let root = f.tmp.path().join("cloud");
    std::fs::create_dir_all(&root).unwrap();
    edit(f, |s| {
        s.set_sync_config(Some(&SyncConfig {
            kind: SyncKind::Folder,
            server_url: None,
            email: None,
            folder_path: Some(root),
            device_name: "desktop".to_owned(),
        }))
        .unwrap();
    });
}

#[test]
fn sync_halt_exit_6() {
    let f = Fixture::new();
    with_folder_backend(&f);
    edit(&f, |s| s.set_sync_halt(SyncHalt::Rollback).unwrap());
    // `--status` reads the store only (no unlock) and reports the halt with exit 6.
    let status = f.cmd(&["--json", "sync", "--status"]).run();
    assert_eq!(status.code, 6, "{}", status.stderr);
    let doc = status.json();
    assert_eq!(doc["data"]["halted"], true);
    assert_eq!(doc["data"]["findings"][0], "Rollback");
    // A sync run does not get past it either.
    f.unlock("5m");
    let run = f.cmd(&["--json", "--no-input", "sync"]).run();
    assert_eq!(run.code, 6, "{}", run.stderr);
}

#[test]
fn acknowledge_needs_typed_word() {
    let f = Fixture::new();
    with_folder_backend(&f);
    edit(&f, |s| {
        let v = s.vaults()[0].vault_id;
        s.set_sync_halt(SyncHalt::Identity).unwrap();
        s.set_vault_flag(v, VaultFlag::ServerInconsistent).unwrap();
    });
    f.unlock("5m");
    let status = f.cmd(&["--json", "sync", "--status"]).run();
    assert_eq!(status.code, 6);
    let findings = status.json()["data"]["findings"].to_string();
    assert!(findings.contains("Identity"), "{findings}");
    assert!(findings.contains("ServerInconsistent"), "{findings}");

    // There is no flag that skips the prompt: scripts cannot acknowledge blind.
    let blind = f
        .cmd(&["--json", "--no-input", "sync", "--acknowledge"])
        .run();
    assert_eq!(blind.code, 3, "{}", blind.stderr);
    assert_eq!(blind.error_json()["error"]["code"], "inputRequired");
    let yes = f.cmd(&["--json", "sync", "--acknowledge", "--yes"]).run();
    assert_eq!(yes.code, 2, "no --yes: {}", yes.stderr);

    // Anything but the word cancels and changes nothing.
    for typed in ["y", "yes", "ACKNOWLEDGE", ""] {
        let out = f
            .cmd(&["--json", "sync", "--acknowledge"])
            .prompts(&[typed])
            .run();
        assert_eq!(out.code, 1, "{typed:?}: {}", out.stderr);
        assert_eq!(out.error_json()["error"]["code"], "cancelled");
        assert_eq!(f.cmd(&["--json", "sync", "--status"]).run().code, 6);
    }

    // The word clears the halt and the flag.
    let ok = f
        .cmd(&["--json", "sync", "--acknowledge"])
        .prompts(&["acknowledge"])
        .run();
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert_eq!(ok.json()["data"]["acknowledged"], true);
    let clean = f.cmd(&["--json", "sync", "--status"]).run();
    assert_eq!(clean.code, 0, "{}", clean.stderr);
    assert_eq!(clean.json()["data"]["halted"], false);
    // Nothing left to acknowledge.
    let nothing = f
        .cmd(&["--json", "sync", "--acknowledge"])
        .prompts(&["acknowledge"])
        .run();
    assert_eq!(nothing.code, 0);
}

#[test]
fn rotation_job_blocks_writes() {
    let f = Fixture::new();
    with_folder_backend(&f);
    edit(&f, |s| {
        s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
    });
    f.unlock("5m");
    let csv = f.tmp.path().join("logins.csv");
    std::fs::write(&csv, "title,username,password\nBank,carol,pw-1\n").unwrap();
    for args in [
        vec!["--json", "--no-input", "sync"],
        vec!["--json", "--no-input", "recover"],
        vec![
            "--json",
            "import",
            "csv",
            csv.to_str().unwrap(),
            "--vault",
            "Personal",
        ],
    ] {
        let out = f.cmd(&args).prompts(&["y"]).run();
        assert_eq!(out.code, 10, "{args:?}: {}", out.stderr);
        assert_eq!(out.error_json()["error"]["code"], "rotationInProgress");
    }
    // Reading is not blocked, and nothing was imported.
    let list = f.cmd(&["--json", "list"]).run();
    assert_eq!(list.code, 0, "{}", list.stderr);
    assert!(!list.stdout.contains("Bank"));
}

#[test]
fn hardware_vault_item_exit_8() {
    let f = Fixture::new();
    // A hardware-protected vault needs its security key to open: a fake PRF output creates one
    // without hardware, and a CLI unlock (password only) cannot open it.
    let mut locked = String::new();
    edit(&f, |s| {
        let key =
            NewHardwareKey::new(vec![0x42; 32], DEFAULT_RP_ID.to_owned(), [0x55; 32]).unwrap();
        let prf = PrfOutput::from_secret(SecretBytes::from_array(&mut [0x77; 32]));
        locked = s
            .create_hardware_vault("Locked", &key, &prf)
            .unwrap()
            .to_string();
    });
    f.unlock("5m");
    // Its name is sealed with it, so it is named by ID (a name finds nothing: exit 4).
    let by_name = f.cmd(&["--json", "list", "--vault", "Locked"]).run();
    assert_eq!(by_name.code, 4, "{}", by_name.stderr);
    let out = f.cmd(&["--json", "list", "--vault", &locked]).run();
    assert_eq!(out.code, 8, "{}", out.stderr);
    assert_eq!(out.error_json()["error"]["code"], "vaultUnavailable");
    // The other vaults still work.
    let ok = f.cmd(&["--json", "list", "--vault", "Personal"]).run();
    assert_eq!(ok.code, 0, "{}", ok.stderr);
}
