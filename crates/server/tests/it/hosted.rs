//! Shared helpers of the hosted simulations: devices of one account, each
//! with its own store, session and sync engine over the HTTP transport
//! (wrapped in a chaos transport) to one real server on `127.0.0.1`.
//!
//! `LoopbackTransport` is the HTTP transport connected to the loopback
//! server. Nothing here sleeps: the server's clock (rate limits) and the
//! engine's clock (backoff, freeze grace) are manual clocks the helpers move.

#![allow(dead_code, reason = "each test module uses some of the helpers")]

use mosaic_core::{AccountId, Argon2Profile, InstallId, MasterPassword, SecretKey, VaultId};
use mosaic_sync::engine::{EngineConfig, SyncEngine, SyncError, SyncReport};
use mosaic_sync::http::{HttpTransport, hosted};
use mosaic_sync::testutil::{
    ChaosConfig, ChaosTransport, FixedJitter, LocalSession, ManualClock, SessionMut, SessionRef,
    SimRng,
};
use mosaic_sync::transport::{Transport, TransportKind};
use mosaic_sync::wire::json::LoginStatus;
use mosaic_vault::session::{ItemFilter, LockedSession, TrashFilter};
use mosaic_vault::storage::Store;

pub use super::loopback::LoopbackServer;

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

/// The hosted transport of the simulations: HTTP to the loopback server.
pub type LoopbackTransport = HttpTransport;
/// What the devices' engines talk through.
pub type Net = ChaosTransport<LoopbackTransport>;
pub type Engine = SyncEngine<Net>;

/// Faults for the flaky phase of a run, in per mille: failures before the
/// call, lost responses after it and duplicated requests.
pub const FLAKY: ChaosConfig = ChaosConfig {
    fail_before_permille: 60,
    lost_response_permille: 60,
    duplicate_permille: 60,
};

/// One device: its session, its engine to the server.
pub struct Dev {
    pub access: LocalSession,
    pub engine: Engine,
    pub last: SyncReport,
    pub device: InstallId,
    pub name: String,
}

impl Dev {
    pub fn s(&self) -> SessionRef<'_> {
        self.access.session().unwrap()
    }

    pub fn s_mut(&mut self) -> SessionMut<'_> {
        self.access.session_mut().unwrap()
    }
}

/// Devices of one account, the server and the clock. The server is the last
/// field: the devices (and their open connections) are dropped before it
/// shuts down.
pub struct World {
    pub tmp: tempfile::TempDir,
    pub devs: Vec<Dev>,
    pub clock: ManualClock,
    pub secret_key: SecretKey,
    pub account: AccountId,
    pub email: String,
    pub rng: SimRng,
    seed: u64,
    chaos: ChaosConfig,
    pub server: LoopbackServer,
}

/// A transport to the server for the session of a login.
pub fn connect(
    server: &LoopbackServer,
    grant: &hosted::LoginGrant,
    device: InstallId,
) -> LoopbackTransport {
    assert_eq!(grant.status, LoginStatus::Ok);
    HttpTransport::connect(&server.base, &grant.token, device).unwrap()
}

