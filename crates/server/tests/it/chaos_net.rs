//! The harness of the hosted sync-chaos scenarios (`chaos_hosted`): a
//! transport that injects one scripted fault at an exact call (fail before,
//! lose the response after the effect, repeat the request, answer 401), runs
//! a hook before or after a call (another device acts at exactly that
//! moment, so responses arrive out of order across devices), and rewrites
//! what a hostile server answers. Plus the helpers that drive devices over it
//! and check the invariants.
//!
//! Nothing here sleeps: the engine clocks are manual clocks, the server
//! clock is the loopback server's manual clock.

#![allow(dead_code, reason = "each scenario uses some of the helpers")]
#![allow(
    clippy::many_single_char_names,
    reason = "w, d, e, a, b: world, device, engine and the two sides of a comparison"
)]

use core::time::Duration;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use mosaic_core::{AttachmentId, InstallId, ItemId, KeyGeneration, VaultId};
use mosaic_core::{VaultMetaCiphertext, WrappedKey};
use mosaic_sync::clock::Clock;
use mosaic_sync::engine::{SyncEngine, SyncError, SyncReport, SyncWarning};
use mosaic_sync::testutil::ManualClock;
use mosaic_sync::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RemoteAttachment, RemoteDevice, RemoteVault, Transport, TransportError, TransportKind,
    VaultCreateResult,
};
use mosaic_vault::session::UnlockedSession;

use crate::hosted::{LoopbackTransport, World, everything, is_idle};
use crate::scenarios::engine_for;

pub const MINUTE_MS: u64 = 60_000;
pub const DAY_MS: u64 = 86_400_000;

/// The calls of a transport, for scripting faults and hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    ListVaults,
    CreateVault,
    Pull,
    PushRecords,
    PushMeta,
    PushKey,
    PutAttachment,
    GetAttachment,
    DeleteAttachment,
    RestoreAttachment,
    Attachments,
    PutCheckpoint,
    Checkpoints,
    Devices,
    Wait,
}

/// A fault injected at one call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Fails with `Unreachable`; the server never sees the request.
    FailBefore,
    /// The server executes the request; the client gets `Timeout`.
    LoseResponse,
    /// The request is executed twice; the client gets the second answer.
    Duplicate,
    /// The client gets `Unauthorized` without the request being sent.
    Unauthorized,
}

