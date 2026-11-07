//! The persisted state of the sync engine (schema v4 and v5, CRYPTO_SPEC
//! §11, ADR-0045 amendments): this install's own sequence numbers per vault,
//! the halt and the checkpoint strikes, the latest checkpoint of every other
//! device, the lag timers, the vault keys seen per generation, when a blob
//! delete marker was first seen, and (v5) the served view of every hosted
//! vault, the vault keys the server confirmed, the findings against the
//! server that stay until acknowledged and the conflict-driven rotations of
//! the last day.
//!
//! All of it is local cleartext sync metadata like the cursors: IDs,
//! counters, times and wrapped keys (ciphertext). None of it is derived from
//! item plaintext (§11).

use mosaic_core::{AttachmentId, InstallId, ItemId, KeyGeneration, VaultId, WrappedKey};
use rusqlite::{OptionalExtension, params};

use super::error::{Error, Result};
use super::{Store, seq_from_sql, seq_to_sql, vault_id_from_sql};

/// Bytes per vault in a stored peer checkpoint.
const PEER_ENTRY_LEN: usize = 16 + 4 + 8 + 32;
/// The most vaults a stored peer checkpoint holds.
const MAX_PEER_ENTRIES: usize = 4096;

/// This install's own sequence numbers in one vault's log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnSeq {
    /// The highest sequence number that may have been used: persisted before
    /// the segment is written.
    pub reserved: u64,
    /// The highest sequence number written successfully: persisted after.
    pub written: u64,
}

/// Why automatic sync stays stopped until the user acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncHalt {
    /// The backend went back in time (a checkpoint of this install or a
    /// segment of its own log disappeared).
    Rollback,
    /// Another install writes under this install's ID.
    Identity,
}

impl SyncHalt {
    const fn code(self) -> i64 {
        match self {
            Self::Rollback => 1,
            Self::Identity => 2,
        }
    }

    const fn from_code(code: i64) -> Result<Option<Self>> {
        match code {
            0 => Ok(None),
            1 => Ok(Some(Self::Rollback)),
            2 => Ok(Some(Self::Identity)),
            _ => Err(Error::Corrupt),
        }
    }
}

/// The engine's persisted halt, counter and strike state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EngineState {
    /// The halt, if sync is stopped.
    pub halt: Option<SyncHalt>,
    /// The checkpoint counter of the last checkpoint the backend accepted.
    pub last_published_counter: u64,
    /// How many times in a row this install's own checkpoint was below
    /// `last_published_counter`.
    pub strike_count: u32,
    /// When the first of those strikes happened (ms since the Unix epoch).
    pub strike_first_ms: Option<u64>,
    /// The `last_published_counter` at which the latest strike was counted:
    /// one strike per value.
    pub strike_counter: Option<u64>,
}

/// What another device's checkpoint says about one vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerEntry {
    /// The vault.
    pub vault_id: VaultId,
    /// The device's key generation of the vault.
    pub generation: KeyGeneration,
    /// The sequence number reached.
    pub high_seq: u64,
    /// The device's view digest of the vault (all zero: none, see
    /// CRYPTO_SPEC §10).
    pub view_digest: [u8; 32],
}

/// What a device found wrong with the server for one vault. The finding is
/// stored until the user acknowledges it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VaultFlag {
    /// The server showed contradicting views (devices at the same head see
    /// different contents, an upload that cannot have happened, a second key
    /// for a generation).
    ServerInconsistent,
    /// The server's head for the vault is below this device's cursor.
    ServerRolledBack,
}

impl VaultFlag {
    const fn code(self) -> i64 {
        match self {
            Self::ServerInconsistent => 1,
            Self::ServerRolledBack => 2,
        }
    }

    const fn from_code(code: i64) -> Result<Self> {
        match code {
            1 => Ok(Self::ServerInconsistent),
            2 => Ok(Self::ServerRolledBack),
            _ => Err(Error::Corrupt),
        }
    }
}

/// The server's latest version of a record as this device knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServedRow {
    /// The record.
    pub record_id: ItemId,
    /// The `server_seq` the server served or accepted it under.
    pub seq: u64,
    /// SHA-256 of the exact ciphertext bytes.
    pub ct_hash: [u8; 32],
}

