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
    // v2 (M6b): identity keys, roles, sharing, emergency access
    "
ALTER TABLE accounts ADD COLUMN ipk BLOB CHECK (ipk IS NULL OR length(ipk) = 32);
ALTER TABLE accounts ADD COLUMN wrapped_isk BLOB CHECK (wrapped_isk IS NULL OR length(wrapped_isk) = 74);
ALTER TABLE accounts ADD COLUMN contacts_ct BLOB CHECK (contacts_ct IS NULL OR length(contacts_ct) <= 262200);
ALTER TABLE accounts ADD COLUMN contacts_seq INTEGER NOT NULL DEFAULT 0;
ALTER TABLE accounts ADD COLUMN inbox_seq INTEGER NOT NULL DEFAULT 0;

CREATE TABLE memberships_v2 (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    role INTEGER NOT NULL CHECK (role BETWEEN 1 AND 4),
    origin INTEGER NOT NULL DEFAULT 0 CHECK (origin IN (0, 1, 2)),
    added_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, vault_id)
) STRICT;
INSERT INTO memberships_v2 (account_id, vault_id, role, origin, added_at)
    SELECT m.account_id, m.vault_id, 1, 0, v.created_at
    FROM memberships m JOIN vaults v ON v.vault_id = m.vault_id;
DROP TABLE memberships;
ALTER TABLE memberships_v2 RENAME TO memberships;
CREATE INDEX memberships_vault ON memberships (vault_id);

CREATE TABLE audit_v2 (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    at INTEGER NOT NULL,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 14),
    device_id BLOB CHECK (device_id IS NULL OR length(device_id) = 16)
) STRICT;
INSERT INTO audit_v2 (account_id, at, kind, device_id) SELECT account_id, at, kind, device_id FROM audit;
DROP TABLE audit;
ALTER TABLE audit_v2 RENAME TO audit;
CREATE INDEX audit_account ON audit (account_id, at);

CREATE TABLE vault_roster (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    key_generation INTEGER NOT NULL CHECK (key_generation >= 1),
    ciphertext BLOB NOT NULL CHECK (length(ciphertext) <= 65600),
    server_seq INTEGER NOT NULL,
    PRIMARY KEY (vault_id, key_generation)
) STRICT;

-- recipient has no foreign key: invites to addresses without an account are
-- stored like real ones (no enumeration through the directory). hidden = 1
-- marks an invite that is stored but never shown (blocked sender, full inbox).
CREATE TABLE share_invites (
    invite_id BLOB PRIMARY KEY CHECK (length(invite_id) = 16),
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    sender BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    recipient BLOB NOT NULL CHECK (length(recipient) = 16),
    role INTEGER NOT NULL CHECK (role BETWEEN 2 AND 4),
    generation INTEGER NOT NULL CHECK (generation >= 1),
    seal BLOB NOT NULL CHECK (length(seal) = 82),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    delivered_at INTEGER,
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 4),
    hidden INTEGER NOT NULL DEFAULT 0 CHECK (hidden IN (0, 1))
) STRICT;
CREATE UNIQUE INDEX share_invites_pending ON share_invites (vault_id, recipient) WHERE state = 0;
CREATE INDEX share_invites_recipient ON share_invites (recipient, state);
CREATE INDEX share_invites_sender ON share_invites (sender, state);

CREATE TABLE key_shares (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    recipient BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    generation INTEGER NOT NULL CHECK (generation >= 1),
    sender BLOB NOT NULL CHECK (length(sender) = 16),
    seal BLOB NOT NULL CHECK (length(seal) = 82),
    server_seq INTEGER NOT NULL,
    PRIMARY KEY (vault_id, recipient, generation)
) STRICT;

-- A removal or leave at generation g blocks a re-invite until the vault
-- rotates past g (the roster merge rule makes a same-generation re-invite lose).
CREATE TABLE vault_removals (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    account_id BLOB NOT NULL CHECK (length(account_id) = 16),
    generation INTEGER NOT NULL CHECK (generation >= 1),
    PRIMARY KEY (vault_id, account_id)
) STRICT;

CREATE TABLE emergency_grants (
    grantor BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    grantee BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    wait_days INTEGER NOT NULL CHECK (wait_days IN (1, 2, 7, 14, 30)),
    scope INTEGER NOT NULL CHECK (scope IN (1, 2)),
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 3),
    requested_at INTEGER,
    decided_at INTEGER,
    cooldown_until INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (grantor, grantee),
    CHECK (grantor <> grantee)
) STRICT;
CREATE INDEX emergency_grants_grantee ON emergency_grants (grantee);

CREATE TABLE emergency_seals (
    grantor BLOB NOT NULL,
    grantee BLOB NOT NULL,
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    generation INTEGER NOT NULL CHECK (generation >= 1),
    seal BLOB NOT NULL CHECK (length(seal) = 82),
    PRIMARY KEY (grantor, grantee, vault_id),
    FOREIGN KEY (grantor, grantee) REFERENCES emergency_grants (grantor, grantee) ON DELETE CASCADE
) STRICT;