pub type Hook = Box<dyn FnOnce() + Send>;
pub type PullTamper = Box<dyn FnMut(VaultId, &mut PullPage) + Send>;
pub type PushIntercept =
    Box<dyn FnMut(VaultId, &[PushRecord<'_>]) -> Option<Vec<PushResult>> + Send>;
pub type PushResultTamper = Box<dyn FnMut(VaultId, &mut Vec<PushResult>) + Send>;
pub type CheckpointTamper = Box<dyn FnMut(&mut Vec<(InstallId, Vec<u8>)>) + Send>;

/// See the [module documentation](self). Every call of an [`Op`] is counted
/// from 0; a fault or hook names the op and the index of the call.
pub struct Faulty<T: Transport> {
    inner: T,
    counts: BTreeMap<Op, usize>,
    pub faults: Vec<(Op, usize, Fault)>,
    /// Runs before the call is sent.
    pub before: Vec<(Op, usize, Hook)>,
    /// Runs after the server answered, before the client sees the answer.
    pub after: Vec<(Op, usize, Hook)>,
    /// Every call, in order.
    pub log: Arc<Mutex<Vec<Op>>>,
    pub tamper_pull: Option<PullTamper>,
    /// `Some(results)`: answered without sending anything to the server.
    pub intercept_push: Option<PushIntercept>,
    pub tamper_push_results: Option<PushResultTamper>,
    pub tamper_checkpoints: Option<CheckpointTamper>,
    /// Pulls of these vaults answer an empty page at the cursor they were
    /// given (the server withholds the vault).
    pub withhold: BTreeSet<VaultId>,
    /// Every pull is withheld.
    pub withhold_all: bool,
}

impl<T: Transport> Faulty<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            counts: BTreeMap::new(),
            faults: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            log: Arc::new(Mutex::new(Vec::new())),
            tamper_pull: None,
            intercept_push: None,
            tamper_push_results: None,
            tamper_checkpoints: None,
            withhold: BTreeSet::new(),
            withhold_all: false,
        }
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// How many calls of `op` were made so far.
    pub fn count(&self, op: Op) -> usize {
        self.counts.get(&op).copied().unwrap_or(0)
    }

    /// Injects `fault` at the next call of `op`.
    pub fn next(&mut self, op: Op, fault: Fault) {
        let n = self.count(op);
        self.faults.push((op, n, fault));
    }

    /// Runs `hook` before the next call of `op`.
    pub fn before_next(&mut self, op: Op, hook: impl FnOnce() + Send + 'static) {
        let n = self.count(op);
        self.before.push((op, n, Box::new(hook)));
    }

    /// Runs `hook` after the server answered the next call of `op`, before
    /// the engine sees the answer.
    pub fn after_next(&mut self, op: Op, hook: impl FnOnce() + Send + 'static) {
        let n = self.count(op);
        self.after.push((op, n, Box::new(hook)));
    }

    fn take_hook(list: &mut Vec<(Op, usize, Hook)>, op: Op, n: usize) -> Option<Hook> {
        let i = list.iter().position(|(o, k, _)| *o == op && *k == n)?;
        Some(list.remove(i).2)
    }

    fn call<R>(
        &mut self,
        op: Op,
        mut f: impl FnMut(&mut T) -> Result<R, TransportError>,
    ) -> Result<R, TransportError> {
        let n = self.count(op);
        self.counts.insert(op, n + 1);
        self.log.lock().unwrap().push(op);
        if let Some(hook) = Self::take_hook(&mut self.before, op, n) {
            hook();
        }
        let fault = self
            .faults
            .iter()
            .position(|(o, k, _)| *o == op && *k == n)
            .map(|i| self.faults.remove(i).2);
        let result = match fault {
            Some(Fault::FailBefore) => return Err(TransportError::Unreachable),
            Some(Fault::Unauthorized) => return Err(TransportError::Unauthorized),
            Some(Fault::LoseResponse) => {
                let _ = f(&mut self.inner);
                Err(TransportError::Timeout)
            }
            Some(Fault::Duplicate) => {
                let _ = f(&mut self.inner);
                f(&mut self.inner)
            }
            None => f(&mut self.inner),
        };
        if let Some(hook) = Self::take_hook(&mut self.after, op, n) {
            hook();
        }
        result
    }
}

