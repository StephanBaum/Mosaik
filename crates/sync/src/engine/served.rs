//! What the server showed of the vault metadata and the roster (M6b, R70).
//!
//! The view digest commits to the metadata and the roster as the server
//! served or accepted them: their sequence numbers and the SHA-256 of the
//! exact ciphertext. A device cannot take that from its stored copy: a merge
//! of equal content may re-seal the metadata under a new nonce. The two
//! objects are therefore noted in the served view of the store under reserved
//! record IDs (never a real item ID: they have no version nibble), next to
//! the records, and read back from there; the digest of the records leaves
//! the reserved rows out.

use mosaic_core::{ItemId, VaultId};
use mosaic_vault::session::{Error, UnlockedSession};
use mosaic_vault::storage::ServedRow;
use sha2::{Digest, Sha256};

use crate::wire::checkpoint::{ABSENT_OBJECT, ServedObject};

/// The reserved row of the vault metadata.
const META_ROW: [u8; 16] = [0xFF; 16];
/// The reserved row of the roster.
const ROSTER_ROW: [u8; 16] = [0xFE; 16];

fn is_reserved(row: &ServedRow) -> bool {
    is_reserved_id(&row.record_id)
}

/// Whether a record ID is one of the reserved rows. No real item ID is: they
/// carry a version nibble. A server that serves a record under one of them
/// is hostile (it would overwrite the noted metadata or roster of the view
/// digest, M6b review M3): the pull ignores the record.
pub(super) fn is_reserved_id(id: &ItemId) -> bool {
    let b = id.as_bytes();
    *b == META_ROW || *b == ROSTER_ROW
}

fn note(
    s: &mut UnlockedSession,
    vault: VaultId,
    row: [u8; 16],
    seq: u64,
    ciphertext: &[u8],
) -> Result<(), Error> {
    let id = ItemId::from_bytes(row).map_err(Error::Core)?;
    s.note_served(vault, id, seq, Sha256::digest(ciphertext).into())
}

/// The server served or accepted the vault metadata under `seq`.
pub(super) fn note_meta(
    s: &mut UnlockedSession,
    vault: VaultId,
    seq: u64,
    ciphertext: &[u8],
) -> Result<(), Error> {
    note(s, vault, META_ROW, seq, ciphertext)
}

/// The server served or accepted the current roster under `seq`.
pub(super) fn note_roster(
    s: &mut UnlockedSession,
    vault: VaultId,
    seq: u64,
    ciphertext: &[u8],
) -> Result<(), Error> {
    note(s, vault, ROSTER_ROW, seq, ciphertext)
}

/// The served view of the records, without the reserved rows.
pub(super) fn record_rows(rows: &[ServedRow]) -> Vec<ServedRow> {
    rows.iter().filter(|r| !is_reserved(r)).copied().collect()
}

/// The metadata and the roster as served: `None` if this device holds
/// something else than what the server showed (a change waiting for upload,
/// or an object it has no served row for), so the view is not clean.
pub(super) fn served_objects(
    s: &UnlockedSession,
    vault: VaultId,
    rows: &[ServedRow],
) -> Result<Option<(ServedObject, ServedObject)>, Error> {
    let state = s.store().vault_sync_state(&vault)?;
    if !state.remote_created || state.meta_upload_pending {
        return Ok(None);
    }
    let find = |id: [u8; 16]| {
        rows.iter()
            .find(|r| *r.record_id.as_bytes() == id)
            .map(|r| (r.seq, r.ct_hash))
    };
    let Some(meta) = find(META_ROW) else {
        return Ok(None);
    };
    let roster = match (s.store().vault_roster(&vault)?, find(ROSTER_ROW)) {
        (None, None) => ABSENT_OBJECT,
        (Some(r), Some(served)) if !r.pending => served,
        _ => return Ok(None),
    };
    Ok(Some((meta, roster)))
}
