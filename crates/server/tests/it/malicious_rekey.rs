//! Hostile re-key scenarios (M7 spec §7.4, CRYPTO_SPEC §6.7, §10.1): what
//! someone who writes to the folder, or a server that lies, can and cannot do
//! to a device that has not re-keyed. Nothing here may move a device to a key
//! the user did not type, halt it for good, or crowd out the genuine re-key.

use std::sync::Arc;

use mosaic_core::{
    AccountId, AccountSalt, Argon2Profile, InstallId, MasterPassword, SecretKey, VaultId,
    WrappedKey,
};
use mosaic_sync::engine::{
    EngineConfig, HostedRekey, RekeyError, RekeySecrets, SyncEngine, SyncError, SyncWarning,
    adopt_hosted_rekey,
};
use mosaic_sync::folder::{self, FolderConfig, FolderTransport};
use mosaic_sync::http::hosted;
use mosaic_sync::testutil::{FixedJitter, LocalSession, ManualClock};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::account_file::AccountFile;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_sync::wire::folder_names::{account_name, dir_name};
use mosaic_sync::wire::{KeyedCrypto, SessionCrypto};
use mosaic_vault::session::{LockedSession, RekeyAdoption, SyncConfig, SyncKind};
use mosaic_vault::storage::Store;

use crate::hosted::{PASSWORD, World, pw};

const NEW_PASSWORD: &str = "a different horse battery staple";

fn copy_key(sk: &SecretKey) -> SecretKey {
    SecretKey::parse(sk.to_display().unwrap().expose_secret()).unwrap()
}

// ---- a folder shared by devices in one directory ---------------------------

type FolderEngine = SyncEngine<FolderTransport<SessionCrypto<LocalSession>>>;

struct FDev {
    access: LocalSession,
    engine: FolderEngine,
}

struct FolderWorld {
    _tmp: tempfile::TempDir,
    dir: std::path::PathBuf,
    clock: ManualClock,
    devs: Vec<FDev>,
    secret_key: SecretKey,
    account: AccountId,
}