impl<T: Transport> Transport for Faulty<T> {
    fn kind(&self) -> TransportKind {
        self.inner.kind()
    }

    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        self.call(Op::ListVaults, T::list_vaults)
    }

    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        device: InstallId,
    ) -> Result<VaultCreateResult, TransportError> {
        self.call(Op::CreateVault, |t| {
            t.create_vault(vault, generation, wrapped, meta, device)
        })
    }

    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        if self.withhold_all || self.withhold.contains(&vault) {
            return self.call(Op::Pull, |_| {
                Ok(PullPage {
                    keys: Vec::new(),
                    metas: Vec::new(),
                    records: Vec::new(),
                    next: cursor.clone(),
                    more: false,
                    units: Vec::new(),
                })
            });
        }
        let mut page = self.call(Op::Pull, |t| t.pull(vault, cursor, limit))?;
        if let Some(tamper) = &mut self.tamper_pull {
            tamper(vault, &mut page);
        }
        Ok(page)
    }

    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        if let Some(intercept) = &mut self.intercept_push
            && let Some(results) = intercept(vault, records)
        {
            let n = self.count(Op::PushRecords);
            self.counts.insert(Op::PushRecords, n + 1);
            self.log.lock().unwrap().push(Op::PushRecords);
            return Ok(results);
        }
        let mut results = self.call(Op::PushRecords, |t| {
            t.push_records(vault, generation, records)
        })?;
        if let Some(tamper) = &mut self.tamper_push_results {
            tamper(vault, &mut results);
        }
        Ok(results)
    }

    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError> {
        self.call(Op::PushMeta, |t| t.push_meta(vault, meta, base_seq))
    }

    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
        expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError> {
        self.call(Op::PushKey, |t| {
            t.push_vault_key(vault, generation, wrapped, device, expected_head)
        })
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.call(Op::PutAttachment, |t| {
            t.put_attachment(vault, id, ciphertext)
        })
    }

    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.call(Op::PutAttachment, |t| {
            t.replace_attachment(vault, id, ciphertext)
        })
    }

    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        self.call(Op::GetAttachment, |t| t.get_attachment(vault, id))
    }

    fn delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        self.call(Op::DeleteAttachment, |t| t.delete_attachment(vault, id))
    }

    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        self.call(Op::RestoreAttachment, |t| t.restore_attachment(vault, id))
    }

    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError> {
        self.call(Op::Attachments, |t| t.attachments(vault))
    }

    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        self.call(Op::PutCheckpoint, |t| t.put_checkpoint(bytes))
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        let mut list = self.call(Op::Checkpoints, T::checkpoints)?;
        if let Some(tamper) = &mut self.tamper_checkpoints {
            tamper(&mut list);
        }
        Ok(list)
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        self.call(Op::Devices, T::devices)
    }

    fn checkpoint_high_seq(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
    ) -> Result<u64, TransportError> {
        self.inner.checkpoint_high_seq(vault, cursor)
    }

    fn own_logs_rolled_back(&mut self, vaults: &[VaultId]) -> Result<bool, TransportError> {
        self.inner.own_logs_rolled_back(vaults)
    }

    fn maintain(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        self.inner.maintain(cx)
    }

    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        self.call(Op::Wait, |t| t.wait_for_change(since, timeout))
    }
}

/// An engine over the scripted transport.
pub type FEngine = SyncEngine<Faulty<LoopbackTransport>>;

/// Signs device `d` in on a session of its own and builds an engine over the
/// scripted transport. The login ends the device's other session, so the
/// device's `World` engine is unusable until `World::relogin(d)`.
pub fn faulty(w: &World, d: usize) -> FEngine {
    let (opw, device, name) = (w.opw(d), w.devs[d].device, w.devs[d].name.clone());
    engine_for(Faulty::new(w.login_transport(device, &name, &opw)))
}

/// One run of device `d` through `e`, with the shared engine clock.
pub fn frun(w: &mut World, e: &mut FEngine, d: usize) -> Result<SyncReport, SyncError> {
    let clock = w.clock.clone();
    frun_at(w, e, d, &clock)
}

/// One run of device `d` through `e` at `clock`.
pub fn frun_at(
    w: &mut World,
    e: &mut FEngine,
    d: usize,
    clock: &ManualClock,
) -> Result<SyncReport, SyncError> {
    w.server.advance_ms(MINUTE_MS);
    let result = e.sync_once(&mut w.devs[d].access, clock);
    if let Ok(report) = &result {
        w.devs[d].last = report.clone();
    }
    result
}

/// Runs device `d` through `e` until a run succeeds, waiting out the backoff
/// on the engine clock; only transient transport errors are retried.
pub fn fsync(w: &mut World, e: &mut FEngine, d: usize) -> SyncReport {
    let clock = w.clock.clone();
    fsync_at(w, e, d, &clock)
}