impl World {
    /// Device 0 creates the account, registers it on a fresh server and
    /// logs in. `chaos` applies to every device's transport until
    /// [`World::set_chaos`].
    pub fn new(seed: u64, chaos: ChaosConfig) -> Self {
        let server = LoopbackServer::start();
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path().join("device-0.db")).unwrap();
        let device = store.install_id().unwrap();
        let (session, created) = LockedSession::open(store)
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
        let email = format!("user{seed}@example.com");
        let boot = session.account_bootstrap().unwrap();
        let opw = session
            .opaque_password(&pw(PASSWORD), &created.secret_key)
            .unwrap();
        hosted::register(&server.base, &email, None, &boot, &opw, device, "device 0").unwrap();
        let account = session.account_id();
        let mut world = Self {
            tmp,
            devs: Vec::new(),
            clock: ManualClock::new(),
            secret_key: created.secret_key,
            account,
            email,
            rng: SimRng::new(seed ^ 0x5eed),
            seed,
            chaos,
            server,
        };
        let access = LocalSession::new(session);
        let dev = world.device(access, device, "device 0", &opw);
        world.devs.push(dev);
        world
    }

    /// A device for a session that is registered on the server: logs in
    /// with `opw` and builds the engine.
    fn device(
        &mut self,
        access: LocalSession,
        device: InstallId,
        name: &str,
        opw: &mosaic_core::OpaquePassword,
    ) -> Dev {
        let transport = self.login_transport(device, name, opw);
        let engine = self.engine_over(transport, self.devs.len());
        Dev {
            access,
            engine,
            last: SyncReport::default(),
            device,
            name: name.to_owned(),
        }
    }

    /// Logs in (the server clock moves a minute first, so the per-IP and
    /// per-email login limits never trip).
    pub fn login_transport(
        &self,
        device: InstallId,
        name: &str,
        opw: &mosaic_core::OpaquePassword,
    ) -> LoopbackTransport {
        self.server.advance_ms(60_000);
        let grant = hosted::login(
            &self.server.base,
            &self.email,
            self.account,
            opw,
            device,
            name,
        )
        .unwrap();
        connect(&self.server, &grant, device)
    }

    /// Signs device `d` in again and gives it a fresh engine. A login deletes
    /// the other sessions of the same device, so a test that opened a second
    /// transport for a device calls this before it uses the device's own
    /// engine again.
    pub fn relogin(&mut self, d: usize) {
        let opw = self.opw(d);
        let (device, name) = (self.devs[d].device, self.devs[d].name.clone());
        let transport = self.login_transport(device, &name, &opw);
        self.devs[d].engine = self.engine_over(transport, d);
    }

    pub fn engine_over(&self, transport: LoopbackTransport, index: usize) -> Engine {
        SyncEngine::with_jitter(
            ChaosTransport::new(
                transport,
                self.seed ^ ((index as u64 + 1) * 0x9e37),
                self.chaos,
            ),
            EngineConfig::for_kind(TransportKind::Hosted),
            Box::new(FixedJitter(1000)),
        )
    }

    /// Faults on every device's transport.
    pub fn set_chaos(&mut self, chaos: ChaosConfig) {
        self.chaos = chaos;
        for d in &mut self.devs {
            d.engine.transport_mut().config = chaos;
        }
    }

    /// A new device joins the account over the wire (pre-login, derivation,
    /// OPAQUE login, the wrapped account key) with `password`.
    pub fn try_join_with(&mut self, password: &str) -> Result<usize, mosaic_sync::TransportError> {
        let i = self.devs.len();
        let store = Store::open(self.tmp.path().join(format!("device-{i}.db"))).unwrap();
        let device = store.install_id().unwrap();
        let name = format!("device {i}");
        self.server.advance_ms(60_000);
        let joined = hosted::join_login(
            &self.server.base,
            &self.email,
            &pw(password),
            &self.secret_key,
            device,
            &name,
        )?;
        let account =
            hosted::join_fetch_account(&self.server.base, &joined.grant.token, &joined.keys)?;
        let session = LockedSession::open(store)
            .join_account_with_keys(&account.wrapped_ak, joined.keys)
            .map_err(|e| e.error)
            .unwrap();
        let access = LocalSession::new(session);
        let transport = connect(&self.server, &joined.grant, device);
        let engine = self.engine_over(transport, i);
        self.devs.push(Dev {
            access,
            engine,
            last: SyncReport::default(),
            device,
            name,
        });
        Ok(i)
    }

    /// Joins with the account's password.
    pub fn join(&mut self) -> usize {
        self.try_join_with(PASSWORD)
            .unwrap_or_else(|e| panic!("join failed: {e:?}"))
    }

    /// One sync run that must work (no faults, or the caller handles them).
    pub fn try_sync(&mut self, d: usize) -> Result<SyncReport, SyncError> {
        // A minute of server time per run keeps the session's rate limit
        // (600 per minute) out of the way.
        self.server.advance_ms(60_000);
        let dev = &mut self.devs[d];
        let report = dev.engine.sync_once(&mut dev.access, &self.clock)?;
        dev.last = report.clone();
        Ok(report)
    }

    /// Syncs device `d`, retrying through backoff and transport faults while
    /// chaos is on; with no faults configured, any error is a failure.
    pub fn sync(&mut self, d: usize) -> SyncReport {
        let faulty = self.chaos.fail_before_permille > 0
            || self.chaos.lost_response_permille > 0
            || self.chaos.duplicate_permille > 0;
        for _ in 0..400 {
            match self.try_sync(d) {
                Ok(report) => return report,
                Err(SyncError::Backoff { retry_at_ms }) if faulty => {
                    let now = mosaic_sync::clock::Clock::now_ms(&self.clock);
                    self.clock.advance_ms(retry_at_ms.saturating_sub(now) + 1);
                }
                Err(SyncError::Transport(_)) if faulty => {}
                Err(e) => panic!("device {d} failed to sync: {e:?}"),
            }
        }
        panic!("device {d} never got through");
    }

    /// Every device syncs once, in random order.
    pub fn sync_all(&mut self) -> Vec<SyncReport> {
        let mut order: Vec<usize> = (0..self.devs.len()).collect();
        self.rng.shuffle(&mut order);
        let mut reports = vec![SyncReport::default(); self.devs.len()];
        for d in order {
            reports[d] = self.sync(d);
        }
        reports
    }

    /// Runs `flaky` passes with faults, then passes without until two in a
    /// row change nothing.
    pub fn converge(&mut self, flaky: usize) {
        let calm = ChaosConfig::default();
        let faults = self.chaos;
        for _ in 0..flaky {
            self.sync_all();
        }
        self.set_chaos(calm);
        let mut idle_passes = 0;
        for _ in 0..40 {
            let reports = self.sync_all();
            if reports.iter().all(is_idle) {
                idle_passes += 1;
                if idle_passes == 2 {
                    self.set_chaos(faults);
                    return;
                }
            } else {
                idle_passes = 0;
            }
        }
        panic!("no convergence after 40 passes");
    }

    /// Advances the engine clock.
    pub fn advance(&self, ms: u64) {
        self.clock.advance_ms(ms);
    }

    pub fn new_vault(&mut self, d: usize, name: &str) -> VaultId {
        self.devs[d].s_mut().create_vault(name).unwrap()
    }

    /// `OPW` of a device's session with the account's password.
    pub fn opw(&self, d: usize) -> mosaic_core::OpaquePassword {
        self.devs[d]
            .s()
            .opaque_password(&pw(PASSWORD), &self.secret_key)
            .unwrap()
    }
}

