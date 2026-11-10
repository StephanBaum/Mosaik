//! `OwnedSession` (M8, ADR-0071): a command-line tool opens its own store,
//! joins the account as its own device and syncs with the engine through a
//! session it owns, then takes the session back.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::sync::Arc;

use mosaic_core::model::{ItemType, SecretField};
use mosaic_sync::clock::Clock;
use mosaic_sync::engine::{EngineConfig, OwnedSession, SyncEngine, SyncError};
use mosaic_sync::folder::{self, FolderConfig, FolderTransport};
use mosaic_sync::testutil::{FixedJitter, ManualClock};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use crate::common::{PASSWORD, World, everything, pw};

type Engine = SyncEngine<FolderTransport<SessionCrypto<OwnedSession>>>;

fn engine_for(owned: &OwnedSession, root: &std::path::Path, clock: &ManualClock) -> Engine {
    let crypto = SessionCrypto::new(owned.clone()).unwrap();
    let (account_id, install_id) = (
        crypto.account_id(),
        mosaic_sync::SyncCrypto::install_id(&crypto),
    );
    let transport = FolderTransport::open(FolderConfig {
        root: root.to_owned(),
        account_id,
        install_id,
        crypto,
        clock: Arc::new(clock.clone()),
        device: DeviceInfo {
            created_at_ms: clock.now_ms(),
            name: "cli".to_owned(),
            platform: Platform::Linux,
        },
    })
    .unwrap();
    SyncEngine::with_jitter(
        transport,
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    )
}

#[test]
fn owned_session_syncs_a_second_device_and_hands_the_session_back() {
    let mut w = World::new(7, 2);
    let vault = w.new_vault(0, "Shared");
    w.devs[0]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("from the desktop")?;
            e.set_secret(SecretField::Password, "hunter2")
        })
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();

    // The CLI's own store joins from the folder (the M6 join) and syncs
    // through an owned session.
    let store = Store::open(w.tmp.path().join("cli.db")).unwrap();
    let session = folder::join_from_folder(
        w.cloud.replica(1),
        w.account,
        LockedSession::open(store),
        &pw(PASSWORD),
        &w.secret_key,
    )
    .map_err(|f| f.error)
    .unwrap();
    let mut owned = OwnedSession::new(session);
    assert!(owned.is_unlocked());
    let mut engine = engine_for(&owned, w.cloud.replica(1), &w.clock);

    let count = |owned: &OwnedSession| {
        mosaic_sync::wire::SessionHandle::with_session(owned, |s| s.items(&everything()).len())
    };
    for _ in 0..3 {
        w.cloud.settle().unwrap();
        engine.sync_once(&mut owned, &w.clock).unwrap();
        w.cloud.settle().unwrap();
        w.sync(0);
    }
    assert_eq!(count(&owned), Some(1), "the desktop's item arrived");

    // An item the CLI adds reaches the desktop.
    let added = mosaic_sync::wire::SessionHandle::with_session_mut(&owned, |s| {
        let vault = s.vaults().first().unwrap().vault_id;
        s.add_item(vault, &ItemType::Login, |e| e.set_title("from the cli"))
            .map(|id| (vault, id))
    })
    .unwrap()
    .unwrap();
    for _ in 0..3 {
        engine.sync_once(&mut owned, &w.clock).unwrap();
        w.cloud.settle().unwrap();
        w.sync(0);
        w.cloud.settle().unwrap();
    }
    assert_eq!(
        w.devs[0].s().item(added.0, added.1).unwrap().title(),
        Some("from the cli")
    );

    // The caller takes the session back; the engine then refuses to run.
    let clone = owned.clone();
    let session = owned.take();
    assert!(session.is_some());
    assert!(!owned.is_unlocked() && !clone.is_unlocked());
    assert!(owned.take().is_none());
    assert!(matches!(
        engine.sync_once(&mut owned, &w.clock),
        Err(SyncError::Locked)
    ));
    assert_eq!(count(&clone), None);
}
