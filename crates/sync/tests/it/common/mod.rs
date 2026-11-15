//! Shared helpers: devices of one account working in their own replica of a
//! shared folder, with a simulated file-sync client between the replicas.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test helpers"
)]
#![allow(dead_code, reason = "each test module uses some of the helpers")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mosaic_core::{AccountId, Argon2Profile, MasterPassword, SecretKey, VaultId};
use mosaic_sync::engine::{EngineConfig, SyncEngine, SyncError, SyncReport};
use mosaic_sync::folder::{
    self, FolderConfig, FolderParts, FolderStore, FolderTransport, RelPath,
};
use mosaic_sync::testutil::{
    CloudSim, FixedJitter, LocalSession, ManualClock, SessionMut, SessionRef, SimConfig, SimRng,
};
use mosaic_sync::transport::{TransportError, TransportKind};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_sync::wire::folder_names::dir_name;
use mosaic_vault::session::{ItemFilter, LockedSession, TrashFilter};
use mosaic_vault::storage::Store;
use std::collections::BTreeMap;

pub const PASSWORD: &str = "correct horse battery staple";

pub fn pw(s: &str) -> MasterPassword {
    MasterPassword::new(s.to_owned())
}

/// Every item of every vault, trash included.
pub fn everything() -> ItemFilter {
    let mut filter = ItemFilter::default();
    filter.trash = TrashFilter::Include;
    filter
}

pub type Crypto = SessionCrypto<LocalSession>;

pub type Engine = SyncEngine<FolderTransport<Crypto>>;

/// Opens a folder transport for the session of `access` in `replica`. The
/// transport holds no key: it shares the session handle.
pub fn open_transport(
    access: &LocalSession,
    replica: &Path,
    clock: &ManualClock,
    name: &str,
) -> FolderTransport<Crypto> {
    let crypto = SessionCrypto::new(access.clone()).unwrap();
    let (account_id, install_id) = (
        crypto.account_id(),
        mosaic_sync::SyncCrypto::install_id(&crypto),
    );
    FolderTransport::open(FolderConfig {
        root: replica.to_owned(),
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
    .unwrap()
}

/// As [`open_transport`], over any store (a provider store, a lagging one).
pub fn open_remote_transport(
    access: &LocalSession,
    store: Box<dyn FolderStore>,
    clock: &ManualClock,
    name: &str,
) -> FolderTransport<Crypto> {
    let crypto = SessionCrypto::new(access.clone()).unwrap();
    let (account_id, install_id) = (
        crypto.account_id(),
        mosaic_sync::SyncCrypto::install_id(&crypto),
    );
    FolderTransport::open_with_store(
        store,
        FolderParts {
            account_id,
            install_id,
            crypto,
            clock: Arc::new(clock.clone()),
            device: DeviceInfo {
                created_at_ms: mosaic_sync::clock::Clock::now_ms(clock),
                name: name.to_owned(),
                platform: Platform::Linux,
            },
        },
    )
    .unwrap()
}

pub fn make_engine(
    access: &LocalSession,
    replica: &Path,
    clock: &ManualClock,
    name: &str,
) -> Engine {
    make_engine_over(open_transport(access, replica, clock, name))
}

/// The engine of the tests over a transport that is already open.
pub fn make_engine_over(transport: FolderTransport<Crypto>) -> Engine {
    SyncEngine::with_jitter(
        transport,
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    )
}

/// One device: its session, its engine over its replica of the folder.
pub struct Dev {
    pub access: LocalSession,
    pub engine: Engine,
    pub last: SyncReport,
}

impl Dev {
    pub fn s(&self) -> SessionRef<'_> {
        self.access.session().unwrap()
    }

    pub fn s_mut(&mut self) -> SessionMut<'_> {
        self.access.session_mut().unwrap()
    }
}

/// Devices of one account, a folder replicated between them, and the clock.
pub struct World {
    pub tmp: tempfile::TempDir,
    pub devs: Vec<Dev>,
    pub cloud: CloudSim,
    pub clock: ManualClock,
    pub secret_key: SecretKey,
    pub account: AccountId,
    pub rng: SimRng,
}

