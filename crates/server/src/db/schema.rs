//! The server schema (M6 spec §6.4) and its own migration list.
//!
//! Nothing here stores an IP address, a request body or any plaintext of a
//! vault (CRYPTO_SPEC §12). Tables are STRICT; foreign keys cascade so that
//! deleting an account removes everything it owns.

/// Migration `n` (zero-based) upgrades `user_version` `n` to `n + 1`.
pub const MIGRATIONS: &[&str] = &[
    // v1
    "
CREATE TABLE accounts (
    account_id BLOB PRIMARY KEY CHECK (length(account_id) = 16),
    email TEXT NOT NULL UNIQUE CHECK (length(email) BETWEEN 3 AND 320),
    salt BLOB NOT NULL CHECK (length(salt) = 16),
    profile INTEGER NOT NULL CHECK (profile BETWEEN 1 AND 4),
    opaque_record BLOB NOT NULL CHECK (length(opaque_record) <= 4096),
    wrapped_ak BLOB NOT NULL CHECK (length(wrapped_ak) = 74),
    account_version INTEGER NOT NULL CHECK (account_version >= 1),
    totp_sealed BLOB CHECK (totp_sealed IS NULL OR length(totp_sealed) <= 256),
    totp_pending_sealed BLOB CHECK (totp_pending_sealed IS NULL OR length(totp_pending_sealed) <= 256),
    totp_last_step INTEGER,
    totp_fail_count INTEGER NOT NULL DEFAULT 0,
    totp_fail_window INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE devices (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    device_id BLOB NOT NULL CHECK (length(device_id) = 16),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 256),
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    revoked_at INTEGER,
    PRIMARY KEY (account_id, device_id)
) STRICT;

CREATE TABLE sessions (
    token_hash BLOB PRIMARY KEY CHECK (length(token_hash) = 32),
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    device_id BLOB NOT NULL CHECK (length(device_id) = 16),
    created_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    fresh_until INTEGER NOT NULL,
    pending_2fa INTEGER NOT NULL CHECK (pending_2fa IN (0, 1)),
    totp_attempts INTEGER NOT NULL DEFAULT 0,
    account_version INTEGER NOT NULL CHECK (account_version >= 1)
) STRICT;
CREATE INDEX sessions_account ON sessions (account_id, device_id);
CREATE INDEX sessions_expiry ON sessions (expires_at);

CREATE TABLE vaults (
    vault_id BLOB PRIMARY KEY CHECK (length(vault_id) = 16),
    owner_account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    current_generation INTEGER NOT NULL CHECK (current_generation >= 1),
    head_seq INTEGER NOT NULL CHECK (head_seq >= 0),
    created_at INTEGER NOT NULL
) STRICT;
CREATE INDEX vaults_owner ON vaults (owner_account_id);

CREATE TABLE memberships (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    role INTEGER NOT NULL CHECK (role = 1),
    PRIMARY KEY (account_id, vault_id)
) STRICT;

CREATE TABLE vault_keys (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    generation INTEGER NOT NULL CHECK (generation >= 1),
    wrapped BLOB NOT NULL CHECK (length(wrapped) = 74),
    device_id BLOB NOT NULL CHECK (length(device_id) = 16),
    server_seq INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (vault_id, account_id, generation)
) STRICT;

CREATE TABLE vault_meta (
    vault_id BLOB PRIMARY KEY REFERENCES vaults (vault_id) ON DELETE CASCADE,
    key_generation INTEGER NOT NULL CHECK (key_generation >= 1),
    ciphertext BLOB NOT NULL CHECK (length(ciphertext) <= 65600),
    server_seq INTEGER NOT NULL
) STRICT;

CREATE TABLE records (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    record_id BLOB NOT NULL CHECK (length(record_id) = 16),
    kind INTEGER NOT NULL CHECK (kind IN (1, 2)),
    key_generation INTEGER NOT NULL CHECK (key_generation >= 1),
    ciphertext BLOB NOT NULL,
    server_seq INTEGER NOT NULL,
    uploaded_at INTEGER NOT NULL,
    PRIMARY KEY (vault_id, record_id)
) STRICT;
CREATE INDEX records_seq ON records (vault_id, server_seq);

CREATE TABLE attachments (
    attachment_id BLOB PRIMARY KEY CHECK (length(attachment_id) = 16),
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    size INTEGER NOT NULL CHECK (size >= 0),
    created_at INTEGER NOT NULL,
    deleted_at INTEGER
) STRICT;
CREATE INDEX attachments_vault ON attachments (vault_id);

CREATE TABLE checkpoints (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    device_id BLOB NOT NULL CHECK (length(device_id) = 16),
    counter INTEGER NOT NULL CHECK (counter >= 0),
    body BLOB NOT NULL CHECK (length(body) <= 245833),
    received_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, device_id)
) STRICT;

CREATE TABLE audit (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    at INTEGER NOT NULL,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 6),
    device_id BLOB CHECK (device_id IS NULL OR length(device_id) = 16)
) STRICT;
CREATE INDEX audit_account ON audit (account_id, at);

CREATE TABLE invites (
    code_hash BLOB PRIMARY KEY CHECK (length(code_hash) = 32),
    created_at INTEGER NOT NULL,
    used_at INTEGER
) STRICT;
",
];

/// The schema version this build writes.
#[must_use]
pub const fn latest_version() -> usize {
    MIGRATIONS.len()
}