pub fn fsync_at(w: &mut World, e: &mut FEngine, d: usize, clock: &ManualClock) -> SyncReport {
    for _ in 0..50 {
        match frun_at(w, e, d, clock) {
            Ok(report) => return report,
            Err(SyncError::Backoff { retry_at_ms }) => {
                let now = clock.now_ms();
                clock.advance_ms(retry_at_ms.saturating_sub(now) + 1);
            }
            Err(SyncError::Transport(TransportError::Unreachable | TransportError::Timeout)) => {}
            Err(e) => panic!("device {d} failed to sync: {e:?}"),
        }
    }
    panic!("device {d} never got through");
}

/// Syncs `devices` (World engines) in the given order until two passes in a
/// row change nothing.
pub fn settle(w: &mut World, devices: &[usize]) {
    let mut idle = 0;
    for _ in 0..30 {
        let mut all_idle = true;
        for &d in devices {
            let r = w.sync(d);
            all_idle &= is_idle(&r);
        }
        if all_idle {
            idle += 1;
            if idle == 2 {
                return;
            }
        } else {
            idle = 0;
        }
    }
    panic!("no convergence of {devices:?} after 30 passes");
}

/// Every record id each device knows, per vault.
fn record_ids(s: &UnlockedSession) -> BTreeSet<(VaultId, ItemId)> {
    let mut out = BTreeSet::new();
    for v in s.vaults() {
        for r in s.store().records(&v.vault_id).unwrap() {
            out.insert((v.vault_id, r.unwrap().record_id));
        }
    }
    out
}

/// The invariants after full delivery: `devices` hold identical items,
/// tombstones, generations and vault names, nothing is queued and their
/// last reports carry no warning.
pub fn assert_converged(w: &mut World, devices: &[usize]) {
    let reference = devices[0];
    let ids: BTreeSet<(VaultId, ItemId)> = devices
        .iter()
        .flat_map(|&d| record_ids(&w.devs[d].s()))
        .collect();
    let gens = |s: &UnlockedSession| -> Vec<(VaultId, KeyGeneration, Option<String>)> {
        s.vaults()
            .iter()
            .map(|v| (v.vault_id, v.key_generation, v.name.map(str::to_owned)))
            .collect()
    };
    let ref_gens = gens(&w.devs[reference].s());
    for &d in devices {
        if d != reference {
            let a = w.devs[reference].s();
            let b = w.devs[d].s();
            for (vault, id) in &ids {
                let x = a.item(*vault, *id);
                let y = b.item(*vault, *id);
                match (x, y) {
                    (Ok(x), Ok(y)) => assert!(
                        x == y,
                        "devices {reference} and {d} disagree on {id:?}: {:?} vs {:?}",
                        x.version(),
                        y.version()
                    ),
                    (x, y) => panic!(
                        "devices {reference} and {d}: {id:?} present on one only ({} / {})",
                        x.is_ok(),
                        y.is_ok()
                    ),
                }
            }
        }
        {
            let b = w.devs[d].s();
            assert_eq!(gens(&b), ref_gens, "device {d}: vaults or generations");
            assert_eq!(b.pending_uploads(1).unwrap().len(), 0, "device {d} outbox");
            assert_eq!(b.corrupt_records().len(), 0, "device {d} corrupt");
            assert!(
                b.pending_attachment_uploads().unwrap().is_empty(),
                "device {d} attachment uploads"
            );
            assert!(
                b.pending_attachment_deletions().unwrap().is_empty(),
                "device {d} attachment deletions"
            );
            for v in b.vaults() {
                for r in b.store().records(&v.vault_id).unwrap() {
                    assert_eq!(r.unwrap().body.key_generation(), v.key_generation);
                }
            }
        }
        let pending = w.devs[d].s_mut().pending_vault_uploads().unwrap();
        assert!(pending.is_empty(), "device {d} vault uploads: {pending:?}");
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "device {d} warns: {:?}",
            w.devs[d].last.warnings
        );
    }
}