CREATE TABLE blocks (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    blocked BLOB NOT NULL CHECK (length(blocked) = 16),
    PRIMARY KEY (account_id, blocked)
) STRICT;
",
    // v3 (M7): account-key epoch, rotation requests, re-key audit kinds
    "
ALTER TABLE accounts ADD COLUMN ak_epoch INTEGER NOT NULL DEFAULT 1 CHECK (ak_epoch >= 1);

-- The epoch never falls and rises by exactly one (only a re-key moves it).
CREATE TRIGGER accounts_epoch_monotone BEFORE UPDATE OF ak_epoch ON accounts
WHEN NEW.ak_epoch <> OLD.ak_epoch AND NEW.ak_epoch <> OLD.ak_epoch + 1
BEGIN SELECT RAISE(ABORT, 'ak_epoch'); END;

-- A member that re-keyed asks the vault owner to rotate (reason 1).
CREATE TABLE vault_rotation_requests (
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    requester BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    reason INTEGER NOT NULL CHECK (reason = 1),
    requested_at INTEGER NOT NULL,
    PRIMARY KEY (vault_id, requester)
) STRICT;

-- state 5: cancelled by the recipient's re-key.
CREATE TABLE share_invites_v3 (
    invite_id BLOB PRIMARY KEY CHECK (length(invite_id) = 16),
    vault_id BLOB NOT NULL REFERENCES vaults (vault_id) ON DELETE CASCADE,
    sender BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    recipient BLOB NOT NULL CHECK (length(recipient) = 16),
    role INTEGER NOT NULL CHECK (role BETWEEN 2 AND 4),
    generation INTEGER NOT NULL CHECK (generation >= 1),
    seal BLOB NOT NULL CHECK (length(seal) = 82),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    delivered_at INTEGER,
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 5),
    hidden INTEGER NOT NULL DEFAULT 0 CHECK (hidden IN (0, 1))
) STRICT;
INSERT INTO share_invites_v3 (invite_id, vault_id, sender, recipient, role, generation, seal,
    created_at, expires_at, delivered_at, state, hidden)
    SELECT invite_id, vault_id, sender, recipient, role, generation, seal,
    created_at, expires_at, delivered_at, state, hidden FROM share_invites;
DROP TABLE share_invites;
ALTER TABLE share_invites_v3 RENAME TO share_invites;
CREATE UNIQUE INDEX share_invites_pending ON share_invites (vault_id, recipient) WHERE state = 0;
CREATE INDEX share_invites_recipient ON share_invites (recipient, state);
CREATE INDEX share_invites_sender ON share_invites (sender, state);

CREATE TABLE audit_v3 (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    at INTEGER NOT NULL,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 16),
    device_id BLOB CHECK (device_id IS NULL OR length(device_id) = 16)
) STRICT;
INSERT INTO audit_v3 (account_id, at, kind, device_id) SELECT account_id, at, kind, device_id FROM audit;
DROP TABLE audit;
ALTER TABLE audit_v3 RENAME TO audit;
CREATE INDEX audit_account ON audit (account_id, at);
",
    // v4 (M9): WebAuthn credentials, recent second factor, revoked sessions,
    // the new audit kinds (ADR-0078, ADR-0080)
    "
ALTER TABLE accounts ADD COLUMN webauthn_user_handle BLOB CHECK (webauthn_user_handle IS NULL OR length(webauthn_user_handle) = 16);

CREATE TABLE webauthn_credentials (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    credential_id BLOB NOT NULL CHECK (length(credential_id) BETWEEN 16 AND 1023),
    public_key BLOB NOT NULL CHECK (length(public_key) <= 1024),
    sign_count INTEGER NOT NULL CHECK (sign_count >= 0),
    label TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 64),
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    PRIMARY KEY (account_id, credential_id)
) STRICT;
CREATE UNIQUE INDEX webauthn_credential_unique ON webauthn_credentials (credential_id);

ALTER TABLE sessions ADD COLUMN second_factor_until INTEGER NOT NULL DEFAULT 0;

CREATE TABLE revoked_sessions (
    token_hash BLOB PRIMARY KEY CHECK (length(token_hash) = 32),
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    reason INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 4),
    revoked_at INTEGER NOT NULL
) STRICT;
CREATE INDEX revoked_sessions_at ON revoked_sessions (revoked_at);

CREATE TABLE audit_v4 (
    account_id BLOB NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    at INTEGER NOT NULL,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 20),
    device_id BLOB CHECK (device_id IS NULL OR length(device_id) = 16)
) STRICT;
INSERT INTO audit_v4 (account_id, at, kind, device_id) SELECT account_id, at, kind, device_id FROM audit;
DROP TABLE audit;
ALTER TABLE audit_v4 RENAME TO audit;
CREATE INDEX audit_account ON audit (account_id, at);
",
];

/// The schema version this build writes.
#[must_use]
pub const fn latest_version() -> usize {
    MIGRATIONS.len()
}
