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

/// Version 2: the local-only settings nonce (CRYPTO_SPEC §11.1, ADR-0030).
/// Additive: one new table, nothing else changes. It is not a record and
/// never enters the outbox.
const V2: &str = r"
CREATE TABLE settings_nonce (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    nonce       BLOB    NOT NULL CHECK (length(nonce) = 16),
    updated_at  INTEGER NOT NULL
) STRICT;
";

/// Version 3: the sync state of M6 (CRYPTO_SPEC §10, ADR-0045). Additive:
/// new columns with neutral defaults and new tables. Nothing here is derived
/// from item plaintext (§11); the install ID is created by `Store::open`.
const V3: &str = r"
-- Upload queue of wrapped vault keys and vault metadata (M2-R1, M2-R11).
-- `local_origin` marks a key this install generated (it names itself as the
-- key's device in the canonical-winner rule, §10.1).
ALTER TABLE vault_keys ADD COLUMN upload_pending INTEGER NOT NULL DEFAULT 0
    CHECK (upload_pending IN (0, 1));
ALTER TABLE vault_keys ADD COLUMN local_origin INTEGER NOT NULL DEFAULT 0
    CHECK (local_origin IN (0, 1));
ALTER TABLE vaults ADD COLUMN remote_created INTEGER NOT NULL DEFAULT 0
    CHECK (remote_created IN (0, 1));
ALTER TABLE vaults ADD COLUMN meta_upload_pending INTEGER NOT NULL DEFAULT 0
    CHECK (meta_upload_pending IN (0, 1));
ALTER TABLE vaults ADD COLUMN meta_server_seq INTEGER CHECK (meta_server_seq >= 0);

-- This install's stable ID (CRYPTO_SPEC §10: checkpoints, folder paths, server
-- device lists). Unlike the writer ID in `local_device` it is created once.
CREATE TABLE install_identity (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    install_id  BLOB    NOT NULL
                CHECK (length(install_id) = 16 AND install_id <> zeroblob(16)),
    created_at  INTEGER NOT NULL
) STRICT;

-- Where this account syncs to (cleartext local metadata, §11). 1 = hosted,
-- 2 = folder.
CREATE TABLE sync_config (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    kind         INTEGER NOT NULL CHECK (kind IN (1, 2)),
    server_url   TEXT    CHECK (server_url IS NULL OR length(server_url) <= 2048),
    email        TEXT    CHECK (email IS NULL OR length(email) <= 320),
    folder_path  TEXT    CHECK (folder_path IS NULL OR length(folder_path) <= 4096),
    device_name  TEXT    NOT NULL CHECK (length(device_name) BETWEEN 1 AND 64),
    updated_at   INTEGER NOT NULL,
    CHECK ((kind = 1 AND server_url IS NOT NULL AND folder_path IS NULL)
        OR (kind = 2 AND folder_path IS NOT NULL AND server_url IS NULL AND email IS NULL))
) STRICT;

-- Opaque per-vault pull cursors of either backend (the legacy `sync_cursors`
-- table stays but M6 does not use it).
CREATE TABLE sync_cursor_blobs (
    vault_id    BLOB    PRIMARY KEY REFERENCES vaults (vault_id) ON DELETE CASCADE,
    cursor      BLOB    NOT NULL CHECK (length(cursor) <= 65536),
    updated_at  INTEGER NOT NULL
) STRICT;

-- Device-local sequence numbers for backends without a server sequence. The
-- stored value is the first number of the next block to reserve, so a crash
-- skips numbers and never reuses one.
CREATE TABLE sync_local_seq (
    vault_id    BLOB    PRIMARY KEY REFERENCES vaults (vault_id) ON DELETE CASCADE,
    next_seq    INTEGER NOT NULL CHECK (next_seq >= 1)
) STRICT;

-- The checkpoint counter (§10); it never decreases.
CREATE TABLE sync_checkpoint (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    counter  INTEGER NOT NULL CHECK (counter >= 0)
) STRICT;

INSERT INTO sync_checkpoint (id, counter) VALUES (1, 0);

CREATE TRIGGER sync_checkpoint_monotonic
BEFORE UPDATE ON sync_checkpoint
WHEN NEW.counter < OLD.counter
BEGIN SELECT RAISE(ABORT, 'mosaic: checkpoint counter decreased'); END;

-- Attachments deleted here after they had been uploaded: the backend still
-- has to be told (soft delete, M3-R25).
CREATE TABLE attachment_deletions (
    attachment_id  BLOB    PRIMARY KEY CHECK (length(attachment_id) = 16),
    vault_id       BLOB    NOT NULL CHECK (length(vault_id) = 16),
    deleted_at     INTEGER NOT NULL
) STRICT;
";

/// Version 4: the persisted state of the sync engine (CRYPTO_SPEC §11,
/// ADR-0045 amendment). Additive: new tables only, nothing is derived from
/// item plaintext. Everything here is local cleartext sync metadata like the
/// cursors: IDs, counters, times and wrapped keys (ciphertext).
const V4: &str = r"
-- This install's own log, per vault: `reserved` is the highest sequence
-- number that may have been used (persisted before the segment is written, so
-- a rolled-back folder never makes the writer reuse one); `written` is the
-- highest that was written successfully (persisted after).
CREATE TABLE sync_own_seq (
    vault_id  BLOB    PRIMARY KEY CHECK (length(vault_id) = 16),
    reserved  INTEGER NOT NULL DEFAULT 0 CHECK (reserved >= 0),
    written   INTEGER NOT NULL DEFAULT 0 CHECK (written >= 0)
) STRICT;

-- The engine's halt, checkpoint and strike state (one row). `halt`: 0 none,
-- 1 rollback detected, 2 install identity duplicated. `strike_counter` is the
-- last published counter at which a strike was counted (one strike per value).
CREATE TABLE sync_engine_state (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    halt                    INTEGER NOT NULL DEFAULT 0 CHECK (halt IN (0, 1, 2)),
    last_published_counter  INTEGER NOT NULL DEFAULT 0 CHECK (last_published_counter >= 0),
    strike_count            INTEGER NOT NULL DEFAULT 0 CHECK (strike_count >= 0),
    strike_first_ms         INTEGER CHECK (strike_first_ms IS NULL OR strike_first_ms >= 0),
    strike_counter          INTEGER CHECK (strike_counter IS NULL OR strike_counter >= 0)
) STRICT;

INSERT INTO sync_engine_state (id) VALUES (1);

-- The highest verified checkpoint of every other device. `entries` holds
-- 28 bytes per vault (vault ID, generation, high sequence number).
CREATE TABLE sync_peer_checkpoint (
    device_id     BLOB    PRIMARY KEY CHECK (length(device_id) = 16),
    counter       INTEGER NOT NULL CHECK (counter >= 0),
    issued_at_ms  INTEGER NOT NULL CHECK (issued_at_ms >= 0),
    entries       BLOB    NOT NULL CHECK (length(entries) <= 114688 AND length(entries) % 28 = 0)
) STRICT;

-- Since when another device lags in a vault (freeze / missing files).
CREATE TABLE sync_lag (
    device_id  BLOB    NOT NULL CHECK (length(device_id) = 16),
    vault_id   BLOB    NOT NULL CHECK (length(vault_id) = 16),
    since_ms   INTEGER NOT NULL CHECK (since_ms >= 0),
    PRIMARY KEY (device_id, vault_id)
) STRICT;

