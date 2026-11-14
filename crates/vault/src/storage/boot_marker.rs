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

    /// Proves that the store accepts a write and that the marker is there:
    /// rewrites the stored time unchanged inside a write transaction. Returns
    /// whether a marker exists.
    ///
    /// # Errors
    /// [`super::Error::Database`] when the transaction cannot be written.
    pub fn touch_boot_marker(&mut self) -> Result<bool> {
        let tx = self.write()?;
        let n = tx.execute(
            "UPDATE boot_marker SET recorded_at_ms = recorded_at_ms WHERE id = 1",
            [],
        )?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// Whether [`Self::clear_boot_marker`] has nothing left to do: no marker,
    /// or the all-`0xFF` overwrite that matches no boot.
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn boot_marker_is_neutral(&self) -> Result<bool> {
        Ok(self
            .boot_marker()?
            .is_none_or(|m| m.boot_id == [0xFF; MAX_BOOT_ID_LEN]))
    }

    /// The wrong master passwords in a row at a re-authentication (0 when
    /// none were counted). Local-only; survives a restart of the app.
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn reauth_failures(&self) -> Result<u8> {
        let n: Option<i64> = self
            .conn
            .query_row(
                "SELECT failures FROM reauth_failures WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(n.and_then(|n| u8::try_from(n).ok()).unwrap_or(0))
    }

    /// Counts one wrong re-authentication and returns the new count (at most
    /// 255).
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn note_reauth_failure(&mut self) -> Result<u8> {
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO reauth_failures (id, failures) VALUES (1, 1)
             ON CONFLICT (id) DO UPDATE SET failures = MIN(failures + 1, 255)",
            [],
        )?;
        let n: i64 = tx.query_row(
            "SELECT failures FROM reauth_failures WHERE id = 1",
            [],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(u8::try_from(n).unwrap_or(u8::MAX))
    }

    /// Ends the count (a right master password). Writes only when a count is
    /// stored.
    ///
    /// # Errors
    /// [`super::Error::Database`].
    pub fn reset_reauth_failures(&mut self) -> Result<()> {
        if self.reauth_failures()? == 0 {
            return Ok(());
        }
        let tx = self.write()?;
        tx.execute("DELETE FROM reauth_failures", [])?;
        tx.commit()?;
        Ok(())
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
    fn reauth_failures_count_cap_and_reset() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.reauth_failures().unwrap(), 0);
        for want in 1..=5u8 {
            assert_eq!(s.note_reauth_failure().unwrap(), want);
        }
        assert_eq!(s.reauth_failures().unwrap(), 5);
        // Deleting the boot marker does not reset the count.
        s.put_boot_marker(&[1], 1).unwrap();
        s.delete_boot_marker().unwrap();
        assert_eq!(s.reauth_failures().unwrap(), 5);
        s.reset_reauth_failures().unwrap();
        assert_eq!(s.reauth_failures().unwrap(), 0);
        s.reset_reauth_failures().unwrap();
        // The count stops at 255.
        for _ in 0..300 {
            s.note_reauth_failure().unwrap();
        }
        assert_eq!(s.reauth_failures().unwrap(), 255);
    }

    #[test]
    fn a_neutral_marker_is_absent_or_all_ff() {
        let mut s = Store::open_in_memory().unwrap();
        assert!(s.boot_marker_is_neutral().unwrap());
        s.put_boot_marker(&[1, 2, 3, 4], 5).unwrap();
        assert!(!s.boot_marker_is_neutral().unwrap());
        s.put_boot_marker(&[0xFF; MAX_BOOT_ID_LEN], 0).unwrap();
        assert!(s.boot_marker_is_neutral().unwrap());
    }

    #[test]
    fn touch_needs_a_marker_and_a_writable_store() {
        let mut s = Store::open_in_memory().unwrap();
        assert!(!s.touch_boot_marker().unwrap());
        s.put_boot_marker(&[1], 5).unwrap();
        assert!(s.touch_boot_marker().unwrap());
        assert_eq!(s.boot_marker().unwrap().unwrap().recorded_at_ms, 5);
        s.block_boot_marker_changes_for_test(false);
        assert!(s.touch_boot_marker().is_err());
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
