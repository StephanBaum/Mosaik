//! Schema and migrations.
//!
//! The schema version is SQLite's `user_version`. Migration `n` takes a
//! store from version `n - 1` to `n`; each one runs in its own `IMMEDIATE`
//! transaction together with the `user_version` bump, so a failed migration
//! leaves the store at the previous version. A store whose version is newer
//! than [`LATEST_VERSION`] is rejected and never modified.
//!
//! Every table is `STRICT`. Generations are `u32` (1 to 2^32 - 1), IDs are
//! 16-byte non-nil blobs. Triggers repeat the anti-rollback rules that the
//! Rust code enforces (CRYPTO_SPEC §5.3, §7.1), so even a bug in this crate
//! cannot lower a vault's accepted key generation or the Argon2id profile.

use rusqlite::{Connection, TransactionBehavior};

use super::error::{Error, Result};

/// `PRAGMA application_id` of a Mosaic store: ASCII `"MSC1"`.
pub(crate) const APPLICATION_ID: i32 = 0x4D53_4331;

/// One schema migration.
pub(crate) struct Migration {
    /// The `user_version` after this migration.
    pub version: u32,
    /// The SQL to run.
    pub sql: &'static str,
}

/// Version 1: the initial schema.
const V1: &str = r"
-- The single account of this store (CRYPTO_SPEC §5.3, §12: public values).
CREATE TABLE account (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    account_id  BLOB    NOT NULL CHECK (length(account_id) = 16 AND account_id <> zeroblob(16)),
    salt        BLOB    NOT NULL CHECK (length(salt) = 16),
    profile     INTEGER NOT NULL CHECK (profile BETWEEN 1 AND 4),
    updated_at  INTEGER NOT NULL
) STRICT;

CREATE TRIGGER account_id_immutable
BEFORE UPDATE OF account_id ON account
WHEN NEW.account_id <> OLD.account_id
BEGIN SELECT RAISE(ABORT, 'mosaic: account_id is immutable'); END;

CREATE TRIGGER account_profile_monotonic
BEFORE UPDATE OF profile ON account
WHEN NEW.profile < OLD.profile
BEGIN SELECT RAISE(ABORT, 'mosaic: Argon2id profile downgrade'); END;

-- One wrapping of AK per unlock method (CRYPTO_SPEC §6). `params` holds the
-- method's public parameters (for example the §6.3 PRF salt).
CREATE TABLE unlock_methods (
    method      INTEGER NOT NULL CHECK (method IN (1, 2, 3)),
    method_id   BLOB    NOT NULL CHECK (length(method_id) = 16),
    wrapped_ak  BLOB    NOT NULL CHECK (length(wrapped_ak) = 74),
    params      BLOB    NOT NULL CHECK (length(params) <= 4096),
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (method, method_id)
) STRICT, WITHOUT ROWID;

CREATE UNIQUE INDEX unlock_methods_one_password ON unlock_methods (method) WHERE method = 1;

-- Vaults (CRYPTO_SPEC §7.1). `highest_accepted_generation` is the
-- anti-rollback high-water mark; `current_generation` is the generation of
-- the newest vault key held, which new objects are sealed under. `key_wrap`
-- says what every vault key of the vault is wrapped under: 1 = AK (§7.1),
-- 2 = the TK of a hardware-protected TOTP vault (§7.4). It never changes.
CREATE TABLE vaults (
    vault_id                    BLOB    PRIMARY KEY
                                CHECK (length(vault_id) = 16 AND vault_id <> zeroblob(16)),
    key_wrap                    INTEGER NOT NULL CHECK (key_wrap IN (1, 2)),
    current_generation          INTEGER NOT NULL CHECK (current_generation BETWEEN 1 AND 4294967295),
    highest_accepted_generation INTEGER NOT NULL
                                CHECK (highest_accepted_generation BETWEEN 1 AND 4294967295),
    meta                        BLOB,
    meta_generation             INTEGER CHECK (meta_generation BETWEEN 1 AND 4294967295),
    updated_at                  INTEGER NOT NULL,
    CHECK (current_generation <= highest_accepted_generation),
    CHECK ((meta IS NULL) = (meta_generation IS NULL)),
    CHECK (meta_generation IS NULL OR meta_generation <= highest_accepted_generation)
) STRICT;

