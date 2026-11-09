//! A transport wrapper that fails the way networks and sync clients do:
//! calls that fail before they have an effect, responses lost after the
//! effect, and duplicated requests.

use core::time::Duration;

use mosaic_core::{
    AttachmentId, InstallId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey,
};

use super::SimRng;
use crate::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RekeyCandidate, RekeyPublish, RekeySignals, RemoteAttachment, RemoteDevice, RemoteVault,
    Transport, TransportError, TransportKind, VaultCreateResult,
};

/// The faults of a [`ChaosTransport`], in per mille per call.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChaosConfig {
    /// The call fails with `Unreachable` and has no effect.
    pub fail_before_permille: u32,
    /// A mutating call takes effect, then fails with `Timeout`: the client
    /// never sees the answer.
    pub lost_response_permille: u32,
    /// A mutating call is executed twice.
    pub duplicate_permille: u32,
}

/// See the [module documentation](self).
pub struct ChaosTransport<T: Transport> {
    inner: T,
    rng: SimRng,
    /// The faults; all zero means a plain pass-through.
    pub config: ChaosConfig,
    /// Counts of the faults injected so far: before, lost, duplicated.
    pub injected: (u32, u32, u32),
}

impl<T: Transport> ChaosTransport<T> {
    /// Wraps `inner` with faults drawn from `seed`.
    pub const fn new(inner: T, seed: u64, config: ChaosConfig) -> Self {
        Self {
            inner,
            rng: SimRng::new(seed),
            config,
            injected: (0, 0, 0),
        }
    }

    /// The wrapped transport.
    pub const fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Gives the wrapped transport back.
    pub fn into_inner(self) -> T {
        self.inner
    }

    fn call<R>(
        &mut self,
        mutating: bool,
        mut f: impl FnMut(&mut T) -> Result<R, TransportError>,
    ) -> Result<R, TransportError> {
        if self.rng.chance(self.config.fail_before_permille) {
            self.injected.0 += 1;
            return Err(TransportError::Unreachable);
        }
        let first = f(&mut self.inner);
        if !mutating {
            return first;
        }
        if self.rng.chance(self.config.duplicate_permille) {
            self.injected.2 += 1;
            let second = f(&mut self.inner);
            if self.rng.chance(self.config.lost_response_permille) {
                self.injected.1 += 1;
                return Err(TransportError::Timeout);
            }
            return second;
        }
        if self.rng.chance(self.config.lost_response_permille) {
            self.injected.1 += 1;
            return Err(TransportError::Timeout);
        }
        first
    }
}

impl<T: Transport> Transport for ChaosTransport<T> {
    fn kind(&self) -> TransportKind {
        self.inner.kind()
    }

    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        self.call(false, T::list_vaults)
    }

    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        device: InstallId,
    ) -> Result<VaultCreateResult, TransportError> {
        self.call(true, |t| {
            t.create_vault(vault, generation, wrapped, meta, device)
        })
    }

    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        self.call(false, |t| t.pull(vault, cursor, limit))
    }

    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        self.call(true, |t| t.push_records(vault, generation, records))
    }

    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError> {
        self.call(true, |t| t.push_meta(vault, meta, base_seq))
    }

    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
        expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError> {
        self.call(true, |t| {
            t.push_vault_key(vault, generation, wrapped, device, expected_head)
        })
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.call(true, |t| t.put_attachment(vault, id, ciphertext))
    }

    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.inner.replace_attachment(vault, id, ciphertext)
    }

    fn rekey_signals(&mut self, epoch: u32) -> Result<RekeySignals, TransportError> {
        self.inner.rekey_signals(epoch)
    }

    fn backend_epoch(&mut self) -> Result<Option<u32>, TransportError> {
        self.inner.backend_epoch()
    }

    fn rekey_candidates(
        &mut self,
        epoch: u32,
        limit: usize,
    ) -> Result<Vec<RekeyCandidate>, TransportError> {
        self.inner.rekey_candidates(epoch, limit)
    }

    fn next_folder_account_version(&mut self) -> Result<Option<u64>, TransportError> {
        self.inner.next_folder_account_version()
    }

    fn publish_rekey(
        &mut self,
        cx: &mut dyn MaintainContext,
        publish: &RekeyPublish<'_>,
    ) -> Result<(), TransportError> {
        self.inner.publish_rekey(cx, publish)
    }

    fn own_logs_rolled_back(&mut self, vaults: &[VaultId]) -> Result<bool, TransportError> {
        self.inner.own_logs_rolled_back(vaults)
    }

    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        self.call(false, |t| t.get_attachment(vault, id))
    }

    fn delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        self.call(true, |t| t.delete_attachment(vault, id))
    }

    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        self.call(true, |t| t.restore_attachment(vault, id))
    }

    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError> {
        self.call(false, |t| t.attachments(vault))
    }

    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        self.call(true, |t| t.put_checkpoint(bytes))
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        self.call(false, T::checkpoints)
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        self.call(false, T::devices)
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

    fn sharing(&mut self) -> Option<&mut dyn crate::sharing::SharingTransport> {
        self.inner.sharing()
    }
}
