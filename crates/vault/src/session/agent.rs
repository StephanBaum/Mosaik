//! Agent access state (CRYPTO_SPEC §11, ADR-0160): device marks that confirm an item's agent
//! binding on this device, and the encrypted agent log.
//!
//! `K_agent_mark` and `K_agent_log` are derived from the account key for each call and dropped at
//! its end. Neither is stored. The log is local only: never synced, never in the outbox.

use mosaic_core::{ItemId, SecretVec, VaultId, new_agent_log_entry_id};

use super::{Result, UnlockedSession};
use crate::storage::{AgentLogCaps, AgentLogRow};

/// The log keeps at most this many entries (the newest).
pub const AGENT_LOG_MAX_ENTRIES: usize = 10_000;
/// Refusal rows (a flood of bad requests) keep this many, apart from the 10,000 of every other
/// kind: they can never push a grant or pairing entry out.
pub const AGENT_LOG_MAX_NOISY: usize = 2_000;
/// The log drops entries older than this many milliseconds (90 days).
pub const AGENT_LOG_MAX_AGE_MS: u64 = 90 * 24 * 3600 * 1000;

/// One page of the agent log.
#[derive(Debug)]
pub struct AgentLogPage {
    /// `(at_ms, plaintext)`, newest first.
    pub entries: Vec<(u64, SecretVec)>,
    /// Rows that did not open (another account key after a re-key, or a damaged row), skipped.
    pub unreadable: usize,
}

impl UnlockedSession {
    /// Whether the user confirmed this binding of the item on this device: a stored mark matches
    /// `HMAC(K_agent_mark, item_id || binding_id || canonical)`.
    ///
    /// # Errors
    /// [`super::Error::Core`], [`super::Error::Store`].
    pub fn agent_mark_confirmed(
        &self,
        vault: VaultId,
        item: ItemId,
        binding_id: &str,
        canonical: &[u8],
    ) -> Result<bool> {
        let mark = self
            .account_key
            .agent_mark_key()?
            .mark(&item, binding_id, canonical)?;
        Ok(self.store.agent_mark_exists(&vault, &item, &mark)?)
    }

    /// Confirms the binding on this device (saving it in this device's editor, or the user's
    /// review of a synced one).
    ///
    /// # Errors
    /// [`super::Error::UnknownVault`], [`super::Error::UnknownItem`],
    /// [`super::Error::ItemDeleted`], [`super::Error::Core`], [`super::Error::Store`].
    pub fn confirm_agent_mark(
        &mut self,
        vault: VaultId,
        item: ItemId,
        binding_id: &str,
        canonical: &[u8],
    ) -> Result<()> {
        self.vault(vault)?.live_item(item)?;
        let mark = self
            .account_key
            .agent_mark_key()?
            .mark(&item, binding_id, canonical)?;
        self.store.put_agent_mark(&vault, &item, &mark)?;
        Ok(())
    }

    /// Drops the item's marks that match none of `current` (`(binding_id, canonical)` pairs): a
    /// binding that was removed or changed no longer counts as confirmed.
    ///
    /// # Errors
    /// [`super::Error::Core`], [`super::Error::Store`].
    pub fn retain_agent_marks(
        &mut self,
        vault: VaultId,
        item: ItemId,
        current: &[(&str, &[u8])],
    ) -> Result<()> {
        let key = self.account_key.agent_mark_key()?;
        let keep = current
            .iter()
            .map(|(id, canonical)| key.mark(&item, id, canonical))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        self.store.retain_agent_marks(&vault, &item, &keep)?;
        Ok(())
    }

    /// Seals one log record and stores it, then prunes entries older than 90 days (measured from
    /// `at_ms`) and those beyond the newest 10,000.
    ///
    /// # Errors
    /// [`super::Error::Core`], [`super::Error::Store`].
    pub fn append_agent_log(&mut self, at_ms: u64, plaintext: &[u8]) -> Result<()> {
        self.append_agent_log_class(at_ms, plaintext, false)
    }

    /// As [`Self::append_agent_log`]; a `noisy` (refusal) row is kept under its own cap of
    /// [`AGENT_LOG_MAX_NOISY`] and the other rows keep the 10,000 of their own.
    ///
    /// # Errors
    /// As [`Self::append_agent_log`].
    pub fn append_agent_log_class(
        &mut self,
        at_ms: u64,
        plaintext: &[u8],
        noisy: bool,
    ) -> Result<()> {
        self.append_agent_log_capped(
            at_ms,
            plaintext,
            AgentLogCaps {
                noisy,
                max_entries: AGENT_LOG_MAX_ENTRIES,
                max_noisy: AGENT_LOG_MAX_NOISY,
            },
        )
    }

    fn append_agent_log_capped(
        &mut self,
        at_ms: u64,
        plaintext: &[u8],
        caps: AgentLogCaps,
    ) -> Result<()> {
        let entry_id = new_agent_log_entry_id()?;
        let sealed = self
            .account_key
            .agent_log_key()?
            .seal(&entry_id, at_ms, plaintext)?;
        self.store.append_agent_log(
            &AgentLogRow {
                entry_id,
                at_ms,
                sealed,
            },
            at_ms.saturating_sub(AGENT_LOG_MAX_AGE_MS),
            caps,
        )?;
        Ok(())
    }