-- Wrapped vault keys seen for a generation and the lowest device that wrote
-- exactly those bytes (the canonical-winner rule, CRYPTO_SPEC §10.1).
CREATE TABLE sync_key_candidate (
    vault_id    BLOB    NOT NULL CHECK (length(vault_id) = 16),
    generation  INTEGER NOT NULL CHECK (generation >= 1),
    wrapped     BLOB    NOT NULL CHECK (length(wrapped) BETWEEN 1 AND 256),
    device_id   BLOB    NOT NULL CHECK (length(device_id) = 16),
    PRIMARY KEY (vault_id, generation, wrapped)
) STRICT;

-- When this device first saw a blob delete marker, by its own clock.
CREATE TABLE sync_marker_seen (
    vault_id       BLOB    NOT NULL CHECK (length(vault_id) = 16),
    attachment_id  BLOB    NOT NULL CHECK (length(attachment_id) = 16),
    device_id      BLOB    NOT NULL CHECK (length(device_id) = 16),
    stamp_ms       INTEGER NOT NULL CHECK (stamp_ms >= 0),
    first_seen_ms  INTEGER NOT NULL CHECK (first_seen_ms >= 0),
    PRIMARY KEY (vault_id, attachment_id, device_id)
) STRICT;
";

/// Version 5: the served view and the engine's persisted server findings
/// (CRYPTO_SPEC §10, ADR-0045 amendment 2). `sync_peer_checkpoint` is
/// recreated for the 60-byte entries of checkpoint v1 (view digest added in
/// place: v1 was never released); the newest checkpoints are fetched again
/// by the next run. The other tables are new: the latest version of every
/// record the server showed or accepted (`sync_served`), the vault keys the
/// server confirmed (`sync_key_confirmed`), the per-vault server findings
/// that stay until the user acknowledges them (`sync_vault_flag`: 1 server
/// inconsistent, 2 server rolled back) and the conflict-driven rotations of
/// the last day (`sync_rotation_log`).
const V5: &str = r"
DROP TABLE sync_peer_checkpoint;
CREATE TABLE sync_peer_checkpoint (
    device_id     BLOB    PRIMARY KEY CHECK (length(device_id) = 16),
    counter       INTEGER NOT NULL CHECK (counter >= 0),
    issued_at_ms  INTEGER NOT NULL CHECK (issued_at_ms >= 0),
    entries       BLOB    NOT NULL CHECK (length(entries) <= 245760 AND length(entries) % 60 = 0),
    -- 1: the user acknowledged a finding against the server after this
    -- checkpoint arrived; its view digests are not compared any more.
    digest_ignored INTEGER NOT NULL DEFAULT 0 CHECK (digest_ignored IN (0, 1))
) STRICT;

