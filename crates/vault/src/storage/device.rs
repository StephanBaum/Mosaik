//! This install's device ID (CRYPTO_SPEC §10).

use mosaic_core::DeviceId;
use rusqlite::{OptionalExtension, params};

use super::error::{Error, Result};
use super::{Store, now_ms};

impl Store {
    /// The device ID of this install, or `None` before
    /// [`Store::set_device_id`].
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn device_id(&self) -> Result<Option<DeviceId>> {
        let raw: Option<Vec<u8>> = self
            .conn
            .query_row("SELECT device_id FROM local_device WHERE id = 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        raw.map(|b| DeviceId::from_slice(&b).map_err(|_| Error::Corrupt))
            .transpose()
    }

    /// Sets (or replaces) the device ID of this install. Replace it only
    /// when it turned out to be shared with another writer
    /// (`mosaic_core::Error::DeviceIdReused`, CRYPTO_SPEC §10): a new ID
    /// starts a fresh counter, so it never collides with the edits made
    /// under the old one.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn set_device_id(&mut self, device_id: &DeviceId) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO local_device (id, device_id, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET device_id = excluded.device_id,
                                            updated_at = excluded.updated_at",
            params![device_id.as_bytes(), now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_round_trips_and_can_be_replaced() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.device_id().unwrap(), None);
        let a = DeviceId::new_v7().unwrap();
        s.set_device_id(&a).unwrap();
        assert_eq!(s.device_id().unwrap(), Some(a));
        let b = DeviceId::new_v7().unwrap();
        s.set_device_id(&b).unwrap();
        assert_eq!(s.device_id().unwrap(), Some(b));
    }

    /// The schema's `CHECK` refuses the statement (not some other SQL
    /// error).
    fn is_check_violation(result: &rusqlite::Result<usize>) -> bool {
        matches!(
            result,
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation
        )
    }

    #[test]
    fn schema_refuses_a_nil_or_wrong_length_device_id() {
        let s = Store::open_in_memory().unwrap();
        s.conn
            .execute(
                "INSERT INTO local_device (id, device_id, updated_at) VALUES (1, ?1, 0)",
                [[0xFFu8; 16]],
            )
            .unwrap();
        // A non-UUIDv7-shaped but non-nil ID still decodes; only the nil ID
        // and wrong lengths are impossible, and the schema rejects them.
        assert!(s.device_id().unwrap().is_some());
        for bad in [
            "zeroblob(16)",
            "zeroblob(15)",
            "randomblob(15)",
            "randomblob(17)",
        ] {
            assert!(
                is_check_violation(&s.conn.execute(
                    &format!("UPDATE local_device SET device_id = {bad} WHERE id = 1"),
                    [],
                )),
                "{bad}"
            );
        }
        assert!(s.device_id().unwrap().is_some(), "the row is unchanged");
    }

    #[test]
    fn a_wrong_length_row_reads_as_corrupt() {
        // Written past the schema's check, as an outside writer could.
        let s = Store::open_in_memory().unwrap();
        s.conn
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        s.conn
            .execute(
                "INSERT INTO local_device (id, device_id, updated_at) VALUES (1, ?1, 0)",
                [[0xABu8; 15]],
            )
            .unwrap();
        assert!(matches!(s.device_id(), Err(Error::Corrupt)));
    }
}