    /// Up to `limit` readable log entries older than `before_ms` (all when `None`), newest first.
    /// A row that does not open is skipped and counted, never an error for the page.
    ///
    /// # Errors
    /// [`super::Error::Core`], [`super::Error::Store`].
    pub fn agent_log_page(&self, before_ms: Option<u64>, limit: usize) -> Result<AgentLogPage> {
        let key = self.account_key.agent_log_key()?;
        let mut page = AgentLogPage {
            entries: Vec::new(),
            unreadable: 0,
        };
        for row in self.store.agent_log_rows(before_ms, limit)? {
            match key.open(&row.entry_id, row.at_ms, &row.sealed) {
                Ok(plain) => page.entries.push((row.at_ms, plain)),
                Err(_) => page.unreadable += 1,
            }
        }
        Ok(page)
    }

    /// [`Self::agent_log_page`] without the count of unreadable rows.
    ///
    /// # Errors
    /// As [`Self::agent_log_page`].
    pub fn agent_log(&self, before_ms: Option<u64>, limit: usize) -> Result<Vec<(u64, SecretVec)>> {
        Ok(self.agent_log_page(before_ms, limit)?.entries)
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::model::ItemType;
    use mosaic_core::{Argon2Profile, ItemId, MasterPassword, SecretKey, VaultId};

    use crate::session::{LockedSession, UnlockedSession};
    use crate::storage::{AgentLogCaps, PendingPolicy, Store};

    const CANON: &[u8] = br#"{"field":"api_key.secret"}"#;
    const DAY: u64 = 24 * 3600 * 1000;

    fn password() -> MasterPassword {
        MasterPassword::new("unit test password".to_owned())
    }

    fn account(store: Store) -> (UnlockedSession, SecretKey) {
        let (session, created) = LockedSession::open(store)
            .create_account(&password(), Argon2Profile::Baseline)
            .unwrap();
        (session, created.secret_key)
    }

    fn memory() -> UnlockedSession {
        account(Store::open_in_memory().unwrap()).0
    }

    fn with_item(session: &mut UnlockedSession) -> (VaultId, ItemId) {
        let vault = session.create_vault("V").unwrap();
        let item = session
            .add_item(vault, &ItemType::Login, |e| e.set_title("T"))
            .unwrap();
        (vault, item)
    }

    fn confirmed(
        session: &UnlockedSession,
        vault: VaultId,
        item: ItemId,
        id: &str,
        canonical: &[u8],
    ) -> bool {
        session
            .agent_mark_confirmed(vault, item, id, canonical)
            .unwrap()
    }

    #[test]
    fn marks_are_per_item_and_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let (mut session, secret_key) = account(Store::open(&path).unwrap());
        let (vault, item) = with_item(&mut session);
        let other = session
            .add_item(vault, &ItemType::Login, |e| e.set_title("U"))
            .unwrap();
        assert!(!confirmed(&session, vault, item, "b1", CANON));
        session
            .confirm_agent_mark(vault, item, "b1", CANON)
            .unwrap();
        session
            .confirm_agent_mark(vault, item, "b1", CANON)
            .unwrap();
        assert!(confirmed(&session, vault, item, "b1", CANON));
        assert!(!confirmed(&session, vault, item, "b1", b"{}"));
        assert!(!confirmed(&session, vault, item, "b2", CANON));
        assert!(!confirmed(&session, vault, other, "b1", CANON));
        drop(session.lock());
        let locked = LockedSession::open(Store::open(&path).unwrap());
        let session = locked.unlock(&password(), &secret_key).ok().unwrap();
        assert!(confirmed(&session, vault, item, "b1", CANON));
    }

    #[test]
    fn retain_drops_stale_marks() {
        let mut session = memory();
        let (vault, item) = with_item(&mut session);
        for id in ["b1", "b2", "b3"] {
            session.confirm_agent_mark(vault, item, id, CANON).unwrap();
        }
        session
            .retain_agent_marks(vault, item, &[("b1", CANON), ("b3", b"{}")])
            .unwrap();
        assert!(confirmed(&session, vault, item, "b1", CANON));
        assert!(!confirmed(&session, vault, item, "b2", CANON));
        assert!(!confirmed(&session, vault, item, "b3", CANON));
        session.retain_agent_marks(vault, item, &[]).unwrap();
        assert!(!confirmed(&session, vault, item, "b1", CANON));
    }

