//! A transport wrapper that lies the way a hostile backend (or a hostile
//! file-sync account) can: it hides checkpoints, serves old or forged ones,
//! withholds everything, and rewrites what a pull returns.

use core::time::Duration;
use std::collections::BTreeSet;

use mosaic_core::{
    AttachmentId, InstallId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey,
};

use crate::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RemoteAttachment, RemoteDevice, RemoteVault, Transport, TransportError, TransportKind,
    VaultCreateResult,
};

/// A function that rewrites a pulled page.
pub type PageTamper = Box<dyn FnMut(&mut PullPage) + Send>;

/// A function that rewrites a pulled page of a given vault.
pub type VaultPageTamper = Box<dyn FnMut(VaultId, &mut PullPage) + Send>;

/// See the [module documentation](self).
pub struct MaliciousTransport<T: Transport> {
    inner: T,
    /// Checkpoints of these devices are not served.
    pub hide_checkpoints_of: BTreeSet<InstallId>,
    /// These checkpoints are served in place of the real ones of the same
    /// device (a replay of old bytes) or in addition (a forgery).
    pub replace_checkpoints: Vec<(InstallId, Vec<u8>)>,
    /// Every pull answers an empty page at the cursor it was given.
    pub withhold_pulls: bool,
    /// Rewrites every pulled page.
    pub tamper_pull: Option<PageTamper>,
    /// Like `tamper_pull`, and told which vault the page is of.
    pub tamper_pull_of: Option<VaultPageTamper>,
}

impl<T: Transport> MaliciousTransport<T> {
    /// Wraps `inner`; it behaves honestly until a field is set.
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            hide_checkpoints_of: BTreeSet::new(),
            replace_checkpoints: Vec::new(),
            withhold_pulls: false,
            tamper_pull: None,
            tamper_pull_of: None,
        }
    }

    /// The wrapped transport.
    pub const fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Transport> Transport for MaliciousTransport<T> {
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
        if self.withhold_pulls {
            return Ok(PullPage {
                next: cursor.clone(),
                ..PullPage::default()
            });
        }
        let mut page = self.inner.pull(vault, cursor, limit)?;
        if let Some(tamper) = &mut self.tamper_pull {
            tamper(&mut page);
        }
        if let Some(tamper) = &mut self.tamper_pull_of {
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

    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
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
        self.inner.put_checkpoint(bytes)
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        let replaced: BTreeSet<InstallId> =
            self.replace_checkpoints.iter().map(|(d, _)| *d).collect();
        let mut out: Vec<(InstallId, Vec<u8>)> = self
            .inner
            .checkpoints()?
            .into_iter()
            .filter(|(d, _)| !self.hide_checkpoints_of.contains(d) && !replaced.contains(d))
            .collect();
        out.extend(self.replace_checkpoints.iter().cloned());
        Ok(out)
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

    fn sharing(&mut self) -> Option<&mut dyn crate::sharing::SharingTransport> {
        self.inner.sharing()
    }
}
