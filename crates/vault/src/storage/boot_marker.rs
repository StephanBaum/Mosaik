//! The boot marker (CRYPTO_SPEC §6.9.2, ADR-0090): on a phone, the boot ID of
//! the device boot in which the master password last unlocked.
//!
//! A local-only row, never synced. It is cleartext, not secret and not derived
//! from a secret. Only `mosaic-mobile` writes it; the desktop and the CLI never
//! do. It is deleted in the same transaction as every deletion of the
//! quick-unlock wrapping (the callers below call [`delete_in`]).

use rusqlite::{OptionalExtension, Transaction, params};

use super::Store;
use super::error::Result;

/// The longest boot ID, in bytes.
pub const MAX_BOOT_ID_LEN: usize = 32;

/// The stored marker.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StoredBootMarker {
    /// The boot ID, 1 to [`MAX_BOOT_ID_LEN`] bytes.
    pub boot_id: Vec<u8>,
    /// When it was written (ms since the Unix epoch).
    pub recorded_at_ms: i64,
}

/// Deletes the marker inside a transaction that deletes a quick-unlock
/// wrapping (or the whole unlock set).
pub(super) fn delete_in(tx: &Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM boot_marker", [])?;
    Ok(())
}

impl Store {
    /// The stored boot marker, if any.
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn boot_marker(&self) -> Result<Option<StoredBootMarker>> {
        Ok(self
            .conn
            .query_row(
                "SELECT boot_id, recorded_at_ms FROM boot_marker WHERE id = 1",
                [],
                |r| {
                    Ok(StoredBootMarker {
                        boot_id: r.get(0)?,
                        recorded_at_ms: r.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    /// Writes the marker, replacing any earlier one.
    ///
    /// # Errors
    /// [`super::Error::Database`] (also for a boot ID outside 1 to 32 bytes:
    /// the schema refuses it).
    pub fn put_boot_marker(&mut self, boot_id: &[u8], recorded_at_ms: i64) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET boot_id = excluded.boot_id,
                                            recorded_at_ms = excluded.recorded_at_ms",
            params![boot_id, recorded_at_ms],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Deletes the marker. Returns whether there was one.
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn delete_boot_marker(&mut self) -> Result<bool> {
        let tx = self.write()?;
        let n = tx.execute("DELETE FROM boot_marker", [])?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// Makes sure no stored marker can match a boot: deletes it, and when the
    /// delete fails, overwrites it with an ID no platform produces (32 bytes,
    /// both platforms use 4 or 16).
    ///
    /// # Errors
    /// The delete error, when the marker could be neither deleted nor
    /// overwritten (a store that cannot be written).
    pub fn clear_boot_marker(&mut self) -> Result<()> {
        match self.delete_boot_marker() {
            Ok(_) => Ok(()),
            Err(e) => self
                .put_boot_marker(&[0xFF; MAX_BOOT_ID_LEN], 0)
                .map_err(|_| e),
        }
    }

    /// Test only: makes the marker undeletable (`delete_only`) or unchangeable.
    #[cfg(test)]
    pub(crate) fn block_boot_marker_changes_for_test(&mut self, delete_only: bool) {
        self.conn
            .execute_batch(
                "CREATE TRIGGER boot_marker_no_delete BEFORE DELETE ON boot_marker                  BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
            )
            .unwrap();
        if !delete_only {
            self.conn
                .execute_batch(
                    "CREATE TRIGGER boot_marker_no_update BEFORE UPDATE ON boot_marker                      BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
                )
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_round_trips_is_replaced_and_deleted() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.boot_marker().unwrap(), None);
        assert!(!s.delete_boot_marker().unwrap());
        s.put_boot_marker(&[1, 2, 3], 10).unwrap();
        assert_eq!(
            s.boot_marker().unwrap(),
            Some(StoredBootMarker {
                boot_id: vec![1, 2, 3],
                recorded_at_ms: 10
            })
        );
        s.put_boot_marker(&[9; 32], 20).unwrap();
        assert_eq!(s.boot_marker().unwrap().unwrap().boot_id, vec![9; 32]);
        assert!(s.put_boot_marker(&[], 0).is_err());
        assert!(s.put_boot_marker(&[0; 33], 0).is_err());
        assert!(s.delete_boot_marker().unwrap());
        assert_eq!(s.boot_marker().unwrap(), None);
    }

    #[test]
    fn the_marker_never_enters_the_outbox() {
        let mut s = Store::open_in_memory().unwrap();
        s.put_boot_marker(&[1], 1).unwrap();
        let pending: i64 = s
            .conn
            .query_row(
                "SELECT count(*) FROM records WHERE outbox_seq IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0);
    }
}