    #[test]
    fn purge_and_tombstone_remove_marks() {
        let mut session = memory();
        let (vault, item) = with_item(&mut session);
        session
            .confirm_agent_mark(vault, item, "b1", CANON)
            .unwrap();
        // The purge turns the record into a tombstone: the trigger removes the marks.
        session.purge_item(vault, item).unwrap();
        assert_eq!(
            session.store.agent_marks_of(&vault, &item).unwrap(),
            Vec::<[u8; 32]>::new()
        );
        // A confirmation for a deleted item is refused.
        assert!(
            session
                .confirm_agent_mark(vault, item, "b1", CANON)
                .is_err()
        );
        // Deleting the record row takes the marks too.
        let (vault, item) = with_item(&mut session);
        session
            .confirm_agent_mark(vault, item, "b1", CANON)
            .unwrap();
        assert!(
            session
                .store
                .delete_record(&vault, &item, PendingPolicy::Discard)
                .unwrap()
        );
        assert_eq!(
            session.store.agent_marks_of(&vault, &item).unwrap(),
            Vec::<[u8; 32]>::new()
        );
    }

    #[test]
    fn log_round_trips_newest_first_and_prunes() {
        let mut session = memory();
        let now = 200 * DAY;
        // An entry older than 90 days goes when a newer one arrives.
        session
            .append_agent_log(now - 91 * DAY, b"ancient")
            .unwrap();
        assert_eq!(session.agent_log(None, 10).unwrap().len(), 1);
        for n in 0..5u64 {
            session
                .append_agent_log_capped(
                    now + n,
                    format!("e{n}").as_bytes(),
                    AgentLogCaps {
                        noisy: false,
                        max_entries: 3,
                        max_noisy: 2,
                    },
                )
                .unwrap();
        }
        let page = session.agent_log_page(None, 10).unwrap();
        assert_eq!(page.unreadable, 0);
        let got: Vec<_> = page
            .entries
            .iter()
            .map(|(at, p)| (*at, String::from_utf8(p.expose_secret().to_vec()).unwrap()))
            .collect();
        assert_eq!(
            got,
            vec![
                (now + 4, "e4".to_owned()),
                (now + 3, "e3".to_owned()),
                (now + 2, "e2".to_owned())
            ]
        );
        // Paging.
        assert_eq!(session.agent_log(Some(now + 3), 10).unwrap().len(), 1);
        assert_eq!(session.agent_log(None, 2).unwrap().len(), 2);
    }

    #[test]
    fn refusal_rows_have_their_own_cap_and_never_evict_other_rows() {
        let mut session = memory();
        let now = 200 * DAY;
        let caps = |noisy| AgentLogCaps {
            noisy,
            max_entries: 3,
            max_noisy: 2,
        };
        session
            .append_agent_log_capped(now, b"grant", caps(false))
            .unwrap();
        session
            .append_agent_log_capped(now + 1, b"pairing", caps(false))
            .unwrap();
        for n in 0..50u64 {
            session
                .append_agent_log_capped(now + 2 + n, format!("r{n}").as_bytes(), caps(true))
                .unwrap();
        }
        let text = |rows: Vec<(u64, mosaic_core::SecretVec)>| -> Vec<String> {
            rows.into_iter()
                .map(|(_, p)| String::from_utf8(p.expose_secret().to_vec()).unwrap())
                .collect()
        };
        assert_eq!(
            text(session.agent_log(None, 100).unwrap()),
            vec!["r49", "r48", "pairing", "grant"]
        );
        // And plain rows never evict refusal rows.
        for n in 0..5u64 {
            session
                .append_agent_log_capped(now + 100 + n, b"plain", caps(false))
                .unwrap();
        }
        let got = text(session.agent_log(None, 100).unwrap());
        assert_eq!(got.len(), 5, "3 plain + 2 refusals");
        assert!(got.contains(&"r49".to_owned()));
    }

    #[test]
    fn log_rows_hold_no_plaintext() {
        let mut session = memory();
        let canary = b"SK-TEST-CANARY-agent-log-plaintext";
        session.append_agent_log(5, canary).unwrap();
        let rows = session.store.agent_log_rows(None, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            !rows[0].sealed.windows(canary.len()).any(|w| w == canary),
            "canary found"
        );
        assert_eq!(rows[0].sealed[..2], [1, 1]);
    }

    #[test]
    fn rows_of_another_key_are_skipped_and_counted() {
        let mut session = memory();
        session.append_agent_log(5, b"mine").unwrap();
        // A row sealed under another time does not open under the stored one.
        let mut row = session.store.agent_log_rows(None, 1).unwrap().remove(0);
        row.entry_id = [7; 16];
        row.at_ms = 6;
        session
            .store
            .append_agent_log(
                &row,
                0,
                AgentLogCaps {
                    noisy: false,
                    max_entries: 10,
                    max_noisy: 10,
                },
            )
            .unwrap();
        let page = session.agent_log_page(None, 10).unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.unreadable, 1);
    }

    #[test]
    fn log_is_not_in_the_outbox_or_export() {
        let mut session = memory();
        let (vault, item) = with_item(&mut session);
        let before = session.store.outbox(100).unwrap().len();
        session
            .confirm_agent_mark(vault, item, "b1", CANON)
            .unwrap();
        session.append_agent_log(5, b"entry").unwrap();
        assert_eq!(session.store.outbox(100).unwrap().len(), before);
    }
}
