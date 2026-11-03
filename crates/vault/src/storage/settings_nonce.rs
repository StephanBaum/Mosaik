//! The settings nonce (CRYPTO_SPEC §11.1, ADR-0030): a random value bound
//! into the MAC of `settings.json`, replaced with every MAC'd write, so a
//! restored older file fails the check.
//!
//! It is a local-only row: no record, no outbox entry, never synced (it is
//! about this install's file, and a synced value would let the server or
//! another device roll the file back or forward).

use mosaic_core::SETTINGS_NONCE_LEN;
use rusqlite::{OptionalExtension, params};

use super::error::{Error, Result};
use super::{Store, now_ms};

impl Store {
    /// The current settings nonce, or `None` if none was written yet.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn settings_nonce(&self) -> Result<Option<[u8; SETTINGS_NONCE_LEN]>> {
        let raw: Option<Vec<u8>> = self
            .conn
            .query_row("SELECT nonce FROM settings_nonce WHERE id = 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        raw.map(|b| <[u8; SETTINGS_NONCE_LEN]>::try_from(b).map_err(|_| Error::Corrupt))
            .transpose()
    }

    /// Replaces the settings nonce (the caller draws a fresh `random(16)`
    /// for every MAC'd write of the settings file).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn replace_settings_nonce(&mut self, nonce: &[u8; SETTINGS_NONCE_LEN]) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO settings_nonce (id, nonce, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET nonce = excluded.nonce,
                                            updated_at = excluded.updated_at",
            params![nonce.as_slice(), now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_round_trips_and_is_replaced() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.settings_nonce().unwrap(), None);
        s.replace_settings_nonce(&[1; 16]).unwrap();
        assert_eq!(s.settings_nonce().unwrap(), Some([1; 16]));
        s.replace_settings_nonce(&[2; 16]).unwrap();
        assert_eq!(s.settings_nonce().unwrap(), Some([2; 16]));
    }

    #[test]
    fn the_schema_refuses_a_wrong_length_and_the_nonce_never_enters_the_outbox() {
        let mut s = Store::open_in_memory().unwrap();
        assert!(
            s.conn
                .execute(
                    "INSERT INTO settings_nonce (id, nonce, updated_at) VALUES (1, randomblob(15), 0)",
                    [],
                )
                .is_err()
        );
        s.replace_settings_nonce(&[7; 16]).unwrap();
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