fn engine_for(
    access: &LocalSession,
    dir: &std::path::Path,
    clock: &ManualClock,
    name: &str,
) -> FolderEngine {
    let crypto = SessionCrypto::new(access.clone()).unwrap();
    let (account_id, install_id) = (
        crypto.account_id(),
        mosaic_sync::SyncCrypto::install_id(&crypto),
    );
    let transport = FolderTransport::open(FolderConfig {
        root: dir.to_owned(),
        account_id,
        install_id,
        crypto,
        clock: Arc::new(clock.clone()),
        device: DeviceInfo {
            created_at_ms: mosaic_sync::clock::Clock::now_ms(clock),
            name: name.to_owned(),
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

fn configure(access: &LocalSession, dir: &std::path::Path, name: &str) {
    let config = SyncConfig {
        kind: SyncKind::Folder,
        server_url: None,
        email: None,
        folder_path: Some(dir.to_owned()),
        device_name: name.to_owned(),
    };
    access
        .session_mut()
        .unwrap()
        .set_sync_config(Some(&config))
        .unwrap();
}

/// Two devices of one account on one folder (no vault: the scenarios are about
/// account files).
fn folder_world() -> FolderWorld {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("folder");
    std::fs::create_dir_all(&dir).unwrap();
    let (session, created) = LockedSession::open(Store::open(tmp.path().join("d0.db")).unwrap())
        .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
        .unwrap();
    folder::init(&dir).unwrap();
    let clock = ManualClock::new();
    let account = session.account_id();
    let access0 = LocalSession::new(session);
    let mut engine0 = engine_for(&access0, &dir, &clock, "device 0");
    let boot = access0.session().unwrap().account_bootstrap().unwrap();
    engine0
        .transport_mut()
        .write_account_file(&boot.params, &boot.wrapped_ak, 1)
        .unwrap();
    let session1 = folder::join_from_folder(
        &dir,
        account,
        LockedSession::open(Store::open(tmp.path().join("d1.db")).unwrap()),
        &pw(PASSWORD),
        &created.secret_key,
    )
    .map_err(|f| f.error)
    .unwrap();
    let access1 = LocalSession::new(session1);
    let engine1 = engine_for(&access1, &dir, &clock, "device 1");
    configure(&access0, &dir, "device 0");
    configure(&access1, &dir, "device 1");
    FolderWorld {
        _tmp: tmp,
        dir,
        clock,
        devs: vec![
            FDev {
                access: access0,
                engine: engine0,
            },
            FDev {
                access: access1,
                engine: engine1,
            },
        ],
        secret_key: created.secret_key,
        account,
    }
}

impl FolderWorld {
    fn sync(&mut self, d: usize) -> Result<mosaic_sync::engine::SyncReport, SyncError> {
        let dev = &mut self.devs[d];
        dev.engine.sync_once(&mut dev.access, &self.clock)
    }

    fn account_dir(&self) -> std::path::PathBuf {
        self.dir
            .join("a")
            .join(dir_name(self.account.as_bytes()))
            .join("account")
    }

    /// Device 0 re-keys to `(NEW_PASSWORD, new_sk)` and finishes.
    fn rekey(&mut self, new_sk: &SecretKey) {
        let current = copy_key(&self.secret_key);
        let (cpw, npw) = (pw(PASSWORD), pw(NEW_PASSWORD));
        let secrets = RekeySecrets {
            current_password: &cpw,
            current_secret_key: &current,
            new_password: &npw,
            new_secret_key: new_sk,
            profile: Argon2Profile::Baseline,
        };
        let dev = &mut self.devs[0];
        dev.engine
            .rekey_folder(&mut dev.access, &self.clock, &secrets)
            .unwrap();
        assert!(
            dev.engine
                .rekey_continue(&mut dev.access, &self.clock)
                .unwrap()
                .finished
        );
    }

    fn adopt(&mut self, d: usize, password: &str, typed: &SecretKey) -> Result<(), RekeyError> {
        let current = copy_key(&self.secret_key);
        let dev = &mut self.devs[d];
        dev.engine
            .adopt_folder_rekey(&mut dev.access, &self.clock, &pw(password), typed, &current)
            .map(|_| ())
    }

    /// Writes a well-formed v2 account file of `epoch` and `version` whose
    /// wrapping and MAC are noise (anyone with write access can).
    fn plant(&self, version: u64, epoch: u32, fill: u8) -> String {
        let device = InstallId::from_bytes([fill; 16]).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MACC\x02");
        bytes.extend_from_slice(self.account.as_bytes());
        bytes.extend_from_slice(&version.to_be_bytes());
        bytes.extend_from_slice(&epoch.to_be_bytes());
        bytes.extend_from_slice(device.as_bytes());
        bytes.extend_from_slice(&[fill; 16]);
        bytes.push(1);
        let mut wrapped = vec![1, 1];
        wrapped.extend_from_slice(&[fill; 72]);
        bytes.extend_from_slice(&wrapped);
        bytes.extend_from_slice(&[fill; 32]);
        assert_eq!(bytes.len(), 172);
        let name = account_name(version, device.as_bytes());
        std::fs::write(self.account_dir().join(&name), bytes).unwrap();
        name
    }
}

fn findings(r: &mosaic_sync::engine::SyncReport) -> Vec<(String, [u8; 32], u32)> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            SyncWarning::UnverifiedAccountFile {
                name,
                sha256,
                epoch,
            } => Some((name.clone(), *sha256, *epoch)),
            _ => None,
        })
        .collect()
}

/// A planted garbage account file naming a higher epoch is a finding that can
/// be dismissed, never a halt: only a verified marker stops a device.
#[test]
fn a_planted_garbage_account_file_is_a_finding_not_a_halt() {
    let mut w = folder_world();
    let name = w.plant(9, 5, 0x44);
    let report = w.sync(1).unwrap();
    let found = findings(&report);
    assert_eq!(found.len(), 1, "{:?}", report.warnings);
    assert_eq!(found[0].0, name);
    assert_eq!(found[0].2, 5);
    assert!(!w.devs[1].access.session().unwrap().rekey_pending().unwrap());
    // Sync keeps working, and a dismissal is remembered for good.
    let dev = &mut w.devs[1];
    dev.engine
        .dismiss_account_file(&mut dev.access, &found[0].0, &found[0].1)
        .unwrap();
    assert_eq!(findings(&w.sync(1).unwrap()).len(), 0);
    assert!(
        findings(&w.sync(0).unwrap()).len() == 1,
        "the other device decides for itself"
    );
}

/// A file that names an epoch far above the device's is never adopted, and
/// neither is a file at the next epoch whose wrapping does not open.
#[test]
fn a_crafted_higher_epoch_is_refused_not_adopted() {
    let mut w = folder_world();
    w.plant(9, 1001, 0x55);
    w.plant(10, 2, 0x56);
    let typed = SecretKey::generate().unwrap();
    assert!(matches!(
        w.adopt(1, NEW_PASSWORD, &typed),
        Err(RekeyError::NoMatchingAccountFile)
    ));
    let s = w.devs[1].access.session().unwrap();
    assert_eq!(s.ak_epoch(), 1);
}

/// Someone who holds the old password and the old Secret Key makes their own
/// re-key (epoch 2, an account key of their choosing) and plants the file. If
/// the user types the old Secret Key, the device refuses: every genuine re-key
/// rotates the Secret Key, and an old one is never accepted again.
#[test]
fn the_old_secret_key_typed_for_an_attackers_file_is_refused() {
    let mut w = folder_world();
    let params = w.devs[0]
        .access
        .session()
        .unwrap()
        .account_bootstrap()
        .unwrap()
        .params;
    let old_sk = copy_key(&w.secret_key);
    // The attacker's re-key: same password, same Secret Key, new salt.
    let attack = mosaic_core::rekey_account(
        &params,
        &MasterPassword::new(PASSWORD.to_owned()),
        &old_sk,
        Argon2Profile::Baseline,
    )
    .unwrap();
    let attacker = InstallId::from_bytes([0x66; 16]).unwrap();
    let crypto = KeyedCrypto::new(&attack.account_key, w.account, attacker)
        .unwrap()
        .with_epoch(2);
    let file = AccountFile {
        account_id: w.account,
        account_version: 5,
        ak_epoch: 2,
        device_id: attacker,
        salt: attack.params.salt,
        profile: attack.params.profile,
        wrapped_ak: attack.wrapped_account_key.clone(),
    };
    std::fs::write(
        w.account_dir().join(account_name(5, attacker.as_bytes())),
        file.encode(&crypto).unwrap(),
    )
    .unwrap();
    // The file opens with the old password and the old Secret Key and its MAC
    // is the attacker's own: the device still refuses the key.
    let err = w.adopt(1, PASSWORD, &old_sk).unwrap_err();
    assert!(
        matches!(
            err,
            RekeyError::Session(mosaic_vault::session::Error::SecretKeyUnchanged)
        ),
        "{err:?}"
    );
    assert_eq!(w.devs[1].access.session().unwrap().ak_epoch(), 1);
    // A fresh Secret Key does not open the attacker's file either.
    let other = SecretKey::generate().unwrap();
    assert!(matches!(
        w.adopt(1, PASSWORD, &other),
        Err(RekeyError::NoMatchingAccountFile)
    ));
}

/// Eight planted files above the genuine one cannot crowd it out of the cap of
/// eight: the file a verified marker names is tried first.
#[test]
fn eight_planted_files_plus_the_genuine_one_still_recovers() {
    let mut w = folder_world();
    let new_sk = SecretKey::generate().unwrap();
    w.rekey(&new_sk);
    for i in 0..8u8 {
        w.plant(100 + u64::from(i), 2, 0x70 + i);
    }
    // The marker verifies on the device of the old epoch: a hard stop, with the
    // flag persisted.
    assert!(matches!(w.sync(1), Err(SyncError::AccountRekeyed)));
    w.adopt(1, NEW_PASSWORD, &new_sk).unwrap();
    assert_eq!(w.devs[1].access.session().unwrap().ak_epoch(), 2);
    assert!(!w.devs[1].access.session().unwrap().rekey_pending().unwrap());
    w.sync(1).unwrap();
}

fn recover(
    base: &str,
    email: &str,
    new_sk: &SecretKey,
    device: InstallId,
    name: &str,
) -> hosted::RekeyRecovered {
    hosted::recover_rekey(&hosted::RecoverRequest {
        base,
        address: email,
        new_password: &pw(NEW_PASSWORD),
        new_secret_key: new_sk,
        device,
        device_name: name,
        totp_code: None,
        member_vaults: &[],
    })
    .unwrap()
}

// ---- a lying server ----------------------------------------------------------

/// After a re-key a server can still serve what it served before: a lower
/// epoch is a rollback and refused, a crafted epoch far above the device's is
/// not adopted, and the old Secret Key logs in nowhere.
#[test]
fn a_server_cannot_roll_back_or_craft_an_epoch() {
    let mut w = World::new(41, mosaic_sync::testutil::ChaosConfig::default());
    w.join();
    w.sync(0);
    w.sync(1);
    let new_sk = SecretKey::generate().unwrap();
    let old_sk = copy_key(&w.secret_key);
    let (cpw, npw) = (pw(PASSWORD), pw(NEW_PASSWORD));
    let secrets = RekeySecrets {
        current_password: &cpw,
        current_secret_key: &old_sk,
        new_password: &npw,
        new_secret_key: &new_sk,
        profile: Argon2Profile::Baseline,
    };
    w.server.advance_ms(60_000);
    let name0 = w.devs[0].name.clone();
    {
        let World {
            devs,
            clock,
            server,
            email,
            ..
        } = &mut w;
        let hosted = HostedRekey {
            base: &server.base,
            address: email,
            device_name: &name0,
            totp_code: None,
        };
        let dev = &mut devs[0];
        dev.engine
            .rekey_hosted(&mut dev.access, clock, &hosted, &secrets)
            .unwrap();
    }
    // Device 1 types the OLD Secret Key with the new password: no login.
    w.server.advance_ms(60_000);
    let (name1, device1) = (w.devs[1].name.clone(), w.devs[1].device);
    let hosted_1 = |w: &World| (w.server.base.clone(), w.email.clone());
    let (base, email) = hosted_1(&w);
    let hosted = HostedRekey {
        base: &base,
        address: &email,
        device_name: &name1,
        totp_code: None,
    };
    let refused = adopt_hosted_rekey(
        &mut w.devs[1].access,
        &hosted,
        &pw(NEW_PASSWORD),
        &old_sk,
        &old_sk,
        device1,
    );
    assert!(refused.is_err(), "the old Secret Key logs in nowhere");
    assert_eq!(w.devs[1].access.session().unwrap().ak_epoch(), 1);

    // What the server serves with the genuine keys, with a crafted epoch.
    w.server.advance_ms(60_000);
    let recovered = recover(&base, &email, &new_sk, device1, &name1);
    assert_eq!(recovered.account.ak_epoch, 2);
    let adopt_at = |w: &mut World, epoch: u32| {
        let recovered = recover(&base, &email, &new_sk, device1, &name1);
        let params = *recovered.keys.params();
        let wrapped: WrappedKey = recovered.account.wrapped_ak.clone();
        let current = copy_key(&old_sk);
        let typed = copy_key(&new_sk);
        w.devs[1]
            .access
            .session_mut()
            .unwrap()
            .adopt_remote_rekey(RekeyAdoption {
                keys: recovered.keys,
                typed_secret_key: &typed,
                current_secret_key: &current,
                wrapped_ak: &wrapped,
                params,
                epoch,
                identity: None,
                member_wraps: &[],
            })
    };
    w.server.advance_ms(60_000);
    assert!(matches!(
        adopt_at(&mut w, 1001),
        Err(mosaic_vault::session::Error::EpochNotNext { .. })
    ));
    assert!(matches!(
        adopt_at(&mut w, 1),
        Err(mosaic_vault::session::Error::UnlockFailed)
    ));
    assert_eq!(w.devs[1].access.session().unwrap().ak_epoch(), 1);
    // The genuine adoption works once, and afterwards the old epoch is a
    // rollback.
    w.server.advance_ms(60_000);
    assert!(adopt_at(&mut w, 2).is_ok());
    let s = w.devs[1].access.session().unwrap();
    assert_eq!(s.ak_epoch(), 2);
    assert!(matches!(
        s.check_backend_epoch(1),
        Err(mosaic_vault::session::Error::EpochRollback { .. })
    ));
    let _ = (
        AccountSalt::from_bytes([0; 16]),
        VaultId::from_bytes([1; 16]),
    );
}
