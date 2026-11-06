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
use mosaic_sync::folder::{self, FolderConfig, FolderTransport};
use mosaic_sync::testutil::{
    CloudSim, FixedJitter, LocalSession, ManualClock, SessionMut, SessionRef, SimConfig, SimRng,
};
use mosaic_sync::transport::TransportKind;
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

pub fn make_engine(
    access: &LocalSession,
    replica: &Path,
    clock: &ManualClock,
    name: &str,
) -> Engine {
    SyncEngine::with_jitter(
        open_transport(access, replica, clock, name),
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
