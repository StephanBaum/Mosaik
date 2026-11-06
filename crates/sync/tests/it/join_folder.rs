//! Joining an account from a folder (ADR-0043): the account file, several
//! accounts side by side and a master password change.

use mosaic_core::Argon2Profile;
use mosaic_core::model::{ItemType, SecretField};
use mosaic_sync::folder::{self, FolderContents, JoinError};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use mosaic_sync::testutil::LocalSession;

use crate::common::{PASSWORD, World, everything, open_transport, pw};

#[test]
fn new_device_joins_and_converges() {
    let mut w = World::new(1, 2);
    let vault = w.new_vault(0, "Shared");
    let item = w.devs[0]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("bank")?;
            e.set_secret(SecretField::Password, "hunter2")
        })
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();

    let joiner = w.join();
    assert_eq!(joiner, 1);
    assert_eq!(w.devs[1].s().items(&everything()).len(), 0, "nothing yet");
    w.converge(2);

    {
        let (sa, sb) = (w.devs[0].s(), w.devs[1].s());
        let a = sa.item(vault, item).unwrap();
        let b = sb.item(vault, item).unwrap();
        assert_eq!(a, b);
        assert_eq!(b.title(), Some("bank"));
    }
    assert_eq!(
        w.devs[1]
            .s()
            .vaults()
            .iter()
            .map(|v| (v.vault_id, v.key_generation))
            .collect::<Vec<_>>(),
        w.devs[0]
            .s()
            .vaults()
            .iter()
            .map(|v| (v.vault_id, v.key_generation))
            .collect::<Vec<_>>()
    );
    // The new device edits and the first one sees it.
    w.devs[1]
        .s_mut()
        .update_item(vault, item, |e| e.set_title("bank 2"))
        .unwrap();
    w.converge(1);
    assert_eq!(
        w.devs[0].s().item(vault, item).unwrap().title(),
        Some("bank 2")
    );
}

#[test]
fn multiple_accounts_in_one_folder() {
    let mut w = World::new(2, 2);
    let vault = w.new_vault(0, "Mine");
    w.sync(0);

    // A second account in the same folder, written by another install.
    let dir_b = tempfile::tempdir().unwrap();
    let (session_b, created_b) =
        LockedSession::open(Store::open(dir_b.path().join("b.db")).unwrap())
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
    let replica0 = w.replica(0);
    let access_b = LocalSession::new(session_b);
    let mut transport_b = open_transport(&access_b, &replica0, &w.clock, "other");
    let boot = access_b.session().unwrap().account_bootstrap().unwrap();
    transport_b
        .write_account_file(&boot.params, &boot.wrapped_ak, 1)
        .unwrap();
    w.cloud.settle().unwrap();

    let accounts = folder::list_accounts(&w.replica(1));
    assert_eq!(accounts.len(), 2);
    assert!(accounts.contains(&w.account));
    assert!(accounts.contains(&access_b.session().unwrap().account_id()));

    // The user picks the account on their Emergency Kit: the right one
    // joins, and the other account's file does not open with these keys.
    let joiner = w.join();
    w.converge(1);
    assert!(
        w.devs[joiner]
            .s()
            .vaults()
            .iter()
            .any(|v| v.vault_id == vault)
    );
    let store = Store::open(dir_b.path().join("wrong.db")).unwrap();
    let failure = folder::join_from_folder(
        &w.replica(1),
        access_b.session().unwrap().account_id(),
        LockedSession::open(store),
        &pw(PASSWORD),
        &w.secret_key,
    )
    .unwrap_err();
    assert!(matches!(failure.error, JoinError::WrongCredentials));
    assert!(failure.session.is_some(), "the locked session comes back");
    let _ = created_b;
}

#[test]
fn password_change_writes_new_account_version_and_old_password_fails_join() {
    let mut w = World::new(3, 3);
    w.new_vault(0, "Shared");
    w.sync(0);
    w.cloud.settle().unwrap();

    let new_password = "a completely different passphrase";
    {
        let dev = &mut w.devs[0];
        let prepared = dev
            .access
            .session_mut()
            .unwrap()
            .prepare_master_password_change(&pw(PASSWORD), &pw(new_password), &w.secret_key)
            .unwrap();
        let version = dev
            .engine
            .transport_mut()
            .highest_account_version()
            .unwrap()
            + 1;
        assert_eq!(version, 2);
        dev.engine
            .transport_mut()
            .write_account_file(prepared.new_params(), prepared.new_wrapped_ak(), version)
            .unwrap();
        dev.access
            .session_mut()
            .unwrap()
            .commit_master_password_change(prepared)
            .unwrap();
    }
    // Only the new version is left in the writer's replica.
    let files: Vec<String> = std::fs::read_dir(
        w.replica(0)
            .join("a")
            .join(mosaic_sync::wire::folder_names::dir_name(
                w.account.as_bytes(),
            ))
            .join("account"),
    )
    .unwrap()
    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
    .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(files[0].starts_with("0000000000000002-"));

    w.cloud.settle().unwrap();
    let failure = w.try_join(PASSWORD).unwrap_err();
    assert!(matches!(failure.error, JoinError::WrongCredentials));
    // The failed join hands the locked session (and its store) back.
    drop(failure);
    let joined = w.try_join(new_password).unwrap();
    w.converge(1);
    assert_eq!(w.devs[joined].s().vaults().len(), 1);
}

#[test]
fn inspect_reports_folder_states() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(folder::inspect(dir.path()), FolderContents::Empty);
    assert_eq!(
        folder::inspect(&dir.path().join("missing")),
        FolderContents::Missing
    );
    folder::init(dir.path()).unwrap();
    assert!(matches!(
        folder::inspect(dir.path()),
        FolderContents::Mosaic { .. }
    ));
}