impl World {
    /// Device 0 creates the account and the folder in its replica; the
    /// other `replicas - 1` replicas exist but hold no device yet (see
    /// [`World::join`]). The folder is delivered to them.
    pub fn new(seed: u64, replicas: usize) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let dirs: Vec<PathBuf> = (0..replicas)
            .map(|i| {
                let p = tmp.path().join(format!("replica-{i}"));
                fs::create_dir_all(&p).unwrap();
                p
            })
            .collect();
        let (session, created) =
            LockedSession::open(Store::open(tmp.path().join("device-0.db")).unwrap())
                .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
                .unwrap();
        folder::init(&dirs[0]).unwrap();
        let clock = ManualClock::new();
        let account = session.account_id();
        let access = LocalSession::new(session);
        let mut engine = make_engine(&access, &dirs[0], &clock, "device 0");
        let boot = access.session().unwrap().account_bootstrap().unwrap();
        engine
            .transport_mut()
            .write_account_file(&boot.params, &boot.wrapped_ak, 1)
            .unwrap();
        let mut world = Self {
            devs: vec![Dev {
                access,
                engine,
                last: SyncReport::default(),
            }],
            cloud: CloudSim::new(seed, dirs),
            clock,
            secret_key: created.secret_key,
            account,
            rng: SimRng::new(seed ^ 0x5eed),
            tmp,
        };
        world.cloud.settle().unwrap();
        world
    }

    /// Replica `i`'s directory.
    pub fn replica(&self, i: usize) -> PathBuf {
        self.cloud.replica(i).to_owned()
    }

    /// A new device in replica `i = devs.len()` joins from the folder.
    pub fn try_join(&mut self, password: &str) -> Result<usize, Box<folder::JoinFailure>> {
        let i = self.devs.len();
        let store = Store::open(self.tmp.path().join(format!("device-{i}.db"))).unwrap();
        let session = folder::join_from_folder(
            self.cloud.replica(i),
            self.account,
            LockedSession::open(store),
            &pw(password),
            &self.secret_key,
        )?;
        let access = LocalSession::new(session);
        let engine = make_engine(
            &access,
            self.cloud.replica(i),
            &self.clock,
            &format!("device {i}"),
        );
        self.devs.push(Dev {
            access,
            engine,
            last: SyncReport::default(),
        });
        Ok(i)
    }

    /// Joins with the account's password.
    pub fn join(&mut self) -> usize {
        match self.try_join(PASSWORD) {
            Ok(i) => i,
            Err(f) => panic!("join failed: {:?}", f.error),
        }
    }

    pub fn try_sync(&mut self, d: usize) -> Result<SyncReport, SyncError> {
        let dev = &mut self.devs[d];
        let report = dev.engine.sync_once(&mut dev.access, &self.clock)?;
        dev.last = report.clone();
        Ok(report)
    }

    pub fn sync(&mut self, d: usize) -> SyncReport {
        self.try_sync(d)
            .unwrap_or_else(|e| panic!("device {d} failed to sync: {e:?}"))
    }

    /// Delivers everything, then every device syncs once, in random order.
    pub fn sync_all(&mut self) -> Vec<SyncReport> {
        self.cloud.settle().unwrap();
        let mut order: Vec<usize> = (0..self.devs.len()).collect();
        self.rng.shuffle(&mut order);
        let mut reports = vec![SyncReport::default(); self.devs.len()];
        for d in order {
            reports[d] = self.sync(d);
        }
        reports
    }

    /// Runs `flaky` passes with a flaky file-sync client, then passes with a
    /// calm one until two in a row change nothing.
    pub fn converge(&mut self, flaky: usize) {
        self.cloud.set_config(SimConfig::FLAKY);
        for _ in 0..flaky {
            self.cloud.round().unwrap();
            let mut order: Vec<usize> = (0..self.devs.len()).collect();
            self.rng.shuffle(&mut order);
            for d in order {
                self.sync(d);
            }
        }
        self.cloud.set_config(SimConfig::CALM);
        let mut idle_passes = 0;
        for _ in 0..40 {
            let reports = self.sync_all();
            if reports.iter().all(is_idle) {
                idle_passes += 1;
                if idle_passes == 2 {
                    return;
                }
            } else {
                idle_passes = 0;
            }
        }
        panic!("no convergence after 40 passes");
    }

    /// Advances the clock.
    pub fn advance(&self, ms: u64) {
        self.clock.advance_ms(ms);
    }

    pub fn new_vault(&mut self, d: usize, name: &str) -> VaultId {
        self.devs[d].s_mut().create_vault(name).unwrap()
    }
}

