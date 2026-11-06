//! Helpers of the sync-chaos scenarios: per-device clocks, a relock (a new
//! engine and transport, so everything they keep in memory is gone), a
//! transport that fails chosen calls, and lookups into the folder.

use std::fs;
use std::path::Path;

use core::time::Duration;
use mosaic_core::model::{ItemType, SecretField, Value};
use mosaic_core::{
    AttachmentId, InstallId, Item, ItemId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey,
};
use mosaic_sync::engine::{SyncError, SyncReport, SyncWarning};
use mosaic_sync::testutil::ManualClock;
use mosaic_sync::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RemoteAttachment, RemoteDevice, RemoteVault, Transport, TransportError, TransportKind,
    VaultCreateResult,
};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::folder_names::dir_name;
use mosaic_sync::wire::segment::{Segment, SegmentPath, SnapshotInfo};

use crate::common::{World, make_engine, segments};

pub const HOUR_MS: u64 = 3_600_000;
pub const DAY_MS: u64 = 24 * HOUR_MS;

pub fn add_login(w: &mut World, d: usize, vault: VaultId, title: &str) -> ItemId {
    w.devs[d]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, "pw")
        })
        .unwrap()
}

pub fn set_password(w: &mut World, d: usize, vault: VaultId, item: ItemId, value: &str) {
    w.devs[d]
        .s_mut()
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, value))
        .unwrap();
}

/// The password an item shows, if it has one and is readable.
pub fn password_of(w: &World, d: usize, vault: VaultId, item: ItemId) -> Option<String> {
    w.devs[d]
        .s()
        .item(vault, item)
        .ok()?
        .secret(SecretField::Password)
        .ok()?
        .map(|s| s.expose_secret().to_owned())
}

/// Whether `value` is the item's password, a conflict copy of it, or in
/// its password history.
pub fn has_password(item: &Item, value: &str) -> bool {
    if item
        .secret(SecretField::Password)
        .ok()
        .flatten()
        .is_some_and(|s| s.expose_secret() == value)
    {
        return true;
    }
    if item
        .password_history()
        .iter()
        .any(|h| h.value.expose_secret() == value)
    {
        return true;
    }
    item.conflicts().iter().any(|c| {
        c.key == SecretField::Password.key()
            && c.losers
                .iter()
                .any(|e| matches!(e.value(), Value::String(t) if t.as_str() == value))
    })
}

/// Whether device `d` shows `value` for the item (value, conflict copy or
/// history) and the item is live.
pub fn shows(w: &World, d: usize, vault: VaultId, item: ItemId, value: &str) -> bool {
    w.devs[d]
        .s()
        .item(vault, item)
        .is_ok_and(|i| !i.is_deleted() && has_password(i, value))
}