CREATE TABLE sync_served (
    vault_id   BLOB    NOT NULL CHECK (length(vault_id) = 16),
    record_id  BLOB    NOT NULL CHECK (length(record_id) = 16),
    seq        INTEGER NOT NULL CHECK (seq >= 0),
    ct_hash    BLOB    NOT NULL CHECK (length(ct_hash) = 32),
    PRIMARY KEY (vault_id, record_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE sync_key_confirmed (
    vault_id    BLOB    NOT NULL CHECK (length(vault_id) = 16),
    generation  INTEGER NOT NULL CHECK (generation >= 1),
    PRIMARY KEY (vault_id, generation)
) STRICT, WITHOUT ROWID;

CREATE TABLE sync_vault_flag (
    vault_id  BLOB    NOT NULL CHECK (length(vault_id) = 16),
    kind      INTEGER NOT NULL CHECK (kind IN (1, 2)),
    PRIMARY KEY (vault_id, kind)
) STRICT, WITHOUT ROWID;

CREATE TABLE sync_rotation_log (
    vault_id  BLOB    NOT NULL CHECK (length(vault_id) = 16),
    at_ms     INTEGER NOT NULL CHECK (at_ms >= 0)
) STRICT;
CREATE INDEX sync_rotation_log_vault ON sync_rotation_log (vault_id, at_ms);

-- Per (peer device, vault): since when this device's comparisons of view
-- digests with the peer keep failing to happen (NULL: the last one
-- succeeded) and the peer's checkpoint counter then; `sync_no_common_view` holds the findings that the
-- heads never met (persisted until acknowledged); `sync_rebuild` marks the
-- vaults whose served view was dropped by an acknowledgement and that must
-- be read again from the start before local records the server lacks are
-- queued again.
CREATE TABLE sync_view_compare (
    device_id        BLOB    NOT NULL CHECK (length(device_id) = 16),
    vault_id         BLOB    NOT NULL CHECK (length(vault_id) = 16),
    since_ms         INTEGER CHECK (since_ms IS NULL OR since_ms >= 0),
    counter          INTEGER NOT NULL CHECK (counter >= 0),
    PRIMARY KEY (device_id, vault_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE sync_no_common_view (
    device_id  BLOB    NOT NULL CHECK (length(device_id) = 16),
    vault_id   BLOB    NOT NULL CHECK (length(vault_id) = 16),
    since_ms   INTEGER NOT NULL CHECK (since_ms >= 0),
    PRIMARY KEY (device_id, vault_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE sync_rebuild (
    vault_id  BLOB    PRIMARY KEY CHECK (length(vault_id) = 16)
) STRICT, WITHOUT ROWID;

-- Seeds for a store that synced before v5. A key this install generated
-- whose upload flag is clear, at the vault's current generation, counts as
-- confirmed (the conservative reading: it is not replaced by adoption). The
-- hosted cursors are dropped, because the served view starts empty: the next
-- run reads the vaults again from the start and rebuilds it, and no digest is
-- published from a half-known view. Folder cursors stay.
INSERT OR IGNORE INTO sync_key_confirmed (vault_id, generation)
SELECT k.vault_id, k.generation FROM vault_keys k
JOIN vaults v ON v.vault_id = k.vault_id AND v.current_generation = k.generation
WHERE k.local_origin = 1 AND k.upload_pending = 0 AND v.key_wrap = 1;

DELETE FROM sync_cursor_blobs
WHERE EXISTS (SELECT 1 FROM sync_config WHERE kind = 1);
";

/// Version 6 (M6 T5 verify follow-up, CRYPTO_SPEC 9.4): the salts this device
/// has retired (every master password change that commits or is adopted
/// locally keeps the salt it replaces; public data, no key material) and the
/// highest folder account-file version this device wrote or adopted. A
/// backend that offers a retired salt or an older version is offering a
/// rollback to an old master password. Additive: new tables only.
const V6: &str = r"
CREATE TABLE retired_salts (
    salt        BLOB    PRIMARY KEY CHECK (length(salt) = 16),
    retired_at  INTEGER NOT NULL CHECK (retired_at >= 0)
) STRICT, WITHOUT ROWID;

CREATE TABLE folder_account_version (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    version  INTEGER NOT NULL CHECK (version >= 0)
) STRICT;
";

/// Version 7 (M6 milestone review M1, CRYPTO_SPEC 10): the skip of stale view
/// digests is kept per (peer, vault) instead of per peer (it replaces the
/// per-peer `sync_peer_checkpoint.digest_ignored`, which stays only for a
/// store that wrote it before; nothing writes it any more), and a rebuild
/// remembers whether a record is still deferred and since when, so it can be
/// bounded. Additive: one table and two columns.
const V7: &str = r"
CREATE TABLE sync_digest_skip (
    device_id  BLOB NOT NULL CHECK (length(device_id) = 16),
    vault_id   BLOB NOT NULL CHECK (length(vault_id) = 16),
    PRIMARY KEY (device_id, vault_id)
) STRICT, WITHOUT ROWID;

ALTER TABLE sync_rebuild ADD COLUMN deferred INTEGER NOT NULL DEFAULT 0 CHECK (deferred IN (0, 1));
ALTER TABLE sync_rebuild ADD COLUMN deferred_since_ms INTEGER CHECK (deferred_since_ms IS NULL OR deferred_since_ms >= 0);

-- A rebuild that was open under v6 keeps its skips (per peer, for the vaults
-- being rebuilt).
INSERT OR IGNORE INTO sync_digest_skip (device_id, vault_id)
SELECT p.device_id, r.vault_id FROM sync_peer_checkpoint p, sync_rebuild r
WHERE p.digest_ignored = 1;
";

/// Version 8 (M6b T2, CRYPTO_SPEC 8.1 to 8.5, 10.2): sharing and emergency
/// access. The account's identity key (the wrapped `isk` and `ipk`, and
/// whether the server holds them), the synced contact book, the per-vault
/// `shared` flag with this account's role (monotone: a row is never removed,
/// so a backend change cannot make a shared key adoptable), the vault roster,
/// the revocations still to be carried out and the findings against the
/// server. Additive: new tables only.
const V8: &str = r"
CREATE TABLE identity_key (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    wrapped_isk  BLOB    NOT NULL CHECK (length(wrapped_isk) = 74),
    ipk          BLOB    NOT NULL CHECK (length(ipk) = 32),
    created_at   INTEGER NOT NULL CHECK (created_at >= 0),
    published    INTEGER NOT NULL CHECK (published IN (0, 1))
) STRICT;

-- Once the server holds an identity key it is never replaced (before M7), and
-- a published key never becomes unpublished.
CREATE TRIGGER identity_key_published_monotonic
BEFORE UPDATE OF published ON identity_key
WHEN OLD.published = 1 AND NEW.published = 0
BEGIN SELECT RAISE(ABORT, 'mosaic: identity key publication cannot be undone'); END;

CREATE TRIGGER identity_key_frozen_once_published
BEFORE UPDATE OF ipk ON identity_key
WHEN OLD.published = 1 AND NEW.ipk <> OLD.ipk
BEGIN SELECT RAISE(ABORT, 'mosaic: a published identity key is never replaced'); END;

CREATE TRIGGER identity_key_never_deleted
BEFORE DELETE ON identity_key
BEGIN SELECT RAISE(ABORT, 'mosaic: the identity key is never deleted'); END;

CREATE TABLE contacts_doc (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    ciphertext  BLOB    NOT NULL,
    seq         INTEGER CHECK (seq IS NULL OR seq >= 0),
    pending     INTEGER NOT NULL CHECK (pending IN (0, 1))
) STRICT;

CREATE TABLE vault_sharing (
    vault_id        BLOB    PRIMARY KEY CHECK (length(vault_id) = 16),
    shared          INTEGER NOT NULL CHECK (shared = 1),
    own_role        INTEGER NOT NULL CHECK (own_role BETWEEN 1 AND 4),
    owner_account   BLOB    NOT NULL CHECK (length(owner_account) = 16),
    access_lost_at  INTEGER CHECK (access_lost_at IS NULL OR access_lost_at >= 0)
) STRICT;

-- Like the generation floor, the flag outlives the vault and backend changes.
CREATE TRIGGER vault_sharing_never_deleted
BEFORE DELETE ON vault_sharing
BEGIN SELECT RAISE(ABORT, 'mosaic: the shared flag is never removed'); END;

CREATE TRIGGER vault_sharing_owner_immutable
BEFORE UPDATE OF owner_account ON vault_sharing
WHEN NEW.owner_account <> OLD.owner_account
BEGIN SELECT RAISE(ABORT, 'mosaic: the vault owner never changes'); END;

CREATE TABLE vault_roster (
    vault_id        BLOB    PRIMARY KEY CHECK (length(vault_id) = 16)
                            REFERENCES vaults (vault_id) ON DELETE CASCADE,
    key_generation  INTEGER NOT NULL CHECK (key_generation BETWEEN 1 AND 4294967295),
    ciphertext      BLOB    NOT NULL,
    seq             INTEGER CHECK (seq IS NULL OR seq >= 0),
    pending         INTEGER NOT NULL CHECK (pending IN (0, 1))
) STRICT;

CREATE TABLE pending_revocations (
    vault_id      BLOB    NOT NULL CHECK (length(vault_id) = 16)
                          REFERENCES vaults (vault_id) ON DELETE CASCADE,
    account_id    BLOB    NOT NULL CHECK (length(account_id) = 16),
    reason        INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 5),
    requested_at  INTEGER NOT NULL CHECK (requested_at >= 0),
    PRIMARY KEY (vault_id, account_id)
) STRICT;

CREATE TABLE sharing_findings (
    vault_id    BLOB    CHECK (vault_id IS NULL OR length(vault_id) = 16),
    kind        INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 7),
    generation  INTEGER CHECK (generation IS NULL OR generation >= 1),
    account_id  BLOB    CHECK (account_id IS NULL OR length(account_id) = 16),
    at          INTEGER NOT NULL CHECK (at >= 0)
) STRICT;
";

/// Version 9 (M7 T2, CRYPTO_SPEC 6.4 to 6.7, 10.1): key rotation. The
/// account-key epoch (public, monotone), the rotation journal (one row, never a
/// secret), the attachment re-seal queue, the pre-re-key own-segment floors of
/// the folder backend, and the `MemberRekeyed` revocation reason. The identity
/// key gets a replacement counter: a re-key is the only thing that may replace
/// a published identity key, and it raises the counter in the same statement
/// (the two v8 triggers that froze the key now allow exactly that). Strict and
/// additive apart from the rebuilt `pending_revocations` and the two
/// recreated identity triggers.
const V9: &str = r"
ALTER TABLE account ADD COLUMN ak_epoch INTEGER NOT NULL DEFAULT 1 CHECK (ak_epoch >= 1);

CREATE TRIGGER account_epoch_monotone
BEFORE UPDATE OF ak_epoch ON account
WHEN NEW.ak_epoch < OLD.ak_epoch
BEGIN SELECT RAISE(ABORT, 'mosaic: ak_epoch decreased'); END;

ALTER TABLE identity_key ADD COLUMN epoch INTEGER NOT NULL DEFAULT 1 CHECK (epoch >= 1);

-- The identity epoch follows the account epoch (review L3): it never decreases
-- and never exceeds the account's `ak_epoch`.
CREATE TRIGGER identity_key_epoch_monotone
BEFORE UPDATE OF epoch ON identity_key
WHEN NEW.epoch < OLD.epoch OR NEW.epoch > (SELECT ak_epoch FROM account WHERE id = 1)
BEGIN SELECT RAISE(ABORT, 'mosaic: identity key epoch decreased'); END;

DROP TRIGGER identity_key_published_monotonic;
CREATE TRIGGER identity_key_published_monotonic
BEFORE UPDATE OF published ON identity_key
WHEN OLD.published = 1 AND NEW.published = 0
     AND NOT (NEW.epoch > OLD.epoch AND NEW.epoch = (SELECT ak_epoch FROM account WHERE id = 1))
BEGIN SELECT RAISE(ABORT, 'mosaic: identity key publication cannot be undone'); END;

DROP TRIGGER identity_key_frozen_once_published;
CREATE TRIGGER identity_key_frozen_once_published
BEFORE UPDATE OF ipk ON identity_key
WHEN OLD.published = 1 AND NEW.ipk <> OLD.ipk
     AND NOT (NEW.epoch > OLD.epoch AND NEW.epoch = (SELECT ak_epoch FROM account WHERE id = 1))
BEGIN SELECT RAISE(ABORT, 'mosaic: a published identity key is replaced only by a re-key'); END;

-- The rotation journal: kind 1 secret key, 2 re-key, 3 profile, 4 vault; phase
-- 1 prepared, 2 backend started, 3 backend accepted, 4 folder written, 5 local
-- committed, 6 attachments, 7 uploading. No secret is ever stored here.
CREATE TABLE rotation_job (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    kind            INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 4),
    phase           INTEGER NOT NULL CHECK (phase BETWEEN 1 AND 7),
    started_at      INTEGER NOT NULL CHECK (started_at >= 0),
    from_epoch      INTEGER CHECK (from_epoch IS NULL OR from_epoch >= 1),
    to_epoch        INTEGER CHECK (to_epoch IS NULL OR to_epoch >= 1),
    target_version  INTEGER CHECK (target_version IS NULL OR target_version >= 0),
    folder_marker   BLOB    CHECK (folder_marker IS NULL OR length(folder_marker) = 93),
    done            INTEGER NOT NULL DEFAULT 0 CHECK (done >= 0),
    total           INTEGER NOT NULL DEFAULT 0 CHECK (total >= 0)
) STRICT;

-- One row per attachment to re-seal: 0 todo, 1 sealed (reserved: a step is one
-- transaction, so this state is never stored today), 2 swapped, 3 unreachable
-- (neither held nor fetchable; it keeps its old file key).
CREATE TABLE reseal_attachments (
    old_id    BLOB    PRIMARY KEY CHECK (length(old_id) = 16),
    vault_id  BLOB    NOT NULL CHECK (length(vault_id) = 16),
    item_id   BLOB    NOT NULL CHECK (length(item_id) = 16),
    new_id    BLOB    CHECK (new_id IS NULL OR length(new_id) = 16),
    state     INTEGER NOT NULL CHECK (state BETWEEN 0 AND 3)
) STRICT, WITHOUT ROWID;

-- The folder backend: this device's own segments up to `floor_seq` were sealed
-- under the previous epoch. They are removed only after a complete snapshot set
-- of the new epoch exists and the retention time has passed.
CREATE TABLE folder_rekey_floor (
    vault_id       BLOB    PRIMARY KEY CHECK (length(vault_id) = 16),
    floor_seq      INTEGER NOT NULL CHECK (floor_seq >= 0),
    rekey_at       INTEGER NOT NULL CHECK (rekey_at >= 0),
    snapshot_done  INTEGER NOT NULL DEFAULT 0 CHECK (snapshot_done IN (0, 1))
) STRICT, WITHOUT ROWID;

-- Commitments of Secret Keys this device replaced (CRYPTO_SPEC 6.7):
-- SHA-256(ctx(secret-key/retired) || 0x00 || account_id || SK). Never deleted.
-- Adoption refuses a typed Secret Key that matches one of them.
CREATE TABLE retired_secret_keys (
    commitment  BLOB    PRIMARY KEY CHECK (length(commitment) = 32),
    retired_at  INTEGER NOT NULL CHECK (retired_at >= 0)
) STRICT, WITHOUT ROWID;

CREATE TRIGGER retired_secret_keys_permanent
BEFORE DELETE ON retired_secret_keys
BEGIN SELECT RAISE(ABORT, 'mosaic: retired Secret Key commitments are permanent'); END;

-- Unverifiable higher-epoch account files the user dismissed (by path and
-- SHA-256): they never raise a finding again (CRYPTO_SPEC 10.1).
CREATE TABLE rekey_dismissed_files (
    path          TEXT    NOT NULL CHECK (length(path) <= 4096),
    sha256        BLOB    NOT NULL CHECK (length(sha256) = 32),
    dismissed_at  INTEGER NOT NULL CHECK (dismissed_at >= 0),
    PRIMARY KEY (path, sha256)
) STRICT, WITHOUT ROWID;

-- A verified re-key marker names this device's epoch (CRYPTO_SPEC 10.1): the
-- account was re-keyed elsewhere. A flag of its own, so it is not a halt that
-- an acknowledgement clears; adopting the re-key clears it.
ALTER TABLE sync_engine_state ADD COLUMN rekey_pending INTEGER NOT NULL DEFAULT 0
    CHECK (rekey_pending IN (0, 1));

-- Vaults an adopted re-key left at the generation this device held (review M5):
-- while the vault's current generation is not above `generation`, the device
-- does not push records of it (they would be sealed under the key the re-key
-- retired). The genuine `g + 1` key arrives by pull and ends the hold.
CREATE TABLE rekey_hold (
    vault_id    BLOB    PRIMARY KEY CHECK (length(vault_id) = 16),
    generation  INTEGER NOT NULL CHECK (generation BETWEEN 1 AND 4294967295)
) STRICT, WITHOUT ROWID;

-- Whether a backend has ever seen the vault (ADR-0064): set when the vault is
-- learned from a backend or created there, never cleared (a change of backend
-- resets `remote_created`, but the old backend keeps the vault). Only such a
-- vault is held by an adoption. A vault that exists when this version is
-- applied counts as seen (the conservative default).
ALTER TABLE vaults ADD COLUMN backend_known INTEGER NOT NULL DEFAULT 1
    CHECK (backend_known IN (0, 1));

CREATE TRIGGER vaults_backend_known_permanent
BEFORE UPDATE OF backend_known ON vaults
WHEN OLD.backend_known = 1 AND NEW.backend_known = 0
BEGIN SELECT RAISE(ABORT, 'mosaic: a vault a backend has seen stays seen'); END;

-- pending_revocations: reason 6 is MemberRekeyed (nobody is revoked).
CREATE TABLE pending_revocations_v9 (
    vault_id      BLOB    NOT NULL CHECK (length(vault_id) = 16)
                          REFERENCES vaults (vault_id) ON DELETE CASCADE,
    account_id    BLOB    NOT NULL CHECK (length(account_id) = 16),
    reason        INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 6),
    requested_at  INTEGER NOT NULL CHECK (requested_at >= 0),
    PRIMARY KEY (vault_id, account_id)
) STRICT;
INSERT INTO pending_revocations_v9 (vault_id, account_id, reason, requested_at)
SELECT vault_id, account_id, reason, requested_at FROM pending_revocations;
DROP TABLE pending_revocations;
ALTER TABLE pending_revocations_v9 RENAME TO pending_revocations;
";

/// Version 10: credential rotation (M7b, CRYPTO_SPEC §11): the breach check's
/// cache, the scan summary and the rotate-now queue's snooze and dismissal.
/// Only IDs, small integers and Lamport value marks: no password, no hash and
/// nothing derived from a password. Rows go with the item (a purged or
/// tombstoned record, here by trigger) and with the vault (cascade);
/// `reset_sync_state` and a re-key leave them alone.
const V10: &str = r"
CREATE TABLE breach_state (
    vault_id    BLOB    NOT NULL CHECK (length(vault_id) = 16)
                        REFERENCES vaults (vault_id) ON DELETE CASCADE,
    item_id     BLOB    NOT NULL CHECK (length(item_id) = 16),
    field       INTEGER NOT NULL CHECK (field IN (1, 2)),
    value_mark  BLOB    NOT NULL CHECK (length(value_mark) BETWEEN 1 AND 32),
    checked_at  INTEGER NOT NULL,
    found       INTEGER NOT NULL CHECK (found IN (0, 1)),
    PRIMARY KEY (vault_id, item_id, field)
) STRICT;

CREATE TABLE breach_scan (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    started_at   INTEGER,
    finished_at  INTEGER,
    checked      INTEGER NOT NULL DEFAULT 0 CHECK (checked >= 0),
    error        INTEGER CHECK (error IS NULL OR error BETWEEN 1 AND 4)
) STRICT;

CREATE TABLE rotation_queue_state (
    vault_id        BLOB    NOT NULL CHECK (length(vault_id) = 16)
                            REFERENCES vaults (vault_id) ON DELETE CASCADE,
    item_id         BLOB    NOT NULL CHECK (length(item_id) = 16),
    snoozed_until   INTEGER,
    dismissed_mark  BLOB    CHECK (dismissed_mark IS NULL OR length(dismissed_mark) BETWEEN 1 AND 32),
    PRIMARY KEY (vault_id, item_id)
) STRICT;

-- A record that is deleted or becomes a tombstone (a purge here or on another
-- device) takes the item's rows with it.
CREATE TRIGGER records_rotation_rows_delete
AFTER DELETE ON records
BEGIN
    DELETE FROM breach_state WHERE vault_id = OLD.vault_id AND item_id = OLD.record_id;
    DELETE FROM rotation_queue_state WHERE vault_id = OLD.vault_id AND item_id = OLD.record_id;
END;

CREATE TRIGGER records_rotation_rows_tombstone_insert
AFTER INSERT ON records
WHEN NEW.kind = 2
BEGIN
    DELETE FROM breach_state WHERE vault_id = NEW.vault_id AND item_id = NEW.record_id;
    DELETE FROM rotation_queue_state WHERE vault_id = NEW.vault_id AND item_id = NEW.record_id;
END;

CREATE TRIGGER records_rotation_rows_tombstone_update
AFTER UPDATE ON records
WHEN NEW.kind = 2
BEGIN
    DELETE FROM breach_state WHERE vault_id = NEW.vault_id AND item_id = NEW.record_id;
    DELETE FROM rotation_queue_state WHERE vault_id = NEW.vault_id AND item_id = NEW.record_id;
END;
";

/// Version 11: the boot marker (M10, CRYPTO_SPEC §6.9.2, ADR-0090): the boot
/// ID of the device boot in which the master password last unlocked, and the
/// time it was written. One row, written only on iOS and Android. It is
/// cleartext, not secret and not derived from a secret. Next to it, the count
/// of wrong re-authentications (`reauth_failures`).
const V11: &str = r"
CREATE TABLE boot_marker (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    boot_id        BLOB    NOT NULL CHECK (length(boot_id) BETWEEN 1 AND 32),
    recorded_at_ms INTEGER NOT NULL
) STRICT;

-- The wrong master passwords in a row at a re-authentication on an unlocked
-- phone (CRYPTO_SPEC §6.9.2, N2): kept across processes so that a restart
-- does not start the count again. One row, local-only, never synced. A right
-- password deletes it; the boot marker's deletion does not.
CREATE TABLE reauth_failures (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    failures INTEGER NOT NULL CHECK (failures BETWEEN 0 AND 255)
) STRICT;
";

/// Version 12: the view-comparison bookkeeping gets a column of its own for
/// this device's checkpoint counter (`own_counter`). Until now the table packed
/// both counters into `counter` (peer in the low 32 bits, own in the high 32).
/// The migration unpacks every existing row: `counter` keeps the peer half
/// and `own_counter` takes the high half (0 for a row written before the
/// packing), so no row is lost or misread.
const V12: &str = r"
ALTER TABLE sync_view_compare ADD COLUMN own_counter INTEGER NOT NULL DEFAULT 0
    CHECK (own_counter >= 0);
UPDATE sync_view_compare
   SET own_counter = counter >> 32,
       counter = counter & 4294967295;
";

/// All migrations, in order. Version `n` is at index `n - 1`.
pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: V1,
    },
    Migration {
        version: 2,
        sql: V2,
    },
    Migration {
        version: 3,
        sql: V3,
    },
    Migration {
        version: 4,
        sql: V4,
    },
    Migration {
        version: 5,
        sql: V5,
    },
    Migration {
        version: 6,
        sql: V6,
    },
    Migration {
        version: 7,
        sql: V7,
    },
    Migration {
        version: 8,
        sql: V8,
    },
    Migration {
        version: 9,
        sql: V9,
    },
    Migration {
        version: 10,
        sql: V10,
    },
    Migration {
        version: 11,
        sql: V11,
    },
    Migration {
        version: 12,
        sql: V12,
    },
];

