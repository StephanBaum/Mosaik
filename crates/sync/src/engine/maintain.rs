//! Step 9: the backend's housekeeping, with the session answering through
//! short locked steps.

use mosaic_core::{AttachmentId, VaultId};
use mosaic_vault::session::{Error, UnlockedSession};
use mosaic_vault::storage::{MarkerStamp, RecordBody};

use super::{SessionAccess, SyncEngine, SyncError};
use crate::transport::{MaintainContext, Transport, TransportError};
use crate::wire::segment::SegmentObject;

/// The vault's current state as segment objects: key, metadata, items,
/// tombstones.
fn snapshot_objects(s: &UnlockedSession, vault: VaultId) -> Result<Vec<SegmentObject>, Error> {
    let generation = s
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .map(|v| v.key_generation)
        .ok_or(Error::UnknownVault)?;
    let mut objects = Vec::new();
    if let Some((_, wrapped)) = s
        .store()
        .vault_keys(&vault)?
        .into_iter()
        .find(|(g, _)| *g == generation)
    {
        objects.push(SegmentObject::VaultKey {
            generation,
            wrapped,
        });
    }
    if let Some(meta) = s.store().vault_meta(&vault)? {
        objects.push(SegmentObject::Meta(meta));
    }
    let mut tombstones = Vec::new();
    for record in s.store().records(&vault)?.into_iter().flatten() {
        if record.body.key_generation() != generation {
            continue;
        }
        match record.body {
            RecordBody::Item(ciphertext) => objects.push(SegmentObject::Item {
                record_id: record.record_id,
                ciphertext,
            }),
            RecordBody::Tombstone(ciphertext) => tombstones.push(SegmentObject::Tombstone {
                record_id: record.record_id,
                ciphertext,
            }),
        }
    }
    objects.extend(tombstones);
    Ok(objects)
}

struct Cx<'a, A: SessionAccess> {
    access: &'a mut A,
    now: u64,
    caught_up: bool,
    failure: Option<SyncError>,
}

impl<A: SessionAccess> Cx<'_, A> {
    fn step<R>(
        &mut self,
        f: impl FnOnce(&mut UnlockedSession) -> Result<R, Error>,
    ) -> Result<R, TransportError> {
        match self.access.with(f) {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => {
                self.failure = Some(SyncError::Session(e));
                Err(TransportError::Io)
            }
            Err(e) => {
                self.failure = Some(e);
                Err(TransportError::Locked)
            }
        }
    }
}

impl<A: SessionAccess> MaintainContext for Cx<'_, A> {
    fn now_ms(&self) -> u64 {
        self.now
    }

    fn snapshot_objects(&mut self, vault: VaultId) -> Result<Vec<SegmentObject>, TransportError> {
        self.step(|s| snapshot_objects(s, vault))
    }

    fn attachment_referenced(&mut self, id: AttachmentId) -> Result<bool, TransportError> {
        self.step(|s| {
            Ok(s.attachment_references()
                .iter()
                .any(|r| r.attachment_id == id))
        })
    }

    fn caught_up(&self) -> bool {
        self.caught_up
    }

    fn marker_first_seen(
        &mut self,
        vault: VaultId,
        markers: &[MarkerStamp],
        now: u64,
    ) -> Result<Vec<u64>, TransportError> {
        self.step(|s| s.marker_first_seen(vault, markers, now))
    }
}

impl<T: Transport> SyncEngine<T> {
    /// Lets the transport compact and clean up. Purging waits until no
    /// device is behind (`caught_up`), no key conflict is open and nothing
    /// was undecryptable for no known reason.
    pub(super) fn maintain(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
        caught_up: bool,
    ) -> Result<(), SyncError> {
        let mut cx = Cx {
            access,
            now,
            caught_up,
            failure: None,
        };
        let result = self.transport.maintain(&mut cx);
        if let Some(e) = cx.failure.take() {
            return Err(e);
        }
        result?;
        Ok(())
    }
}