/// The newest verified checkpoint of another device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerCheckpoint {
    /// The device.
    pub device_id: InstallId,
    /// Its monotonic counter.
    pub counter: u64,
    /// When it was issued (ms since the Unix epoch).
    pub issued_at_ms: u64,
    /// The entries, in the order they were published.
    pub entries: Vec<PeerEntry>,
    /// The user acknowledged a finding against the server after this
    /// checkpoint was stored ([`Store::clear_sync_halt`]): its view digests
    /// are not compared any more (they describe the server's earlier
    /// behaviour). Ignored by [`Store::put_peer_checkpoint`], which always
    /// stores `false`.
    pub digests_acknowledged: bool,
}

/// A blob delete marker as the folder shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerStamp {
    /// The attachment.
    pub attachment_id: AttachmentId,
    /// The device that wrote the marker.
    pub device_id: InstallId,
    /// The `deleted_at_ms` inside the marker. A marker whose stamp changed
    /// is a new deletion and counts as first seen now.
    pub stamp_ms: u64,
}

fn ms_to_sql(ms: u64) -> Result<i64> {
    i64::try_from(ms).map_err(|_| Error::SeqOutOfRange)
}

fn install_from_sql(bytes: &[u8]) -> Result<InstallId> {
    InstallId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

fn encode_entries(entries: &[PeerEntry]) -> Result<Vec<u8>> {
    if entries.len() > MAX_PEER_ENTRIES {
        return Err(Error::SyncDataInvalid);
    }
    let mut out = Vec::with_capacity(entries.len() * PEER_ENTRY_LEN);
    for e in entries {
        out.extend_from_slice(e.vault_id.as_bytes());
        out.extend_from_slice(&e.generation.get().to_be_bytes());
        out.extend_from_slice(&e.high_seq.to_be_bytes());
        out.extend_from_slice(&e.view_digest);
    }
    Ok(out)
}

fn decode_entries(bytes: &[u8]) -> Result<Vec<PeerEntry>> {
    bytes
        .as_chunks::<PEER_ENTRY_LEN>()
        .0
        .iter()
        .map(|c| {
            let (vault, rest) = c.split_at_checked(16).ok_or(Error::Corrupt)?;
            let (generation, rest) = rest.split_at_checked(4).ok_or(Error::Corrupt)?;
            let (high, digest) = rest.split_at_checked(8).ok_or(Error::Corrupt)?;
            let generation = u32::from_be_bytes(generation.try_into().map_err(|_| Error::Corrupt)?);
            let high = u64::from_be_bytes(high.try_into().map_err(|_| Error::Corrupt)?);
            Ok(PeerEntry {
                vault_id: vault_id_from_sql(vault)?,
                generation: KeyGeneration::new(generation).map_err(|_| Error::Corrupt)?,
                high_seq: high,
                view_digest: digest.try_into().map_err(|_| Error::Corrupt)?,
            })
        })
        .collect()
}

impl Store {
    /// This install's own sequence numbers in `vault`'s log (zeros if none).
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn own_seq(&self, vault_id: &VaultId) -> Result<OwnSeq> {
        let row: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT reserved, written FROM sync_own_seq WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map_or_else(
            || Ok(OwnSeq::default()),
            |(reserved, written)| {
                Ok(OwnSeq {
                    reserved: seq_from_sql(reserved)?,
                    written: seq_from_sql(written)?,
                })
            },
        )
    }

    /// Raises `reserved` to at least `through`. Call it before the segment
    /// with that sequence number is written, so a folder that is rolled back
    /// never makes this install reuse a number.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn reserve_own_seq(&mut self, vault_id: &VaultId, through: u64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_own_seq (vault_id, reserved) VALUES (?1, ?2)
             ON CONFLICT (vault_id) DO UPDATE SET reserved = max(reserved, excluded.reserved)",
            params![vault_id.as_bytes(), seq_to_sql(through)?],
        )?;
        Ok(())
    }

    /// Raises `written` (and `reserved`) to at least `through`: the segment
    /// with that sequence number was written.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn confirm_own_seq(&mut self, vault_id: &VaultId, through: u64) -> Result<()> {
        let through = seq_to_sql(through)?;
        self.conn.execute(
            "INSERT INTO sync_own_seq (vault_id, reserved, written) VALUES (?1, ?2, ?2)
             ON CONFLICT (vault_id) DO UPDATE SET
                 reserved = max(reserved, excluded.reserved),
                 written = max(written, excluded.written)",
            params![vault_id.as_bytes(), through],
        )?;
        Ok(())
    }

    /// The engine's halt, counter and strike state.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn engine_state(&self) -> Result<EngineState> {
        let (halt, counter, strikes, first, at): (i64, i64, i64, Option<i64>, Option<i64>) = self
            .conn
            .query_row(
                "SELECT halt, last_published_counter, strike_count, strike_first_ms,
                        strike_counter
                 FROM sync_engine_state WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?
            .ok_or(Error::Corrupt)?;
        Ok(EngineState {
            halt: SyncHalt::from_code(halt)?,
            last_published_counter: seq_from_sql(counter)?,
            strike_count: u32::try_from(strikes).map_err(|_| Error::Corrupt)?,
            strike_first_ms: first.map(seq_from_sql).transpose()?,
            strike_counter: at.map(seq_from_sql).transpose()?,
        })
    }

    /// Stops automatic sync until [`Store::clear_sync_halt`]. A halt that is
    /// already set stays (the first reason wins).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn set_sync_halt(&mut self, halt: SyncHalt) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_engine_state SET halt = ?1 WHERE id = 1 AND halt = 0",
            [halt.code()],
        )?;
        Ok(())
    }

    /// The user's acknowledgement of a halt or of a finding against the
    /// server: clears them (the vaults that were flagged are read again from
    /// the start, see below) and the strikes, and
    /// forgets what was written to the own logs (`written` becomes 0, so the
    /// rolled-back state is accepted as the new truth) while `reserved`
    /// stays, so no sequence number is ever reused.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn clear_sync_halt(&mut self) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "UPDATE sync_engine_state
             SET halt = 0, strike_count = 0, strike_first_ms = NULL, strike_counter = NULL
             WHERE id = 1",
            [],
        )?;
        tx.execute("UPDATE sync_own_seq SET written = 0", [])?;
        // The findings against the server are acknowledged: the served view
        // and the cursor of every flagged vault are rebuilt from the server
        // (the next run reads the vault again from the start), and the
        // digests the peers published so far are not compared.
        tx.execute(
            "DELETE FROM sync_served
             WHERE vault_id IN (SELECT vault_id FROM sync_vault_flag)",
            [],
        )?;
        tx.execute(
            "DELETE FROM sync_cursor_blobs
             WHERE vault_id IN (SELECT vault_id FROM sync_vault_flag)",
            [],
        )?;
        tx.execute("UPDATE sync_peer_checkpoint SET digest_ignored = 1", [])?;
        tx.execute("DELETE FROM sync_vault_flag", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Records the counter of the checkpoint the backend accepted (it never
    /// decreases).
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn set_last_published_counter(&mut self, counter: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_engine_state
             SET last_published_counter = max(last_published_counter, ?1) WHERE id = 1",
            [seq_to_sql(counter)?],
        )?;
        Ok(())
    }

    /// Stores the checkpoint strikes.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn set_checkpoint_strikes(
        &mut self,
        count: u32,
        first_ms: Option<u64>,
        at_counter: Option<u64>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_engine_state
             SET strike_count = ?1, strike_first_ms = ?2, strike_counter = ?3 WHERE id = 1",
            params![
                count,
                first_ms.map(ms_to_sql).transpose()?,
                at_counter.map(ms_to_sql).transpose()?
            ],
        )?;
        Ok(())
    }

    /// The newest verified checkpoint of every other device.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn peer_checkpoints(&self) -> Result<Vec<PeerCheckpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT device_id, counter, issued_at_ms, entries, digest_ignored
             FROM sync_peer_checkpoint ORDER BY device_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Vec<u8>>(3)?,
                r.get::<_, bool>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (device, counter, issued, entries, ignored) = row?;
            Ok(PeerCheckpoint {
                device_id: install_from_sql(&device)?,
                counter: seq_from_sql(counter)?,
                issued_at_ms: seq_from_sql(issued)?,
                entries: decode_entries(&entries)?,
                digests_acknowledged: ignored,
            })
        })
        .collect()
    }

    /// Stores `checkpoint` unless a checkpoint of the same device with an
    /// equal or higher counter is stored (a replayed older one is never
    /// accepted). Returns whether it was stored.
    ///
    /// # Errors
    /// [`Error::SyncDataInvalid`] for more than 4096 entries,
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn put_peer_checkpoint(&mut self, checkpoint: &PeerCheckpoint) -> Result<bool> {
        let entries = encode_entries(&checkpoint.entries)?;
        let changed = self.conn.execute(
            "INSERT INTO sync_peer_checkpoint (device_id, counter, issued_at_ms, entries)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (device_id) DO UPDATE SET
                 counter = excluded.counter,
                 issued_at_ms = excluded.issued_at_ms,
                 entries = excluded.entries,
                 digest_ignored = 0
             WHERE excluded.counter > sync_peer_checkpoint.counter",
            params![
                checkpoint.device_id.as_bytes(),
                seq_to_sql(checkpoint.counter)?,
                seq_to_sql(checkpoint.issued_at_ms)?,
                entries
            ],
        )?;
        Ok(changed > 0)
    }

    /// Since when each `(device, vault)` pair has been lagging.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn sync_lag(&self) -> Result<Vec<(InstallId, VaultId, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT device_id, vault_id, since_ms FROM sync_lag ORDER BY device_id, vault_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (device, vault, since) = row?;
            Ok((
                install_from_sql(&device)?,
                vault_id_from_sql(&vault)?,
                seq_from_sql(since)?,
            ))
        })
        .collect()
    }

    /// Replaces the lag timers.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn set_sync_lag(&mut self, lag: &[(InstallId, VaultId, u64)]) -> Result<()> {
        let tx = self.write()?;
        tx.execute("DELETE FROM sync_lag", [])?;
        for (device, vault, since) in lag {
            tx.execute(
                "INSERT INTO sync_lag (device_id, vault_id, since_ms) VALUES (?1, ?2, ?3)",
                params![device.as_bytes(), vault.as_bytes(), ms_to_sql(*since)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Notes that `wrapped` was seen as the key of `generation`, written by
    /// `device`. The lowest device seen for exactly these bytes is kept.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn note_key_observation(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: &InstallId,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_key_candidate (vault_id, generation, wrapped, device_id)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (vault_id, generation, wrapped) DO UPDATE SET
                 device_id = min(device_id, excluded.device_id)",
            params![
                vault_id.as_bytes(),
                generation.get(),
                wrapped.as_bytes().as_slice(),
                device.as_bytes()
            ],
        )?;
        Ok(())
    }

    /// Every key seen for `generation` with the lowest device that wrote
    /// exactly those bytes.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn key_observations(
        &self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<Vec<(InstallId, WrappedKey)>> {
        let mut stmt = self.conn.prepare(
            "SELECT device_id, wrapped FROM sync_key_candidate
             WHERE vault_id = ?1 AND generation = ?2 ORDER BY device_id, wrapped",
        )?;
        let rows = stmt.query_map(params![vault_id.as_bytes(), generation.get()], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|row| {
            let (device, wrapped) = row?;
            Ok((
                install_from_sql(&device)?,
                WrappedKey::from_bytes(&wrapped).map_err(|_| Error::Corrupt)?,
            ))
        })
        .collect()
    }

    /// Whether a key other than the held one was seen for `generation` of
    /// the vault (a concurrent rotation).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn has_key_conflict(&self, vault_id: &VaultId, generation: KeyGeneration) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sync_key_candidate c
             JOIN vault_keys k ON k.vault_id = c.vault_id AND k.generation = c.generation
             WHERE c.vault_id = ?1 AND c.generation = ?2 AND c.wrapped <> k.wrapped_vk",
            params![vault_id.as_bytes(), generation.get()],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Whether any vault has a key conflict at its current generation.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn has_any_unresolved_key_conflict(&self) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sync_key_candidate c
             JOIN vaults v ON v.vault_id = c.vault_id AND v.current_generation = c.generation
             JOIN vault_keys k ON k.vault_id = c.vault_id AND k.generation = c.generation
             WHERE c.wrapped <> k.wrapped_vk",
            [],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Forgets the keys seen for generations below the vault's current one
    /// (they are resolved).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn prune_key_observations(&mut self, vault_id: &VaultId) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sync_key_candidate
             WHERE vault_id = ?1 AND generation < COALESCE(
                 (SELECT current_generation FROM vaults WHERE vault_id = ?1), 1)",
            [vault_id.as_bytes()],
        )?;
        Ok(())
    }

    /// Overwrites the server's version of a record as this device knows it
    /// (the latest served or accepted, never a maximum).
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn put_served(
        &mut self,
        vault_id: &VaultId,
        record_id: &ItemId,
        seq: u64,
        ct_hash: &[u8; 32],
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_served (vault_id, record_id, seq, ct_hash)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (vault_id, record_id) DO UPDATE SET
                 seq = excluded.seq, ct_hash = excluded.ct_hash",
            params![
                vault_id.as_bytes(),
                record_id.as_bytes(),
                seq_to_sql(seq)?,
                ct_hash.as_slice()
            ],
        )?;
        Ok(())
    }

    /// The served version of one record.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn served(&self, vault_id: &VaultId, record_id: &ItemId) -> Result<Option<ServedRow>> {
        let row: Option<(i64, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT seq, ct_hash FROM sync_served WHERE vault_id = ?1 AND record_id = ?2",
                params![vault_id.as_bytes(), record_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(seq, hash)| {
            Ok(ServedRow {
                record_id: *record_id,
                seq: seq_from_sql(seq)?,
                ct_hash: hash.as_slice().try_into().map_err(|_| Error::Corrupt)?,
            })
        })
        .transpose()
    }

    /// Every served record of a vault, sorted by record ID.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn served_rows(&self, vault_id: &VaultId) -> Result<Vec<ServedRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT record_id, seq, ct_hash FROM sync_served
             WHERE vault_id = ?1 ORDER BY record_id",
        )?;
        let rows = stmt.query_map([vault_id.as_bytes()], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (record, seq, hash) = row?;
            Ok(ServedRow {
                record_id: ItemId::from_slice(&record).map_err(|_| Error::Corrupt)?,
                seq: seq_from_sql(seq)?,
                ct_hash: hash.as_slice().try_into().map_err(|_| Error::Corrupt)?,
            })
        })
        .collect()
    }

    /// Notes that the server confirmed this vault key (an accepted upload or
    /// the vault's creation).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_key_confirmed(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sync_key_confirmed (vault_id, generation) VALUES (?1, ?2)",
            params![vault_id.as_bytes(), generation.get()],
        )?;
        Ok(())
    }

    /// Whether the server confirmed this vault key.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn key_confirmed(&self, vault_id: &VaultId, generation: KeyGeneration) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sync_key_confirmed WHERE vault_id = ?1 AND generation = ?2",
            params![vault_id.as_bytes(), generation.get()],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Stores a finding against the server for a vault.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn set_vault_flag(&mut self, vault_id: &VaultId, flag: VaultFlag) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sync_vault_flag (vault_id, kind) VALUES (?1, ?2)",
            params![vault_id.as_bytes(), flag.code()],
        )?;
        Ok(())
    }

    /// The findings against the server, by vault.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault_flags(&self) -> Result<Vec<(VaultId, VaultFlag)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT vault_id, kind FROM sync_vault_flag ORDER BY vault_id, kind")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?)))?;
        rows.map(|row| {
            let (vault, kind) = row?;
            Ok((vault_id_from_sql(&vault)?, VaultFlag::from_code(kind)?))
        })
        .collect()
    }

    /// Records a conflict-driven rotation at `now_ms`. Rows of the vault
    /// older than `window_ms` are dropped.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn log_rotation(&mut self, vault_id: &VaultId, now_ms: u64, window_ms: u64) -> Result<()> {
        let tx = self.write()?;
        let from = ms_to_sql(now_ms.saturating_sub(window_ms))?;
        tx.execute(
            "DELETE FROM sync_rotation_log WHERE vault_id = ?1 AND at_ms <= ?2",
            params![vault_id.as_bytes(), from],
        )?;
        tx.execute(
            "INSERT INTO sync_rotation_log (vault_id, at_ms) VALUES (?1, ?2)",
            params![vault_id.as_bytes(), ms_to_sql(now_ms)?],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// How many conflict-driven rotations of the vault fall into the window
    /// that ends at `now_ms`.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn recent_rotations(&self, vault_id: &VaultId, now_ms: u64, window_ms: u64) -> Result<u32> {
        let from = ms_to_sql(now_ms.saturating_sub(window_ms))?;
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sync_rotation_log WHERE vault_id = ?1 AND at_ms > ?2",
            params![vault_id.as_bytes(), from],
            |r| r.get(0),
        )?;
        u32::try_from(n).map_err(|_| Error::Corrupt)
    }

    /// When this device first saw each marker of `vault`, by its own clock,
    /// in the order of `markers` (a marker seen for the first time, or with
    /// another stamp, is first seen `now`). Rows of markers that are gone
    /// are dropped.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Corrupt`], [`Error::Database`].
    pub fn marker_first_seen(
        &mut self,
        vault_id: &VaultId,
        markers: &[MarkerStamp],
        now: u64,
    ) -> Result<Vec<u64>> {
        let tx = self.write()?;
        let mut out = Vec::with_capacity(markers.len());
        for m in markers {
            tx.execute(
                "INSERT INTO sync_marker_seen
                     (vault_id, attachment_id, device_id, stamp_ms, first_seen_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (vault_id, attachment_id, device_id) DO UPDATE SET
                     stamp_ms = excluded.stamp_ms, first_seen_ms = excluded.first_seen_ms
                 WHERE sync_marker_seen.stamp_ms <> excluded.stamp_ms",
                params![
                    vault_id.as_bytes(),
                    m.attachment_id.as_bytes(),
                    m.device_id.as_bytes(),
                    ms_to_sql(m.stamp_ms)?,
                    ms_to_sql(now)?
                ],
            )?;
            let first: i64 = tx.query_row(
                "SELECT first_seen_ms FROM sync_marker_seen
                 WHERE vault_id = ?1 AND attachment_id = ?2 AND device_id = ?3",
                params![
                    vault_id.as_bytes(),
                    m.attachment_id.as_bytes(),
                    m.device_id.as_bytes()
                ],
                |r| r.get(0),
            )?;
            out.push(seq_from_sql(first)?);
        }
        let stored: Vec<(Vec<u8>, Vec<u8>)> = {
            let mut stmt = tx.prepare(
                "SELECT attachment_id, device_id FROM sync_marker_seen WHERE vault_id = ?1",
            )?;
            let rows = stmt.query_map([vault_id.as_bytes()], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (attachment, device) in stored {
            let kept = markers.iter().any(|m| {
                m.attachment_id.as_bytes().as_slice() == attachment.as_slice()
                    && m.device_id.as_bytes().as_slice() == device.as_slice()
            });
            if !kept {
                tx.execute(
                    "DELETE FROM sync_marker_seen
                     WHERE vault_id = ?1 AND attachment_id = ?2 AND device_id = ?3",
                    params![vault_id.as_bytes(), attachment, device],
                )?;
            }
        }
        tx.commit()?;
        Ok(out)
    }

    /// Whether this device deleted the vault itself (it keeps a generation
    /// floor for it) and no vault of that ID exists now.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn vault_deleted_locally(&self, vault_id: &VaultId) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM vault_generation_floors f
             WHERE f.vault_id = ?1
               AND NOT EXISTS (SELECT 1 FROM vaults v WHERE v.vault_id = f.vault_id)",
            [vault_id.as_bytes()],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Every vault ID with a row in `vaults` (any protection).
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn all_vault_ids(&self) -> Result<Vec<VaultId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT vault_id FROM vaults ORDER BY vault_id")?;
        let rows = stmt.query_map([], |r| r.get::<_, Vec<u8>>(0))?;
        rows.map(|row| vault_id_from_sql(&row?)).collect()
    }

    /// Clears the engine state that belongs to one backend: peer
    /// checkpoints, lag timers, key observations, marker times, strikes, the
    /// written own sequence numbers and the last published counter (the new
    /// backend has none of this install's checkpoints). The halt stays, so
    /// do the reserved own sequence numbers (the conservative choice: a
    /// folder that comes back never sees a number reused; the hole this
    /// leaves in a new folder is closed by the first compaction), and the
    /// checkpoint counter stays (it only grows).
    pub(super) fn clear_engine_state_in(tx: &rusqlite::Transaction<'_>) -> Result<()> {
        for sql in [
            // Reserved numbers stay (nothing is reused if the old folder
            // comes back); the new backend counts from zero written.
            "UPDATE sync_own_seq SET written = 0",
            "DELETE FROM sync_peer_checkpoint",
            "DELETE FROM sync_lag",
            "DELETE FROM sync_key_candidate",
            "DELETE FROM sync_marker_seen",
            "DELETE FROM sync_served",
            "DELETE FROM sync_key_confirmed",
            "DELETE FROM sync_vault_flag",
            "DELETE FROM sync_rotation_log",
            "UPDATE sync_engine_state SET strike_count = 0, strike_first_ms = NULL,
                 strike_counter = NULL, last_published_counter = 0
             WHERE id = 1",
        ] {
            tx.execute(sql, [])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> InstallId {
        InstallId::from_bytes([n; 16]).unwrap()
    }

    fn vault(n: u8) -> VaultId {
        VaultId::from_bytes([n; 16]).unwrap()
    }

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn own_seq_only_grows_and_reserve_precedes_confirm() {
        let mut s = store();
        let v = vault(1);
        assert_eq!(s.own_seq(&v).unwrap(), OwnSeq::default());
        s.reserve_own_seq(&v, 5).unwrap();
        assert_eq!(
            s.own_seq(&v).unwrap(),
            OwnSeq {
                reserved: 5,
                written: 0
            }
        );
        s.confirm_own_seq(&v, 5).unwrap();
        s.reserve_own_seq(&v, 3).unwrap();
        s.confirm_own_seq(&v, 2).unwrap();
        assert_eq!(
            s.own_seq(&v).unwrap(),
            OwnSeq {
                reserved: 5,
                written: 5
            }
        );
        // Confirming raises the reservation too.
        s.confirm_own_seq(&v, 9).unwrap();
        assert_eq!(s.own_seq(&v).unwrap().reserved, 9);
    }

    #[test]
    fn halt_strikes_and_counters_persist_and_clear() {
        let mut s = store();
        assert_eq!(s.engine_state().unwrap(), EngineState::default());
        s.set_sync_halt(SyncHalt::Rollback).unwrap();
        // The first reason stays.
        s.set_sync_halt(SyncHalt::Identity).unwrap();
        s.set_last_published_counter(4).unwrap();
        s.set_last_published_counter(2).unwrap();
        s.set_checkpoint_strikes(2, Some(77), Some(3)).unwrap();
        s.reserve_own_seq(&vault(1), 8).unwrap();
        s.confirm_own_seq(&vault(1), 8).unwrap();
        let st = s.engine_state().unwrap();
        assert_eq!(st.halt, Some(SyncHalt::Rollback));
        assert_eq!(
            (
                st.last_published_counter,
                st.strike_count,
                st.strike_first_ms,
                st.strike_counter
            ),
            (4, 2, Some(77), Some(3))
        );
        s.clear_sync_halt().unwrap();
        let st = s.engine_state().unwrap();
        assert_eq!(
            (st.halt, st.strike_count, st.strike_first_ms),
            (None, 0, None)
        );
        assert_eq!(st.last_published_counter, 4);
        // The rolled-back state is accepted, the numbers are not reused.
        assert_eq!(
            s.own_seq(&vault(1)).unwrap(),
            OwnSeq {
                reserved: 8,
                written: 0
            }
        );
    }

    #[test]
    fn peer_checkpoints_never_go_back() {
        let mut s = store();
        let entry = PeerEntry {
            vault_id: vault(3),
            generation: KeyGeneration::FIRST,
            high_seq: 12,
            view_digest: [7; 32],
        };
        let cp = |counter, high| PeerCheckpoint {
            device_id: id(9),
            counter,
            issued_at_ms: counter * 10,
            entries: vec![PeerEntry {
                high_seq: high,
                ..entry
            }],
            digests_acknowledged: false,
        };
        assert!(s.put_peer_checkpoint(&cp(5, 12)).unwrap());
        assert!(!s.put_peer_checkpoint(&cp(4, 99)).unwrap());
        assert!(!s.put_peer_checkpoint(&cp(5, 99)).unwrap());
        assert!(s.put_peer_checkpoint(&cp(6, 13)).unwrap());
        assert_eq!(s.peer_checkpoints().unwrap(), vec![cp(6, 13)]);
        let too_many = PeerCheckpoint {
            entries: vec![entry; MAX_PEER_ENTRIES + 1],
            ..cp(7, 1)
        };
        assert!(matches!(
            s.put_peer_checkpoint(&too_many),
            Err(Error::SyncDataInvalid)
        ));
    }

    #[test]
    fn lag_and_markers_round_trip() {
        let mut s = store();
        s.set_sync_lag(&[(id(2), vault(1), 100), (id(3), vault(1), 200)])
            .unwrap();
        s.set_sync_lag(&[(id(3), vault(1), 200)]).unwrap();
        assert_eq!(s.sync_lag().unwrap(), vec![(id(3), vault(1), 200)]);

        let att = AttachmentId::from_bytes([5; 16]).unwrap();
        let m = |stamp| MarkerStamp {
            attachment_id: att,
            device_id: id(2),
            stamp_ms: stamp,
        };
        let v = vault(1);
        assert_eq!(s.marker_first_seen(&v, &[m(10)], 1000).unwrap(), vec![1000]);
        // Seen again later: the first time stays.
        assert_eq!(s.marker_first_seen(&v, &[m(10)], 5000).unwrap(), vec![1000]);
        // A new deletion (another stamp) counts as new.
        assert_eq!(s.marker_first_seen(&v, &[m(20)], 6000).unwrap(), vec![6000]);
        // A marker that vanished is forgotten.
        assert_eq!(
            s.marker_first_seen(&v, &[], 7000).unwrap(),
            Vec::<u64>::new()
        );
        assert_eq!(s.marker_first_seen(&v, &[m(20)], 8000).unwrap(), vec![8000]);
    }

    fn record(n: u8) -> ItemId {
        ItemId::from_bytes([n; 16]).unwrap()
    }

    #[test]
    fn the_served_view_is_overwritten_never_maximised() {
        let mut s = store();
        let v = vault(1);
        assert_eq!(s.served(&v, &record(9)).unwrap(), None);
        s.put_served(&v, &record(9), 50, &[1; 32]).unwrap();
        s.put_served(&v, &record(2), 7, &[2; 32]).unwrap();
        // A lower sequence number replaces a higher one: the server's last
        // answer wins.
        s.put_served(&v, &record(9), 8, &[3; 32]).unwrap();
        assert_eq!(
            s.served(&v, &record(9)).unwrap(),
            Some(ServedRow {
                record_id: record(9),
                seq: 8,
                ct_hash: [3; 32]
            })
        );
        // Sorted by record ID; other vaults are separate.
        s.put_served(&vault(2), &record(1), 1, &[4; 32]).unwrap();
        let rows = s.served_rows(&v).unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| (r.record_id, r.seq))
                .collect::<Vec<_>>(),
            vec![(record(2), 7), (record(9), 8)]
        );
    }

    #[test]
    fn confirmed_keys_flags_and_the_rotation_window() {
        let mut s = store();
        let v = vault(1);
        let g2 = KeyGeneration::new(2).unwrap();
        assert!(!s.key_confirmed(&v, g2).unwrap());
        s.mark_key_confirmed(&v, g2).unwrap();
        s.mark_key_confirmed(&v, g2).unwrap();
        assert!(s.key_confirmed(&v, g2).unwrap());
        assert!(!s.key_confirmed(&v, KeyGeneration::FIRST).unwrap());

        assert_eq!(s.vault_flags().unwrap(), vec![]);
        s.set_vault_flag(&v, VaultFlag::ServerRolledBack).unwrap();
        s.set_vault_flag(&v, VaultFlag::ServerInconsistent).unwrap();
        s.set_vault_flag(&v, VaultFlag::ServerInconsistent).unwrap();
        assert_eq!(
            s.vault_flags().unwrap(),
            vec![
                (v, VaultFlag::ServerInconsistent),
                (v, VaultFlag::ServerRolledBack)
            ]
        );

        let day = 86_400_000;
        // Two rotations in one millisecond count twice.
        for at in [1_000, 1_000, 2_000, 3_000] {
            s.log_rotation(&v, at, day).unwrap();
        }
        assert_eq!(s.recent_rotations(&v, 3_000, day).unwrap(), 4);
        // A day later the first ones fall out of the window and are dropped.
        s.log_rotation(&v, 2_500 + day, day).unwrap();
        assert_eq!(s.recent_rotations(&v, 2_500 + day, day).unwrap(), 2);
        assert_eq!(s.recent_rotations(&vault(2), 3_000, day).unwrap(), 0);
    }

    #[test]
    fn clearing_the_halt_forgets_the_findings_and_old_digests() {
        let mut s = store();
        let v = vault(1);
        s.set_vault_flag(&v, VaultFlag::ServerInconsistent).unwrap();
        s.put_served(&v, &record(1), 3, &[1; 32]).unwrap();
        s.put_served(&vault(2), &record(1), 3, &[1; 32]).unwrap();
        let cp = PeerCheckpoint {
            device_id: id(9),
            counter: 1,
            issued_at_ms: 1,
            entries: vec![],
            digests_acknowledged: false,
        };
        assert!(s.put_peer_checkpoint(&cp).unwrap());
        s.clear_sync_halt().unwrap();
        assert_eq!(s.vault_flags().unwrap(), vec![]);
        // The flagged vault's view is rebuilt; an unflagged one stays.
        assert_eq!(s.served_rows(&v).unwrap(), vec![]);
        assert_eq!(s.served_rows(&vault(2)).unwrap().len(), 1);
        // The checkpoint that arrived before the acknowledgement is not
        // compared any more; a newer one is.
        assert!(s.peer_checkpoints().unwrap()[0].digests_acknowledged);
        assert!(
            s.put_peer_checkpoint(&PeerCheckpoint { counter: 2, ..cp })
                .unwrap()
        );
        assert!(!s.peer_checkpoints().unwrap()[0].digests_acknowledged);
    }
}