/// The newest schema version this build knows.
pub const LATEST_VERSION: u32 = 12;

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

    const V6_TEST: &str = "
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
                sql: V2,
            },
            Migration {
                version: 3,
                sql: V3,
            },
            Migration {
                version: 4,
                sql: V4,
            },
            Migration {
                version: 5,
                sql: V5,
            },
            Migration {
                version: 6,
                sql: V6,
            },
            Migration {
                version: 7,
                sql: V7,
            },
            Migration {
                version: 8,
                sql: V8,
            },
            Migration {
                version: 9,
                sql: V9,
            },
            Migration {
                version: 10,
                sql: V10,
            },
            Migration {
                version: 11,
                sql: V11,
            },
            Migration {
                version: 12,
                sql: V12,
            },
            Migration {
                version: 13,
                sql: V6_TEST,
            },
        ];
        assert_eq!(migrate(&mut c, &upgraded).unwrap(), 13);
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
                found: 13,
                supported: 12
            })
        ));
        assert_eq!(user_version(&c).unwrap(), 13);
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
                sql: V2,
            },
            Migration {
                version: 3,
                sql: V3,
            },
            Migration {
                version: 4,
                sql: V4,
            },
            Migration {
                version: 5,
                sql: V5,
            },
            Migration {
                version: 6,
                sql: V6,
            },
            Migration {
                version: 7,
                sql: V7,
            },
            Migration {
                version: 8,
                sql: V8,
            },
            Migration {
                version: 9,
                sql: V9,
            },
            Migration {
                version: 10,
                sql: V10,
            },
            Migration {
                version: 11,
                sql: V11,
            },
            Migration {
                version: 12,
                sql: V12,
            },
            Migration {
                version: 13,
                sql: "CREATE TABLE half_done (x INTEGER); THIS IS NOT SQL;",
            },
        ];
        assert!(matches!(migrate(&mut c, &broken), Err(Error::Database(_))));
        assert_eq!(user_version(&c).unwrap(), 12);
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'half_done'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    /// v11 to v12: a row written with the two counters packed into `counter`
    /// (and one written before the packing) is unpacked, not lost or misread.
    #[test]
    fn migration_v11_to_v12_unpacks_the_view_compare_counters() {
        let mut c = conn();
        migrate(&mut c, &MIGRATIONS[..11]).unwrap();
        let packed: i64 = (9_i64 << 32) | 7;
        for (device, since, counter) in [
            (1_u8, Some(50_i64), packed),
            (2, None, 4),
            (3, Some(60), 4_294_967_295),
        ] {
            c.execute(
                "INSERT INTO sync_view_compare (device_id, vault_id, since_ms, counter)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![[device; 16], [8u8; 16], since, counter],
            )
            .unwrap();
        }
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
        let rows: Vec<(u8, Option<i64>, i64, i64)> = c
            .prepare(
                "SELECT device_id, since_ms, counter, own_counter
                 FROM sync_view_compare ORDER BY device_id",
            )
            .unwrap()
            .query_map([], |r| {
                let d: Vec<u8> = r.get(0)?;
                Ok((d[0], r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            rows,
            vec![
                (1, Some(50), 7, 9),
                (2, None, 4, 0),
                (3, Some(60), 4_294_967_295, 0)
            ]
        );
    }

    #[test]
    fn gap_in_migrations_is_refused() {
        let mut c = conn();
        let gappy = [Migration {
            version: 2,
            sql: V6_TEST,
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

    fn check_generation_triggers(c: &Connection, v: [u8; 16]) {
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
    }

    fn check_floor_triggers(c: &Connection, v: [u8; 16]) {
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
    }

    fn check_account_and_cursor_triggers(c: &Connection, v: [u8; 16]) {
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

    #[test]
    fn triggers_block_rollbacks_even_through_raw_sql() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        let v = [9u8; 16];
        check_generation_triggers(&c, v);
        check_floor_triggers(&c, v);
        check_account_and_cursor_triggers(&c, v);
    }

    /// A v2 store with a vault, a key, a record and an attachment row is
    /// upgraded to v3: every row survives with neutral sync flags and the
    /// new tables exist (the install ID is created by `Store::open`).
    #[test]
    fn migration_v2_to_v3_keeps_data() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..2]).unwrap(), 2);
        let v = [7u8; 16];
        c.execute_batch(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, meta, meta_generation, updated_at)
             VALUES (x'07070707070707070707070707070707', 1, 2, 2, x'aa', 2, 5);
             INSERT INTO vault_keys (vault_id, generation, wrapped_vk, updated_at)
             VALUES (x'07070707070707070707070707070707', 2, zeroblob(74), 6);
             INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext,
                                  server_seq, outbox_seq, updated_at)
             VALUES (x'07070707070707070707070707070707', x'09090909090909090909090909090909',
                     1, 2, x'bb', 11, 3, 7);
             INSERT INTO attachments (attachment_id, vault_id, record_id, size,
                                      upload_pending, created_at)
             VALUES (x'0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a', x'07070707070707070707070707070707',
                     x'09090909090909090909090909090909', 100, 1, 8);
             INSERT INTO sync_cursors (vault_id, server_seq, updated_at)
             VALUES (x'07070707070707070707070707070707', 11, 9);",
        )
        .unwrap();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..3]).unwrap(), 3);
        let flags: (i64, i64, i64, i64, Option<i64>, Vec<u8>, i64) = c
            .query_row(
                "SELECT k.upload_pending, k.local_origin, v.remote_created,
                        v.meta_upload_pending, v.meta_server_seq, v.meta, v.updated_at
                 FROM vaults v JOIN vault_keys k USING (vault_id) WHERE v.vault_id = ?1",
                [v],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(flags, (0, 0, 0, 0, None, vec![0xAA], 5));
        let record: (i64, i64, Vec<u8>) = c
            .query_row(
                "SELECT server_seq, outbox_seq, ciphertext FROM records",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(record, (11, 3, vec![0xBB]));
        for (table, expected) in [
            ("attachments", 1),
            ("sync_cursors", 1),
            ("sync_checkpoint", 1),
            ("install_identity", 0),
            ("sync_config", 0),
            ("sync_cursor_blobs", 0),
            ("sync_local_seq", 0),
            ("attachment_deletions", 0),
        ] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, expected, "{table}");
        }
        assert!(
            c.execute("UPDATE vault_keys SET upload_pending = 2", [])
                .is_err(),
            "the new flag columns are checked"
        );
    }

    /// A v3 store with sync data is upgraded to v4: the data survives, the
    /// new tables are empty (the engine state has its one neutral row) and
    /// their checks hold.
    #[test]
    fn migration_v3_to_v4_keeps_data_and_adds_neutral_state() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..3]).unwrap(), 3);
        c.execute_batch(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (x'07070707070707070707070707070707', 1, 1, 1, 5);
             INSERT INTO sync_cursor_blobs (vault_id, cursor, updated_at)
             VALUES (x'07070707070707070707070707070707', x'0102', 6);
             INSERT INTO sync_local_seq (vault_id, next_seq)
             VALUES (x'07070707070707070707070707070707', 1025);
             UPDATE sync_checkpoint SET counter = 9;",
        )
        .unwrap();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..4]).unwrap(), 4);
        let kept: (Vec<u8>, i64, i64) = c
            .query_row(
                "SELECT c.cursor, l.next_seq, k.counter
                 FROM sync_cursor_blobs c, sync_local_seq l, sync_checkpoint k",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(kept, (vec![1, 2], 1025, 9));
        for table in [
            "sync_own_seq",
            "sync_peer_checkpoint",
            "sync_lag",
            "sync_key_candidate",
            "sync_marker_seen",
        ] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table}");
        }
        let state: (i64, i64, i64, Option<i64>, Option<i64>) = c
            .query_row(
                "SELECT halt, last_published_counter, strike_count, strike_first_ms,
                        strike_counter
                 FROM sync_engine_state WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(state, (0, 0, 0, None, None));
        for bad in [
            "UPDATE sync_engine_state SET halt = 3",
            "INSERT INTO sync_own_seq (vault_id) VALUES (x'01')",
            "INSERT INTO sync_peer_checkpoint (device_id, counter, issued_at_ms, entries)
             VALUES (zeroblob(16), 1, 1, zeroblob(27))",
            "INSERT INTO sync_key_candidate (vault_id, generation, wrapped, device_id)
             VALUES (zeroblob(16), 0, x'01', zeroblob(16))",
        ] {
            assert!(c.execute(bad, []).is_err(), "{bad}");
        }
    }

    /// A v4 store is upgraded to v5: the peer checkpoints are recreated for
    /// the longer entries (empty, to be fetched again), the new tables are
    /// empty and their checks hold, the other data survives.
    #[test]
    fn migration_v4_to_v5_recreates_peer_checkpoints_and_adds_the_served_view() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..4]).unwrap(), 4);
        c.execute_batch(
            "UPDATE sync_checkpoint SET counter = 9;
             INSERT INTO sync_peer_checkpoint (device_id, counter, issued_at_ms, entries)
             VALUES (zeroblob(16), 3, 1, zeroblob(28));",
        )
        .unwrap();
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
        let counter: i64 = c
            .query_row("SELECT counter FROM sync_checkpoint", [], |r| r.get(0))
            .unwrap();
        assert_eq!(counter, 9);
        for table in [
            "sync_peer_checkpoint",
            "sync_served",
            "sync_key_confirmed",
            "sync_vault_flag",
            "sync_rotation_log",
        ] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table}");
        }
        c.execute(
            "INSERT INTO sync_peer_checkpoint (device_id, counter, issued_at_ms, entries)
             VALUES (zeroblob(16), 1, 1, zeroblob(120))",
            [],
        )
        .unwrap();
        for bad in [
            "INSERT INTO sync_peer_checkpoint (device_id, counter, issued_at_ms, entries)
             VALUES (x'01010101010101010101010101010101', 1, 1, zeroblob(28))",
            "INSERT INTO sync_served (vault_id, record_id, seq, ct_hash)
             VALUES (zeroblob(16), zeroblob(16), 1, zeroblob(31))",
            "INSERT INTO sync_served (vault_id, record_id, seq, ct_hash)
             VALUES (zeroblob(16), zeroblob(16), -1, zeroblob(32))",
            "INSERT INTO sync_key_confirmed (vault_id, generation) VALUES (zeroblob(16), 0)",
            "INSERT INTO sync_vault_flag (vault_id, kind) VALUES (zeroblob(16), 3)",
            "INSERT INTO sync_rotation_log (vault_id, at_ms) VALUES (zeroblob(15), 1)",
        ] {
            assert!(c.execute(bad, []).is_err(), "{bad}");
        }
    }

    /// R-L3 and R-L6. The v5 migration seeds the confirmed mark for a key this
    /// install generated whose upload flag is clear (the conservative
    /// reading) and drops hosted cursors (the served view starts empty);
    /// folder cursors stay.
    #[test]
    fn migration_v5_seeds_confirmed_keys_and_drops_hosted_cursors() {
        let setup = |kind_sql: &str| {
            let mut c = conn();
            migrate(&mut c, &MIGRATIONS[..4]).unwrap();
            c.execute_batch(
                "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                     highest_accepted_generation, updated_at)
                 VALUES (x'07070707070707070707070707070707', 1, 1, 1, 5),
                        (x'08080808080808080808080808080808', 1, 1, 1, 5);
                 INSERT INTO vault_keys (vault_id, generation, wrapped_vk, updated_at,
                                         upload_pending, local_origin)
                 VALUES (x'07070707070707070707070707070707', 1, zeroblob(74), 1, 0, 1),
                        (x'08080808080808080808080808080808', 1, zeroblob(74), 1, 1, 1);
                 INSERT INTO sync_cursor_blobs (vault_id, cursor, updated_at)
                 VALUES (x'07070707070707070707070707070707', x'0102', 6),
                        (x'08080808080808080808080808080808', x'0304', 6);",
            )
            .unwrap();
            c.execute_batch(kind_sql).unwrap();
            assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
            c
        };
        let hosted = setup(
            "INSERT INTO sync_config (id, kind, server_url, device_name, updated_at)
             VALUES (1, 1, 'https://x.example', 'Laptop', 0);",
        );
        let confirmed: Vec<Vec<u8>> = hosted
            .prepare("SELECT vault_id FROM sync_key_confirmed")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(confirmed, vec![vec![7u8; 16]], "only the uploaded key");
        let cursors: i64 = hosted
            .query_row("SELECT count(*) FROM sync_cursor_blobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cursors, 0, "hosted cursors are dropped");

        let folder = setup(
            "INSERT INTO sync_config (id, kind, folder_path, device_name, updated_at)
             VALUES (1, 2, '/tmp/f', 'Laptop', 0);",
        );
        let cursors: i64 = folder
            .query_row("SELECT count(*) FROM sync_cursor_blobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cursors, 2, "folder cursors stay");
    }

    #[test]
    fn checkpoint_counter_monotonic_trigger() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        c.execute("UPDATE sync_checkpoint SET counter = 5", [])
            .unwrap();
        c.execute("UPDATE sync_checkpoint SET counter = 5", [])
            .unwrap();
        let e = c
            .execute("UPDATE sync_checkpoint SET counter = 4", [])
            .unwrap_err();
        assert!(e.to_string().contains("mosaic:"), "{e}");
        let n: i64 = c
            .query_row("SELECT counter FROM sync_checkpoint", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 5);
    }

    #[test]
    fn sync_config_check_enforces_the_backend_shape() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        for ok in [
            "INSERT INTO sync_config (id, kind, server_url, email, device_name, updated_at)
             VALUES (1, 1, 'https://x.example', NULL, 'Laptop', 0)",
            "UPDATE sync_config SET kind = 2, server_url = NULL, email = NULL,
                 folder_path = '/tmp/f'",
        ] {
            c.execute(ok, []).unwrap();
        }
        for bad in [
            "UPDATE sync_config SET kind = 1",
            "UPDATE sync_config SET email = 'a@b.example'",
            "UPDATE sync_config SET kind = 3",
            "UPDATE sync_config SET device_name = ''",
        ] {
            assert!(c.execute(bad, []).is_err(), "{bad}");
        }
    }
    #[test]
    fn migration_v7_to_v8() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..7]).unwrap(), 7);
        // Data of the older schema survives and gets no sharing state.
        c.execute(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (x'01010101010101010101010101010101', 1, 1, 1, 0)",
            [],
        )
        .unwrap();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..8]).unwrap(), 8);
        let vaults: i64 = c
            .query_row("SELECT count(*) FROM vaults", [], |r| r.get(0))
            .unwrap();
        assert_eq!(vaults, 1);
        for table in [
            "identity_key",
            "contacts_doc",
            "vault_sharing",
            "vault_roster",
            "pending_revocations",
            "sharing_findings",
        ] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} starts empty");
        }
    }

    #[test]
    fn sharing_triggers_keep_flags_monotone() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        c.execute(
            "INSERT INTO identity_key (id, wrapped_isk, ipk, created_at, published)
             VALUES (1, zeroblob(74), zeroblob(32), 0, 1)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO vault_sharing (vault_id, shared, own_role, owner_account)
             VALUES (zeroblob(16), 1, 1, zeroblob(16))",
            [],
        )
        .unwrap();
        for sql in [
            "UPDATE identity_key SET published = 0",
            "UPDATE identity_key SET ipk = x'0101010101010101010101010101010101010101010101010101010101010101'",
            "DELETE FROM identity_key",
            "DELETE FROM vault_sharing",
            "UPDATE vault_sharing SET owner_account = x'01010101010101010101010101010101'",
            "UPDATE vault_sharing SET shared = 0",
        ] {
            let e = c.execute(sql, []).unwrap_err();
            assert!(
                e.to_string().contains("mosaic:") || e.to_string().contains("CHECK"),
                "{sql}: {e}"
            );
        }
        // Re-wrapping the key (a re-key) and changing the role stay possible.
        c.execute("UPDATE identity_key SET wrapped_isk = zeroblob(74)", [])
            .unwrap();
        c.execute("UPDATE vault_sharing SET own_role = 3", [])
            .unwrap();
    }
    #[test]
    fn migration_v8_to_v9() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..8]).unwrap(), 8);
        c.execute_batch(
            "INSERT INTO account (id, account_id, salt, profile, updated_at)
                 VALUES (1, x'01010101010101010101010101010101', zeroblob(16), 2, 0);
             INSERT INTO identity_key (id, wrapped_isk, ipk, created_at, published)
                 VALUES (1, zeroblob(74), zeroblob(32), 0, 1);
             INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
                 VALUES (x'02020202020202020202020202020202', 1, 1, 1, 0);
             INSERT INTO pending_revocations (vault_id, account_id, reason, requested_at)
                 VALUES (x'02020202020202020202020202020202',
                         x'03030303030303030303030303030303', 5, 7);",
        )
        .unwrap();
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
        assert_eq!(user_version(&c).unwrap(), LATEST_VERSION);
        // Existing rows survive and start at epoch 1.
        let epoch: i64 = c
            .query_row("SELECT ak_epoch FROM account", [], |r| r.get(0))
            .unwrap();
        assert_eq!(epoch, 1);
        let id_epoch: i64 = c
            .query_row("SELECT epoch FROM identity_key", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id_epoch, 1);
        let (reason, at): (i64, i64) = c
            .query_row(
                "SELECT reason, requested_at FROM pending_revocations",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((reason, at), (5, 7));
        for table in ["rotation_job", "reseal_attachments", "folder_rekey_floor"] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} starts empty");
        }
        // The rebuilt table takes reason 6 and refuses 7.
        c.execute(
            "INSERT INTO pending_revocations (vault_id, account_id, reason, requested_at)
             VALUES (x'02020202020202020202020202020202', x'04040404040404040404040404040404', 6, 0)",
            [],
        )
        .unwrap();
        assert!(
            c.execute(
                "INSERT INTO pending_revocations (vault_id, account_id, reason, requested_at)
                 VALUES (x'02020202020202020202020202020202', x'05050505050505050505050505050505', 7, 0)",
                [],
            )
            .is_err()
        );
        // The vault delete still cascades into the rebuilt table.
        c.execute("DELETE FROM vaults", []).unwrap_err();
    }

    #[test]
    fn migration_v9_to_v10() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..9]).unwrap(), 9);
        c.execute_batch(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
                 VALUES (x'02020202020202020202020202020202', 1, 1, 1, 5);
             INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext, updated_at)
                 VALUES (x'02020202020202020202020202020202',
                         x'0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a', 1, 1, x'0102', 6);",
        )
        .unwrap();
        assert_eq!(migrate(&mut c, MIGRATIONS).unwrap(), LATEST_VERSION);
        assert_eq!(user_version(&c).unwrap(), LATEST_VERSION);
        // Existing data is kept, the new tables start empty.
        let (updated, record_updated): (i64, i64) = c
            .query_row(
                "SELECT (SELECT updated_at FROM vaults), (SELECT updated_at FROM records)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((updated, record_updated), (5, 6));
        for table in ["breach_state", "breach_scan", "rotation_queue_state"] {
            let n: i64 = c
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} starts empty");
        }
        // The CHECKs hold: a field outside 1..2, an empty and an oversized mark.
        let bad = |field: i64, mark_len: i64| {
            c.execute(
                "INSERT INTO breach_state (vault_id, item_id, field, value_mark, checked_at, found)
                 VALUES (x'02020202020202020202020202020202', x'0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a',
                         ?1, zeroblob(?2), 0, 1)",
                [field, mark_len],
            )
        };
        assert!(bad(3, 24).is_err());
        assert!(bad(1, 0).is_err());
        assert!(bad(1, 33).is_err());
        bad(1, 24).unwrap();
        // The record's deletion removes the item's rows.
        c.execute("DELETE FROM records", []).unwrap();
        let n: i64 = c
            .query_row("SELECT count(*) FROM breach_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn migration_v10_to_v11() {
        let mut c = conn();
        assert_eq!(migrate(&mut c, &MIGRATIONS[..10]).unwrap(), 10);
        assert_eq!(migrate(&mut c, &MIGRATIONS[..11]).unwrap(), 11);
        assert_eq!(user_version(&c).unwrap(), 11);
        let n: i64 = c
            .query_row("SELECT count(*) FROM boot_marker", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "the marker starts absent");
        // One row, an id of 1, a boot ID of 1 to 32 bytes.
        for sql in [
            "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (2, x'01', 0)",
            "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (1, x'', 0)",
            "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (1, zeroblob(33), 0)",
        ] {
            assert!(c.execute(sql, []).is_err(), "{sql}");
        }
        c.execute(
            "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (1, zeroblob(32), 0)",
            [],
        )
        .unwrap();
        assert!(
            c.execute(
                "INSERT INTO boot_marker (id, boot_id, recorded_at_ms) VALUES (1, x'01', 0)",
                [],
            )
            .is_err()
        );
        // The re-authentication counter: absent at the start, one row, 0 to 255.
        let n: i64 = c
            .query_row("SELECT count(*) FROM reauth_failures", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "the counter starts absent");
        for sql in [
            "INSERT INTO reauth_failures (id, failures) VALUES (2, 1)",
            "INSERT INTO reauth_failures (id, failures) VALUES (1, -1)",
            "INSERT INTO reauth_failures (id, failures) VALUES (1, 256)",
        ] {
            assert!(c.execute(sql, []).is_err(), "{sql}");
        }
        c.execute(
            "INSERT INTO reauth_failures (id, failures) VALUES (1, 255)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn epoch_triggers_refuse_decrease_and_freeze_identity() {
        let mut c = conn();
        migrate(&mut c, MIGRATIONS).unwrap();
        c.execute_batch(
            "INSERT INTO account (id, account_id, salt, profile, updated_at)
                 VALUES (1, x'01010101010101010101010101010101', zeroblob(16), 2, 0);
             INSERT INTO identity_key (id, wrapped_isk, ipk, created_at, published)
                 VALUES (1, zeroblob(74), zeroblob(32), 0, 1);",
        )
        .unwrap();
        c.execute("UPDATE account SET ak_epoch = 2", []).unwrap();
        for sql in [
            "UPDATE account SET ak_epoch = 1",
            "UPDATE account SET ak_epoch = 0",
            "UPDATE identity_key SET published = 0",
            "UPDATE identity_key SET ipk = x'0101010101010101010101010101010101010101010101010101010101010101'",
            // L3: the identity epoch never runs ahead of the account's.
            "UPDATE identity_key SET epoch = 3",
            "UPDATE identity_key SET published = 0, epoch = 3",
            "UPDATE identity_key SET ipk = x'0101010101010101010101010101010101010101010101010101010101010101', epoch = 3",
        ] {
            assert!(c.execute(sql, []).is_err(), "{sql}");
        }
        // The replacement counter must rise with the key.
        c.execute(
            "UPDATE identity_key SET ipk = x'0101010101010101010101010101010101010101010101010101010101010101',
                 published = 0, epoch = epoch + 1",
            [],
        )
        .unwrap();
        assert!(c.execute("UPDATE identity_key SET epoch = 1", []).is_err());
        // The job table only takes its kinds, phases and a 93-byte marker.
        for sql in [
            "INSERT INTO rotation_job (id, kind, phase, started_at) VALUES (1, 5, 1, 0)",
            "INSERT INTO rotation_job (id, kind, phase, started_at) VALUES (1, 1, 8, 0)",
            "INSERT INTO rotation_job (id, kind, phase, started_at) VALUES (2, 1, 1, 0)",
            "INSERT INTO rotation_job (id, kind, phase, started_at, folder_marker)
             VALUES (1, 2, 1, 0, zeroblob(92))",
        ] {
            assert!(c.execute(sql, []).is_err(), "{sql}");
        }
        c.execute(
            "INSERT INTO rotation_job (id, kind, phase, started_at, folder_marker)
             VALUES (1, 2, 1, 0, zeroblob(93))",
            [],
        )
        .unwrap();
    }
}