CREATE TRIGGER vaults_immutable
BEFORE UPDATE OF vault_id, key_wrap ON vaults
WHEN NEW.vault_id <> OLD.vault_id OR NEW.key_wrap <> OLD.key_wrap
BEGIN SELECT RAISE(ABORT, 'mosaic: vault_id and key_wrap are immutable'); END;

CREATE TRIGGER vaults_generation_monotonic
BEFORE UPDATE OF current_generation, highest_accepted_generation ON vaults
WHEN NEW.highest_accepted_generation < OLD.highest_accepted_generation
  OR NEW.current_generation < OLD.current_generation
BEGIN SELECT RAISE(ABORT, 'mosaic: key generation rollback'); END;

CREATE TRIGGER vaults_meta_generation
BEFORE UPDATE OF meta, meta_generation ON vaults
WHEN NEW.meta IS NOT NULL AND NEW.meta_generation <> NEW.highest_accepted_generation
BEGIN SELECT RAISE(ABORT, 'mosaic: vault metadata generation not accepted'); END;

-- The anti-rollback mark outlives the vault (CRYPTO_SPEC §7.1): when a vault
-- is deleted, its highest accepted generation stays here, and the vault can
-- only come back at that generation or a later one. Rows are never deleted
-- and never decrease.
CREATE TABLE vault_generation_floors (
    vault_id    BLOB    PRIMARY KEY
                CHECK (length(vault_id) = 16 AND vault_id <> zeroblob(16)),
    generation  INTEGER NOT NULL CHECK (generation BETWEEN 1 AND 4294967295),
    updated_at  INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TRIGGER vault_generation_floors_monotonic
BEFORE UPDATE ON vault_generation_floors
WHEN NEW.generation < OLD.generation OR NEW.vault_id <> OLD.vault_id
BEGIN SELECT RAISE(ABORT, 'mosaic: key generation rollback'); END;

CREATE TRIGGER vault_generation_floors_permanent
BEFORE DELETE ON vault_generation_floors
BEGIN SELECT RAISE(ABORT, 'mosaic: generation floors are permanent'); END;

CREATE TRIGGER vaults_insert_above_floor
BEFORE INSERT ON vaults
WHEN NEW.highest_accepted_generation
     < coalesce((SELECT generation FROM vault_generation_floors WHERE vault_id = NEW.vault_id), 0)
BEGIN SELECT RAISE(ABORT, 'mosaic: key generation rollback'); END;

CREATE TRIGGER vaults_delete_keeps_floor
BEFORE DELETE ON vaults
WHEN OLD.highest_accepted_generation
     > coalesce((SELECT generation FROM vault_generation_floors WHERE vault_id = OLD.vault_id), 0)
BEGIN SELECT RAISE(ABORT, 'mosaic: generation floor not recorded'); END;

-- Wrapped vault keys, one per generation (CRYPTO_SPEC §7.1, §7.4), wrapped
-- under what the vault's `key_wrap` says. No new row below the accepted
-- generation; an existing row may be re-wrapped (`wrapped_vk` replaced, for
-- example by a §6.6 re-key), but its vault and generation never change.
CREATE TABLE vault_keys (
    vault_id    BLOB    NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    generation  INTEGER NOT NULL CHECK (generation BETWEEN 1 AND 4294967295),
    wrapped_vk  BLOB    NOT NULL CHECK (length(wrapped_vk) = 74),
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (vault_id, generation)
) STRICT, WITHOUT ROWID;

CREATE TRIGGER vault_keys_generation_insert
BEFORE INSERT ON vault_keys
WHEN NEW.generation < (SELECT highest_accepted_generation FROM vaults WHERE vault_id = NEW.vault_id)
BEGIN SELECT RAISE(ABORT, 'mosaic: key generation rollback'); END;

CREATE TRIGGER vault_keys_generation_update
BEFORE UPDATE OF vault_id, generation ON vault_keys
WHEN NEW.vault_id <> OLD.vault_id OR NEW.generation <> OLD.generation
BEGIN SELECT RAISE(ABORT, 'mosaic: vault key generation is immutable'); END;

-- The wrappings of a hardware-protected TOTP vault's TK, one per registered
-- hardware key (CRYPTO_SPEC §7.4). `method_id` = SHA-256(credential_id)[..16]
-- and `params` holds that key's own `prf_salt_hw`. This is a table of its
-- own, so a TK wrapping can never replace the same credential's §6.3 AK
-- wrapping in `unlock_methods`.
CREATE TABLE tk_wrappings (
    vault_id    BLOB    NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    method_id   BLOB    NOT NULL CHECK (length(method_id) = 16),
    wrapped_tk  BLOB    NOT NULL CHECK (length(wrapped_tk) = 74),
    params      BLOB    NOT NULL CHECK (length(params) <= 4096),
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (vault_id, method_id)
) STRICT, WITHOUT ROWID;

CREATE TRIGGER tk_wrappings_insert_tk_vault
BEFORE INSERT ON tk_wrappings
WHEN (SELECT key_wrap FROM vaults WHERE vault_id = NEW.vault_id) IS NOT 2
BEGIN SELECT RAISE(ABORT, 'mosaic: TK wrapping for a vault not under TK'); END;

CREATE TRIGGER tk_wrappings_update_tk_vault
BEFORE UPDATE OF vault_id, method_id ON tk_wrappings
WHEN NEW.vault_id <> OLD.vault_id OR NEW.method_id <> OLD.method_id
BEGIN SELECT RAISE(ABORT, 'mosaic: TK wrapping key is immutable'); END;

-- Sync records (CRYPTO_SPEC §7.2, §10): items and tombstones. `outbox_seq`
-- is set while a local change waits for upload (the outbox); it comes from
-- `outbox_counter` and is never reused. There is no cleartext sync state:
-- the highest accepted version vector is the one inside the stored
-- ciphertext (ADR-0010).
CREATE TABLE records (
    vault_id        BLOB    NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    record_id       BLOB    NOT NULL CHECK (length(record_id) = 16 AND record_id <> zeroblob(16)),
    kind            INTEGER NOT NULL CHECK (kind IN (1, 2)),
    key_generation  INTEGER NOT NULL CHECK (key_generation BETWEEN 1 AND 4294967295),
    ciphertext      BLOB    NOT NULL,
    server_seq      INTEGER CHECK (server_seq >= 0),
    outbox_seq      INTEGER UNIQUE CHECK (outbox_seq >= 1),
    updated_at      INTEGER NOT NULL,
    UNIQUE (vault_id, record_id)
) STRICT;

CREATE TRIGGER records_generation_insert
BEFORE INSERT ON records
WHEN NEW.key_generation <> (SELECT highest_accepted_generation FROM vaults WHERE vault_id = NEW.vault_id)
BEGIN SELECT RAISE(ABORT, 'mosaic: record key generation not accepted'); END;

CREATE TRIGGER records_generation_update
BEFORE UPDATE OF ciphertext, key_generation ON records
WHEN NEW.key_generation <> (SELECT highest_accepted_generation FROM vaults WHERE vault_id = NEW.vault_id)
BEGIN SELECT RAISE(ABORT, 'mosaic: record key generation not accepted'); END;

CREATE INDEX records_outbox ON records (outbox_seq) WHERE outbox_seq IS NOT NULL;

-- This install's device ID (CRYPTO_SPEC §10, §12: public). It stamps every
-- local edit. It is replaced only when the ID turns out to be reused.
CREATE TABLE local_device (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    device_id   BLOB    NOT NULL
                CHECK (length(device_id) = 16 AND device_id <> zeroblob(16)),
    updated_at  INTEGER NOT NULL
) STRICT;

CREATE TABLE outbox_counter (
    id      INTEGER PRIMARY KEY CHECK (id = 1),
    last    INTEGER NOT NULL CHECK (last >= 0)
) STRICT;

INSERT INTO outbox_counter (id, last) VALUES (1, 0);

-- Per-vault sync cursor: the highest server_seq pulled (CRYPTO_SPEC §10).
CREATE TABLE sync_cursors (
    vault_id    BLOB    PRIMARY KEY REFERENCES vaults (vault_id) ON DELETE CASCADE,
    server_seq  INTEGER NOT NULL CHECK (server_seq >= 0),
    updated_at  INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TRIGGER sync_cursors_monotonic
BEFORE UPDATE OF server_seq ON sync_cursors
WHEN NEW.server_seq < OLD.server_seq
BEGIN SELECT RAISE(ABORT, 'mosaic: sync cursor rollback'); END;

-- Attachment ciphertexts live in files next to the database (CRYPTO_SPEC
-- §7.3); this table holds their routing metadata.
CREATE TABLE attachments (
    attachment_id   BLOB    PRIMARY KEY
                    CHECK (length(attachment_id) = 16 AND attachment_id <> zeroblob(16)),
    vault_id        BLOB    NOT NULL,
    record_id       BLOB    NOT NULL,
    size            INTEGER NOT NULL CHECK (size >= 0),
    upload_pending  INTEGER NOT NULL CHECK (upload_pending IN (0, 1)),
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (vault_id, record_id) REFERENCES records (vault_id, record_id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

CREATE INDEX attachments_record ON attachments (vault_id, record_id);
";

/// All migrations, in order. Version `n` is at index `n - 1`.
pub(crate) const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: V1,
}];

/// The newest schema version this build knows.
pub const LATEST_VERSION: u32 = 1;

fn user_version(conn: &Connection) -> Result<u32> {
    let v: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    u32::try_from(v).map_err(|_| Error::Corrupt)
}

fn application_id(conn: &Connection) -> Result<i32> {
    Ok(conn.pragma_query_value(None, "application_id", |r| r.get(0))?)
}

/// The newest version in `migrations`.
pub(crate) fn latest_of(migrations: &[Migration]) -> u32 {
    migrations.last().map_or(0, |m| m.version)
}

/// Checks, reading only, that the database is a fresh one (no schema
/// objects, `user_version` 0, no application ID) or a Mosaic store no newer
/// than `latest`, and returns its version. [`super::Store`] runs this before
/// it sets any persistent pragma, so a foreign or newer file is refused
/// without a single write.
pub(crate) fn check_compatible(conn: &Connection, latest: u32) -> Result<u32> {
    let version = user_version(conn)?;
    let app_id = application_id(conn)?;
    if version == 0 && app_id == 0 {
        let objects: i64 =
            conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
        if objects != 0 {
            return Err(Error::NotAMosaicStore);
        }
    } else if app_id != APPLICATION_ID {
        return Err(Error::NotAMosaicStore);
    }
    if version > latest {
        return Err(Error::SchemaTooNew {
            found: version,
            supported: latest,
        });
    }
    Ok(version)
}

/// Brings the database to the newest version in `migrations`.
///
/// A fresh database (no schema objects, `user_version` 0) is stamped with
/// [`APPLICATION_ID`]. Any other database must already carry it.
pub(crate) fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<u32> {
    let latest = latest_of(migrations);
    loop {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version = check_compatible(&tx, latest)?;
        let Some(next) = migrations.iter().find(|m| m.version > version) else {
            return Ok(version);
        };
        if next.version != version.saturating_add(1) {
            // The migration list has a gap: a build bug, never a data issue.
            return Err(Error::Corrupt);
        }
        tx.execute_batch(next.sql)?;
        tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        tx.pragma_update(None, "user_version", next.version)?;
        tx.commit()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        c
    }

    #[test]
    fn migration_list_is_contiguous() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.version as usize, i + 1);
        }
        assert_eq!(MIGRATIONS.last().unwrap().version, LATEST_VERSION);
    }

    #[test]
    fn fresh_database_reaches_latest() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
        assert_eq!(user_version(&c).unwrap(), LATEST_VERSION);
        assert_eq!(application_id(&c).unwrap(), APPLICATION_ID);
        // Idempotent.
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
    }

    const V2_TEST: &str = "
        ALTER TABLE vaults ADD COLUMN test_flag INTEGER NOT NULL DEFAULT 7;
        CREATE TABLE test_v2 (x INTEGER) STRICT;
    ";

    /// The upgrade path: a v1 store with data is upgraded to a later
    /// version, and its data survives.
    #[test]
    fn upgrade_path_preserves_data() {
        let mut c = conn();
        migrate(&mut c, &MIGRATIONS[..1]).unwrap();
        c.execute(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (?1, 1, 1, 1, 0)",
            [[7u8; 16]],
        )
        .unwrap();
        let upgraded = [
            Migration {
                version: 1,
                sql: V1,
            },
            Migration {
                version: 2,
                sql: V2_TEST,
            },
        ];
        assert_eq!(migrate(&mut c, &upgraded).unwrap(), 2);
        let flag: i64 = c
            .query_row(
                "SELECT test_flag FROM vaults WHERE vault_id = ?1",
                [[7u8; 16]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 7);
        // Back on a build that only knows v1, the store is refused, not
        // touched.
        assert!(matches!(
            migrate(&mut c, MIGRATIONS),
            Err(Error::SchemaTooNew {
                found: 2,
                supported: 1
            })
        ));
        assert_eq!(user_version(&c).unwrap(), 2);
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        let broken = [
            Migration {
                version: 1,
                sql: V1,
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE half_done (x INTEGER); THIS IS NOT SQL;",
            },
        ];
        assert!(matches!(migrate(&mut c, &broken), Err(Error::Database(_))));
        assert_eq!(user_version(&c).unwrap(), 1);
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'half_done'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn gap_in_migrations_is_refused() {
        let mut c = conn();
        let gappy = [Migration {
            version: 2,
            sql: V2_TEST,
        }];
        assert!(matches!(migrate(&mut c, &gappy), Err(Error::Corrupt)));
        assert_eq!(user_version(&c).unwrap(), 0);
    }

    #[test]
    fn foreign_databases_are_refused() {
        let mut c = conn();
        c.execute_batch("CREATE TABLE other (x INTEGER);").unwrap();
        assert!(matches!(
            migrate(&mut c, MIGRATIONS),
            Err(Error::NotAMosaicStore)
        ));

        let mut c = conn();
        c.pragma_update(None, "application_id", 1234).unwrap();
        assert!(matches!(
            migrate(&mut c, MIGRATIONS),
            Err(Error::NotAMosaicStore)
        ));

        let mut c = conn();
        c.pragma_update(None, "user_version", 1).unwrap();
        assert!(matches!(
            migrate(&mut c, MIGRATIONS),
            Err(Error::NotAMosaicStore)
        ));
    }

    #[test]
    fn triggers_block_rollbacks_even_through_raw_sql() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        let v = [9u8; 16];
        c.execute(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (?1, 1, 3, 3, 0)",
            [v],
        )
        .unwrap();
        c.execute(
            "INSERT INTO vault_keys (vault_id, generation, wrapped_vk, updated_at)
             VALUES (?1, 3, zeroblob(74), 0)",
            [v],
        )
        .unwrap();
        for sql in [
            "UPDATE vaults SET highest_accepted_generation = 2 WHERE vault_id = ?1",
            "UPDATE vaults SET current_generation = 2 WHERE vault_id = ?1",
            "UPDATE vaults SET key_wrap = 2 WHERE vault_id = ?1",
            "INSERT INTO vault_keys (vault_id, generation, wrapped_vk, updated_at)
             VALUES (?1, 2, zeroblob(74), 0)",
            "UPDATE vault_keys SET generation = 2 WHERE vault_id = ?1",
            "INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext,
                                  updated_at)
             VALUES (?1, ?1, 1, 2, x'00', 0)",
            "INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext,
                                  updated_at)
             VALUES (?1, ?1, 1, 4, x'00', 0)",
            "UPDATE vaults SET meta = x'00', meta_generation = 2 WHERE vault_id = ?1",
            // A TK wrapping only belongs to a vault under TK (§7.4).
            "INSERT INTO tk_wrappings (vault_id, method_id, wrapped_tk, params, updated_at)
             VALUES (?1, ?1, zeroblob(74), x'', 0)",
            // Deleting a vault needs its floor first.
            "DELETE FROM vaults WHERE vault_id = ?1",
        ] {
            let e = c.execute(sql, [v]).unwrap_err();
            assert!(e.to_string().contains("mosaic:"), "{sql}: {e}");
        }
        // Re-wrapping a held key in place is allowed, even below the mark.
        c.execute(
            "UPDATE vault_keys SET wrapped_vk = zeroblob(74), updated_at = 1 WHERE vault_id = ?1",
            [v],
        )
        .unwrap();

        // The floor: recorded, then the vault can go, and it can only come
        // back at the floor or above. Floors never drop or disappear.
        c.execute(
            "INSERT INTO vault_generation_floors (vault_id, generation, updated_at)
             VALUES (?1, 3, 0)",
            [v],
        )
        .unwrap();
        c.execute("DELETE FROM vaults WHERE vault_id = ?1", [v])
            .unwrap();
        for sql in [
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (?1, 1, 2, 2, 0)",
            "UPDATE vault_generation_floors SET generation = 2 WHERE vault_id = ?1",
            "DELETE FROM vault_generation_floors WHERE vault_id = ?1",
        ] {
            let e = c.execute(sql, [v]).unwrap_err();
            assert!(e.to_string().contains("mosaic:"), "{sql}: {e}");
        }
        c.execute(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (?1, 2, 3, 3, 0)",
            [v],
        )
        .unwrap();
        c.execute(
            "INSERT INTO tk_wrappings (vault_id, method_id, wrapped_tk, params, updated_at)
             VALUES (?1, ?1, zeroblob(74), x'', 0)",
            [v],
        )
        .unwrap();
        let e = c
            .execute("UPDATE tk_wrappings SET method_id = zeroblob(16)", [])
            .unwrap_err();
        assert!(e.to_string().contains("mosaic:"), "{e}");
        c.execute(
            "INSERT INTO account (id, account_id, salt, profile, updated_at)
             VALUES (1, ?1, zeroblob(16), 2, 0)",
            [v],
        )
        .unwrap();
        for sql in [
            "UPDATE account SET profile = 1",
            "UPDATE account SET account_id = x'0102030405060708090a0b0c0d0e0f10'",
        ] {
            let e = c.execute(sql, []).unwrap_err();
            assert!(e.to_string().contains("mosaic:"), "{sql}: {e}");
        }
        c.execute(
            "INSERT INTO sync_cursors (vault_id, server_seq, updated_at) VALUES (?1, 10, 0)",
            [v],
        )
        .unwrap();
        let e = c
            .execute("UPDATE sync_cursors SET server_seq = 9", [])
            .unwrap_err();
        assert!(e.to_string().contains("mosaic:"), "{e}");
    }
}