/// Every password value an item holds: the shown one, its history and its
/// conflict copies.
pub fn password_values(item: &Item) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(Some(s)) = item.secret(SecretField::Password) {
        out.push(s.expose_secret().to_owned());
    }
    for h in item.password_history() {
        out.push(h.value.expose_secret().to_owned());
    }
    for c in item.conflicts() {
        if c.key == SecretField::Password.key() {
            for e in &c.losers {
                if let Value::String(t) = e.value() {
                    out.push(t.as_str().to_owned());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A relock (or an app restart) of device `d`: the folder transport holds
/// the session's keys and must be dropped when the session locks, and the
/// engine owns it, so both are built again and whatever they kept in
/// memory is gone. The store keeps everything persisted.
pub fn relock(w: &mut World, d: usize) {
    let clock = w.clock.clone();
    relock_with_clock(w, d, &clock);
}

/// A relock with a device clock of its own.
pub fn relock_with_clock(w: &mut World, d: usize, clock: &ManualClock) {
    let replica = w.replica(d);
    let engine = make_engine(&w.devs[d].access, &replica, clock, &format!("device {d}"));
    w.devs[d].engine = engine;
}

/// One sync run of device `d` with its own clock.
pub fn try_sync_with(
    w: &mut World,
    d: usize,
    clock: &ManualClock,
) -> Result<SyncReport, SyncError> {
    let dev = &mut w.devs[d];
    let report = dev.engine.sync_once(&mut dev.access, clock)?;
    dev.last = report.clone();
    Ok(report)
}

pub fn sync_with(w: &mut World, d: usize, clock: &ManualClock) -> SyncReport {
    try_sync_with(w, d, clock).unwrap_or_else(|e| panic!("device {d} failed to sync: {e:?}"))
}

pub fn warns_missing(r: &SyncReport, device: InstallId) -> bool {
    r.warnings
        .iter()
        .any(|x| matches!(x, SyncWarning::MissingFiles { device: d, .. } if *d == device))
}

/// The sequence number in a segment file's name.
pub fn seq_of(path: &Path) -> u64 {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    u64::from_str_radix(name.trim_end_matches(".mseg"), 16).unwrap()
}

/// A needle that matches exactly one segment file of `writer` in every
/// replica (the cloud simulation matches paths by substring).
pub fn segment_needle(w: &World, writer: usize, vault: VaultId, seq: u64) -> String {
    format!(
        "{}/log/{}/{seq:016x}.mseg",
        dir_name(vault.as_bytes()),
        dir_name(w.devs[writer].s().install_id().as_bytes())
    )
}

/// The snapshot parts in `writer`'s log as replica `replica` holds it:
/// `(seq, snapshot fields, record IDs in the part)`.
pub fn snapshot_parts(
    w: &World,
    replica: usize,
    writer: usize,
    vault: VaultId,
) -> Vec<(u64, SnapshotInfo, Vec<ItemId>)> {
    let keys = SessionCrypto::new(w.devs[writer].access.clone()).unwrap();
    let install = w.devs[writer].s().install_id();
    let mut out = Vec::new();
    for p in segments(&crate::common::log_dir(w, replica, writer, vault)) {
        let seq = seq_of(&p);
        let Ok(segment) = Segment::decode(
            &fs::read(&p).unwrap(),
            &SegmentPath {
                account_id: w.account,
                vault_id: vault,
                device_id: install,
                seq,
            },
            &keys,
        ) else {
            continue;
        };
        if let Some(info) = segment.snapshot {
            let ids = segment
                .objects
                .iter()
                .filter_map(|o| match o {
                    mosaic_sync::wire::segment::SegmentObject::Item { record_id, .. } => {
                        Some(*record_id)
                    }
                    _ => None,
                })
                .collect();
            out.push((seq, info, ids));
        }
    }
    out
}

/// The highest segment sequence number in a directory (0 if none).
pub fn max_seq(dir: &Path) -> u64 {
    segments(dir).iter().map(|p| seq_of(p)).max().unwrap_or(0)
}

/// Which calls of a [`Faulty`] transport fail.
#[derive(Default)]
pub struct Faults {
    /// Runs after a `push_vault_key` call took effect (a relock of the
    /// session between the backend's answer and the local commit).
    pub after_key_push: Option<Box<dyn FnMut() + Send>>,
    /// The next this many `put_checkpoint` calls fail with `Io` (nothing
    /// written).
    pub fail_put_checkpoint: u32,
    /// The `push_vault_key` call with this 1-based index fails with `Io`
    /// (nothing written).
    pub fail_push_key_call: Option<u32>,
    pub push_key_calls: u32,
    /// The next this many `push_records` calls take effect and then fail
    /// with `Timeout`: a crash between the write and the commit.
    pub lose_push_records: u32,
    /// `replace_attachment` fails with `Io` (nothing written).
    pub fail_replace_attachment: bool,
}

/// A transport that fails chosen calls (see [`Faults`]), otherwise honest.
pub struct Faulty<T: Transport> {
    pub inner: T,
    pub faults: Faults,
}

impl<T: Transport> Faulty<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            faults: Faults::default(),
        }
    }
}

impl<T: Transport> Transport for Faulty<T> {
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
        let result = self.inner.push_records(vault, generation, records);
        if result.is_ok() && self.faults.lose_push_records > 0 {
            self.faults.lose_push_records -= 1;
            return Err(TransportError::Timeout);
        }
        result
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
        self.faults.push_key_calls += 1;
        if self.faults.fail_push_key_call == Some(self.faults.push_key_calls) {
            return Err(TransportError::Io);
        }
        let result = self
            .inner
            .push_vault_key(vault, generation, wrapped, device, expected_head);
        if let Some(hook) = self.faults.after_key_push.as_mut() {
            hook();
        }
        result
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.inner.put_attachment(vault, id, ciphertext)
    }

    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        if self.faults.fail_replace_attachment {
            return Err(TransportError::Io);
        }
        self.inner.replace_attachment(vault, id, ciphertext)
    }

    fn own_logs_rolled_back(&mut self, vaults: &[VaultId]) -> Result<bool, TransportError> {
        self.inner.own_logs_rolled_back(vaults)
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
        if self.faults.fail_put_checkpoint > 0 {
            self.faults.fail_put_checkpoint -= 1;
            return Err(TransportError::Io);
        }
        self.inner.put_checkpoint(bytes)
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        self.inner.checkpoints()
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        self.inner.devices()
    }

    fn checkpoint_high_seq(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
    ) -> Result<u64, TransportError> {
        self.inner.checkpoint_high_seq(vault, cursor)
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

/// A crypto provider whose `confirm_own_seq` fails once, after a chosen
/// number of successful confirmations: a segment that was written but whose
/// confirmation failed, i.e. a write that dies half-way through a snapshot
/// set. Everything else is the session's.
pub struct FlakyCrypto {
    pub inner: mosaic_sync::wire::SessionCrypto<mosaic_sync::testutil::LocalSession>,
    /// Confirmations still allowed; the call after that fails once, then the
    /// provider is honest again (negative values never fail).
    pub allow: std::sync::Arc<std::sync::atomic::AtomicI64>,
}

impl mosaic_sync::wire::SyncCrypto for FlakyCrypto {
    fn install_id(&self) -> InstallId {
        self.inner.install_id()
    }

    fn folder_mac(
        &self,
        purpose: mosaic_core::sync_auth::FolderMacPurpose,
        msg: &[u8],
    ) -> Result<[u8; 32], mosaic_sync::WireError> {
        self.inner.folder_mac(purpose, msg)
    }

    fn verify_folder_mac(
        &self,
        purpose: mosaic_core::sync_auth::FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool, mosaic_sync::WireError> {
        self.inner.verify_folder_mac(purpose, msg, mac)
    }

    fn checkpoint_mac(
        &self,
        counter: u64,
        seen_digest: &[u8; 32],
    ) -> Result<[u8; 32], mosaic_sync::WireError> {
        self.inner.checkpoint_mac(counter, seen_digest)
    }

    fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool, mosaic_sync::WireError> {
        self.inner
            .verify_checkpoint_mac(device, counter, seen_digest, mac)
    }

    fn seal_device_info(&self, plaintext: &[u8]) -> Result<Vec<u8>, mosaic_sync::WireError> {
        self.inner.seal_device_info(plaintext)
    }

    fn open_device_info(
        &self,
        device: InstallId,
        sealed: &[u8],
    ) -> Result<mosaic_core::SecretVec, mosaic_sync::WireError> {
        self.inner.open_device_info(device, sealed)
    }
}

impl mosaic_sync::wire::OwnLogState for FlakyCrypto {
    fn own_seq(&self, vault: VaultId) -> Result<mosaic_sync::wire::OwnSeq, mosaic_sync::WireError> {
        self.inner.own_seq(vault)
    }

    fn reserve_own_seq(&self, vault: VaultId, through: u64) -> Result<(), mosaic_sync::WireError> {
        self.inner.reserve_own_seq(vault, through)
    }

    fn confirm_own_seq(&self, vault: VaultId, through: u64) -> Result<(), mosaic_sync::WireError> {
        if self.allow.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            return Err(mosaic_sync::WireError::Crypto);
        }
        self.inner.confirm_own_seq(vault, through)
    }
}