/// A run that changed nothing.
pub fn is_idle(r: &SyncReport) -> bool {
    r.pushed == 0
        && r.pulled == 0
        && r.merged == 0
        && r.attachments_up == 0
        && r.attachments_down == 0
        && r.changed_vaults.is_empty()
}

/// Every file under `root` whose name ends with `suffix`.
pub fn files_ending(root: &Path, suffix: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, suffix: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, suffix, out);
            } else if p.to_string_lossy().ends_with(suffix) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, suffix, &mut out);
    out.sort();
    out
}

/// The whole tree under `dir` as relative path to bytes.
pub fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for p in files_ending(dir, "") {
        let rel = p
            .strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        out.insert(rel, fs::read(&p).unwrap());
    }
    out
}

pub fn account_dir(w: &World, replica: usize) -> PathBuf {
    w.replica(replica)
        .join("a")
        .join(dir_name(w.account.as_bytes()))
}

pub fn log_dir(w: &World, replica: usize, writer: usize, vault: VaultId) -> PathBuf {
    account_dir(w, replica)
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("log")
        .join(dir_name(w.devs[writer].s().install_id().as_bytes()))
}

pub fn device_dir(w: &World, replica: usize, owner: usize) -> PathBuf {
    account_dir(w, replica)
        .join("d")
        .join(dir_name(w.devs[owner].s().install_id().as_bytes()))
}

pub fn segments(dir: &Path) -> Vec<PathBuf> {
    files_ending(dir, ".mseg")
}

/// What every `FolderStore` must do, below the directory `contract/` of its
/// root (which must not exist yet). `begin_run` selects whether the store is
/// used inside a run (a remote store then caches its listings).
pub fn store_contract(store: &dyn FolderStore, begin_run: bool) {
    let dir = RelPath::root().child("contract").unwrap();
    let file = |n: &str| dir.child(n).unwrap();
    if begin_run {
        store.begin_run().unwrap();
    }
    // A missing directory has no entries and no stat.
    assert!(store.list(&dir).unwrap().is_empty());
    assert!(store.stat(&dir).unwrap().is_none());
    assert_eq!(store.read_capped(&file("a.bin"), 10), Ok(None));
    // Writes create the directory; the listing is sorted.
    store.write_atomic(&dir, "b.bin", b"two", false).unwrap();
    store.write_atomic(&dir, "a.bin", b"one", false).unwrap();
    let names = |store: &dyn FolderStore| -> Vec<(String, u64)> {
        store
            .list(&dir)
            .unwrap()
            .iter()
            .map(|e| (e.name.to_string_lossy().into_owned(), e.len))
            .collect()
    };
    assert_eq!(names(store), [("a.bin".to_owned(), 3), ("b.bin".to_owned(), 3)]);
    assert!(store.stat(&dir).unwrap().unwrap().is_dir());
    // Reads: the cap is exact, a longer file is a protocol error.
    assert_eq!(store.read_capped(&file("a.bin"), 3), Ok(Some(b"one".to_vec())));
    assert_eq!(
        store.read_capped(&file("a.bin"), 2),
        Err(TransportError::Protocol)
    );
    // Write-once: the second write fails and changes nothing.
    assert_eq!(
        store.write_atomic(&dir, "a.bin", b"other", false),
        Err(TransportError::Io)
    );
    assert_eq!(store.read_capped(&file("a.bin"), 10), Ok(Some(b"one".to_vec())));
    // Replace.
    store.write_atomic(&dir, "a.bin", b"newer", true).unwrap();
    assert_eq!(store.read_capped(&file("a.bin"), 10), Ok(Some(b"newer".to_vec())));
    assert_eq!(store.stat(&file("a.bin")).unwrap().unwrap().len, 5);
    // Remove, and removing what is not there.
    store.remove(&file("b.bin")).unwrap();
    store.remove(&file("b.bin")).unwrap();
    store.remove(&file("never.bin")).unwrap();
    assert!(store.stat(&file("b.bin")).unwrap().is_none());
    assert_eq!(names(store), [("a.bin".to_owned(), 5)]);
    // No temp file is left behind.
    assert!(names(store).iter().all(|(n, _)| !n.contains(".mtmp")));
    // Nested directories.
    let nested = dir.child("x").unwrap().child("y").unwrap();
    store.create_dir_all(&nested).unwrap();
    assert!(store.is_real_dir(&nested));
    store.remove_empty_dir(&nested);
    assert!(!store.is_real_dir(&nested));
    assert!(store.is_real_dir(&dir.child("x").unwrap()));
}