/// An honest server never earns a finding: no device carries a persisted
/// `ServerInconsistent` or `ServerRollback` finding, and no report says so.
pub fn assert_no_server_findings(w: &World) {
    for (d, dev) in w.devs.iter().enumerate() {
        let findings = dev.s().sync_findings().unwrap();
        assert!(
            findings.is_clear(),
            "device {d} holds a finding against an honest server: {findings:?}"
        );
        assert!(
            !dev.last.warnings.iter().any(|w| matches!(
                w,
                mosaic_sync::engine::SyncWarning::ServerInconsistent { .. }
                    | mosaic_sync::engine::SyncWarning::ServerRollback { .. }
                    | mosaic_sync::engine::SyncWarning::NoCommonView { .. }
            )),
            "device {d}: {:?}",
            dev.last.warnings
        );
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

/// Boxes a transport the way the desktop holds it.
pub fn boxed<T: Transport + 'static>(t: T) -> Box<dyn Transport> {
    Box::new(t)
}

// ---- a transport that lets a test act in the middle of a call -------------------

use core::time::Duration;
use std::sync::{Arc, Mutex};

use mosaic_core::{AttachmentId, KeyGeneration, VaultMetaCiphertext, WrappedKey};
use mosaic_sync::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RemoteAttachment, RemoteDevice, RemoteVault, TransportError, VaultCreateResult,
};
use mosaic_vault::storage::RecordKind;

/// One `push_records` call as seen by [`Intercept`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushedBatch {
    pub vault: VaultId,
    pub kinds: Vec<RecordKind>,
}

/// Wraps a transport: records every record push and runs a closure once
/// before the next record push, and another before the next vault key push
/// (to let another device act at exactly that moment).
pub struct Intercept<T: Transport> {
    inner: T,
    pub before_push_records: Option<Box<dyn FnOnce() + Send>>,
    pub before_key_push: Option<Box<dyn FnOnce() + Send>>,
    pub pushes: Arc<Mutex<Vec<PushedBatch>>>,
}

impl<T: Transport> Intercept<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            before_push_records: None,
            before_key_push: None,
            pushes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<T: Transport> Transport for Intercept<T> {
    fn kind(&self) -> TransportKind {
        self.inner.kind()
    }

    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        self.inner.list_vaults()
    }

    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        device: InstallId,
    ) -> Result<VaultCreateResult, TransportError> {
        self.inner
            .create_vault(vault, generation, wrapped, meta, device)
    }

    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        self.inner.pull(vault, cursor, limit)
    }

    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        if let Some(act) = self.before_push_records.take() {
            act();
        }
        self.pushes.lock().unwrap().push(PushedBatch {
            vault,
            kinds: records.iter().map(|r| r.body.kind()).collect(),
        });
        self.inner.push_records(vault, generation, records)
    }

    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError> {
        self.inner.push_meta(vault, meta, base_seq)
    }

    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
        expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError> {
        if let Some(act) = self.before_key_push.take() {
            act();
        }
        self.inner
            .push_vault_key(vault, generation, wrapped, device, expected_head)
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.inner.put_attachment(vault, id, ciphertext)
    }

    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        self.inner.get_attachment(vault, id)
    }

    fn delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        self.inner.delete_attachment(vault, id)
    }

    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        self.inner.restore_attachment(vault, id)
    }

    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError> {
        self.inner.attachments(vault)
    }

    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        self.inner.put_checkpoint(bytes)
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        self.inner.checkpoints()
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        self.inner.devices()
    }

    fn maintain(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        self.inner.maintain(cx)
    }

    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        self.inner.wait_for_change(since, timeout)
    }
}