/// The password of a live item (`None` if deleted or missing).
pub fn password(w: &World, d: usize, vault: VaultId, item: ItemId) -> Option<String> {
    let s = w.devs[d].s();
    let it = s.item(vault, item).ok()?;
    if it.is_deleted() {
        return None;
    }
    it.secret(mosaic_core::model::SecretField::Password)
        .unwrap()
        .map(|p| p.expose_secret().to_owned())
}

/// Whether some live item of device `d` shows `value` as its password, a
/// conflict copy or a history entry.
pub fn anywhere(w: &World, d: usize, value: &str) -> bool {
    w.devs[d]
        .s()
        .items(&everything())
        .iter()
        .any(|r| crate::sim_hosted::has_password(r.item, value))
}

/// Whether a report carries a freeze or rollback warning.
pub fn warns_freeze(r: &SyncReport) -> bool {
    r.warnings.iter().any(|w| {
        matches!(
            w,
            SyncWarning::FreezeSuspected { .. } | SyncWarning::RollbackDetected
        )
    })
}

/// The device's stored pull cursor of `vault` as a server sequence number.
pub fn cursor_of(w: &World, d: usize, vault: VaultId) -> u64 {
    w.devs[d].s().sync_cursor(vault).unwrap().map_or(0, |c| {
        u64::from_be_bytes(<[u8; 8]>::try_from(c.as_slice()).unwrap())
    })
}

/// The server's head of `vault`, read from its database.
pub fn server_head(w: &World, vault: VaultId) -> u64 {
    let id = *vault.as_bytes();
    w.server
        .state
        .db()
        .run(|c| {
            Ok(c.query_row(
                "SELECT head_seq FROM vaults WHERE vault_id = ?1",
                [id.as_slice()],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .map(|h| u64::try_from(h).unwrap())
        .unwrap()
}

/// Deletes every session of device `d` on the server (an expiry or an
/// eviction while a run is in flight).
pub fn drop_sessions(state: &mosaic_server::AppState, device: InstallId) {
    let id = *device.as_bytes();
    state
        .db()
        .run(|c| {
            c.execute("DELETE FROM sessions WHERE device_id = ?1", [id.as_slice()])?;
            Ok(())
        })
        .unwrap();
}

/// Copies one vault's rows (head, keys, metadata, records) aside on the
/// server, for [`restore_vault`].
pub fn snapshot_vault(w: &World, vault: VaultId) {
    let id = *vault.as_bytes();
    w.server
        .state
        .db()
        .run(|c| {
            for (snap, table) in [
                ("snap_vaults", "vaults"),
                ("snap_keys", "vault_keys"),
                ("snap_meta", "vault_meta"),
                ("snap_records", "records"),
            ] {
                c.execute(&format!("DROP TABLE IF EXISTS {snap}"), [])?;
                c.execute(
                    &format!("CREATE TABLE {snap} AS SELECT * FROM {table} WHERE vault_id = ?1"),
                    [id.as_slice()],
                )?;
            }
            Ok(())
        })
        .unwrap();
}

/// Puts the vault back to the copy [`snapshot_vault`] made: a server
/// restored from a backup for this vault only (or a hostile one that rolls
/// back one vault). Other vaults and the checkpoints are untouched.
pub fn restore_vault(w: &World, vault: VaultId) {
    let id = *vault.as_bytes();
    w.server
        .state
        .db()
        .run(|c| {
            for (snap, table) in [
                ("snap_keys", "vault_keys"),
                ("snap_meta", "vault_meta"),
                ("snap_records", "records"),
            ] {
                c.execute(
                    &format!("DELETE FROM {table} WHERE vault_id = ?1"),
                    [id.as_slice()],
                )?;
                c.execute(&format!("INSERT INTO {table} SELECT * FROM {snap}"), [])?;
            }
            c.execute(
                "UPDATE vaults SET head_seq = (SELECT head_seq FROM snap_vaults),
                     current_generation = (SELECT current_generation FROM snap_vaults)
                 WHERE vault_id = ?1",
                [id.as_slice()],
            )?;
            Ok(())
        })
        .unwrap();
}
