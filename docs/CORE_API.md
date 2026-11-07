# Mosaic core API (frozen at the end of M2)

This document fixes the Rust API that M3 to M6 build on: `mosaic-core`,
`mosaic-vault`, `mosaic-totp` and `mosaic-generator` (brief §7: "once the
core crate API is frozen (end of M2), develop desktop, extension and server
in parallel"). The rustdoc of each item is the detailed reference; this file
says what is **stable**, what is **restricted** to certain consumers, and
what is **internal**, and how any of it may change.

Mosaic is pre-audit. Nothing here has been externally reviewed.

## Change policy

- **Stable** items keep their names, signatures and documented behavior.
  A breaking change (removing or renaming an item, changing a signature, a
  default, an error a caller is told to handle, or documented behavior)
  needs an ADR in `docs/DECISIONS.md` and an update of this file in the same
  change. Behavior that `docs/CRYPTO_SPEC.md` fixes also needs the spec
  changed first (CLAUDE.md).
- **Additive** changes need no ADR: new functions, methods, constants, new
  variants of `#[non_exhaustive]` enums (every `Error` enum and
  `ApplyOutcome` are), new fields of `#[non_exhaustive]` structs with
  public fields (`PasswordOptions`, `PassphraseOptions`, `ItemFilter`), and
  new optional behavior that leaves existing calls unchanged. Record them
  in this file. A new public field of any other struct that callers build
  with a struct literal (`OtpCode`, `ItemRef`, `OtpAuth`, `Otp`, ...) is a
  breaking change.
- **Restricted** items are stable as well, but only the named consumers may
  call them. An app that needs one asks for a session-level method instead.
- **Internal** items (`pub(crate)`, `#[doc(hidden)]`, `cfg(fuzzing)`
  helpers, anything not listed here) may change at any time.
- The crates are unpublished (`version = "0.0.0"`, `publish = false`); this
  document, not semver, is the contract. Gate for every change: CLAUDE.md.

## Layers

```
 desktop (M3) · CLI (M8) · mobile (M10)          extension (M4, via the desktop app)
        │
        ▼
 mosaic-vault::session  ──►  mosaic-vault::otp ──► mosaic-totp
        │                                          mosaic-generator (used directly by apps)
        ├──► mosaic-core::model   (decrypted items, edits, merge)
        ├──► mosaic-core          (key flows, ciphertext types)
        └──► mosaic-vault::storage (SQLite, ciphertext only)

 sync server (M6): mosaic-core ciphertext types only (parse, size, key generation); never keys
```

Apps hold an `UnlockedSession` and never touch keys, ciphertext or the
store directly. The M6 sync client sits next to the session and uses its
`pending_uploads` / `mark_uploaded` / `apply_remote_*` methods.

## mosaic-vault

### `session` (stable)

| Item | Purpose |
|---|---|
| `LockedSession::open(Store)`, `has_account`, `store`, `into_store` | A store without keys. |
| `LockedSession::create_account(&MasterPassword, Argon2Profile)` → `(UnlockedSession, CreatedAccount)` | New account (CRYPTO_SPEC §5, §6). `CreatedAccount` carries the `EmergencyKit` (account ID and Secret Key display string, show once), the `SecretKey` (for the OS keystore), the public `PasswordParams` and the `OpaquePassword` for M6 registration. |
| `LockedSession::join_account(&PasswordParams, &WrappedKey, &MasterPassword, &SecretKey)` | Adds this device to an existing account (M6 pre-login data); nothing is stored on a wrong password. |
| `LockedSession::{derive_join_keys, join_account_with_keys, sync_config}` (M6 T1) | The two halves of `join_account` (`JoinKeys` carries `OPW` for the login before the wrapped `AK` is fetched, and the parameters it was derived under; `join_account_with_keys(wrapped_ak, keys)` stores those), and the stored backend choice. |
| `LockedSession::unlock(&MasterPassword, &SecretKey)` | Password unlock (§6.1). Failures return `UnlockError { session, error }`, the locked session unchanged; `Error::UnlockFailed` does not say which secret was wrong. |
| `UnlockedSession::lock(self)` → `LockedSession` | Drops AK, every VK, the decrypted items and the search index (all zeroize). |
| `UnlockedSession::lock_requiring_password(self)` → `LockedSession` | Like `lock`, and sets the failed-quick-attempt counter to `MAX_FAILED_QUICK_ATTEMPTS`: no quick method is offered or accepted until the next password unlock (ADR-0028 item 8; the app uses it for the five-wrong-reauthentications lock). |
| `account_id`, `device_id`, `store`, `corrupt_records` | State and diagnostics. `device_id` is this session's writer ID, fresh for every unlock (ADR-0015, third M2 integration review). |
| `change_master_password(current, new, &SecretKey)` → `PasswordChanged` | §6.4; returns the new params and `OPW'` for M6. Since ADR-0028 it first runs `verify_master_password`, deletes every quick-unlock wrapping in the same transaction (`PasswordChanged::removed_quick_unlock`). The session API does not offer Secret Key rotation yet (`secret_key` must be the current one); when it does, it MUST also delete every quick-unlock wrapping. |
| `rotate_device_id` | After `DeviceIdReused` (ADR-0009 item 7); then apply the record again. The old ID stays suspect for the session, so the re-applied record keeps both writers' values (ADR-0015). |
| `create_vault(name)`, `vaults()` → `Vec<VaultSummary>`, `vault_meta`, `rename_vault`, `rotate_vault_key` | Vaults; names live only in encrypted metadata (`VaultSummary`'s `Debug` does not print them); key generations only go up, and a rotation re-seals and queues every readable record and the metadata (§7.1). |
| `item`, `items(&ItemFilter)` → `Vec<ItemRef>`, `search(query)` | Reading. `ItemFilter { vault, trash: TrashFilter, favorites_only }`. `ItemFilter` is `#[non_exhaustive]` (third M2 integration review): start from `ItemFilter::default()` and set fields, so a new filter is additive. Search covers title, user name, email, URLs and tags, in memory only. |
| `add_item(vault, &ItemType, edit)`, `update_item(vault, item, edit)` | Writing through `mosaic_core::ItemEdit` closures; one sync event per call; a failing closure stores nothing; password changes go to the history. |
| `set_favorite`, `trash_item`, `restore_item`, `purge_item`, `move_item` | Favorites, trash, "delete forever" (a tombstone), move to another vault (re-encrypted, new item ID, attachments re-sealed). |
| `add_attachment`, `attachment`, `remove_attachment`, `attachment_usage`, `MAX_ACCOUNT_ATTACHMENT_BYTES` | Encrypted files, 50 MiB each, 1 GiB per account (§7.3). |
| `otp_code(vault, item, unix_time)` → `OtpCode { code, seconds_remaining }` | Current code of an item's OTP configuration; HOTP advances and saves the counter first. |
| `apply_remote_record(&RemoteRecord)` → `ApplyOutcome` | Merge a received record (§10, ADR-0012): `New`, `FastForward`, `Merged { restored_fields, stamp_collision }`, `Ignored`, and (added in the third M2 integration review) `Split { conflict_copies }` for a merge above the item limit (ADR-0016; the count of conflict copies, each marked with `conflict_copy_of`; changed from `Split { conflict_copy: Option<ItemId> }` in the fourth M2 integration review, ADR-0016 amendment) and `ReplacedUnreadable` for a local row that could not be read and so had no rollback anchor (THREAT_MODEL R17). |
| `apply_remote_vault_key(vault, generation, &WrappedKey)`, `apply_remote_vault_meta(vault, &VaultMetaCiphertext)` | New vaults, rotations, renames from other devices. A newer generation re-seals and queues every readable record still below it; the key of a held generation again is a no-op, a different one is `Error::VaultKeyConflict` (ADR-0013), a lower one `Error::KeyGenerationRollback`. A vault unavailable because its key row is damaged is repaired by a wrapping that opens (ADR-0012 amendment). Metadata merges with `VaultMeta::merge_and_repair` (`DeviceIdReused`, `Merged { stamp_collision }` as for records), and metadata that replaces an unreadable local metadata row is `ReplacedUnreadable` (fourth M2 integration review, ADR-0012 amendment). |
| `verify_master_password` → `Reauthenticated` (a proof bound to the session, valid `REAUTH_GRANT` = 120 s; `enroll_quick_unlock(&Kek, params, &Reauthenticated)` and `enroll_hardware_key(&NewHardwareKey, &PrfOutput, &Reauthenticated)` refuse a missing, expired or foreign one with `Error::ReauthenticationRequired`, so the API enforces "enrolling needs the master password"), `LockedSession::note_failed_hardware_attempt`, quick unlock (`enroll_quick_unlock`, `remove_quick_unlock`, `quick_unlock_enrollment`, `LockedSession::{unlock_quick, quick_unlock_enrollment, remove_quick_unlock, password_required}`), hardware keys (`enroll_hardware_key`, `remove_hardware_key`, `hardware_keys`, `LockedSession::{unlock_with_hardware_key, hardware_keys}`), hardware-protected vaults (`create_hardware_vault`, `open_hardware_vault`, `close_hardware_vault`, `hardware_vault_keys`, `add_hardware_vault_key`, `remove_hardware_vault_key`, `vault_protection`), types `QuickUnlockPolicy`, `QuickUnlockEnrollment`, `HardwareKeyEnrollment`, `NewHardwareKey`, `VaultProtection` | CRYPTO_SPEC §6.2, §6.3, §7.4, ADR-0028 (added in M3). |
| `pending_uploads(limit)`, `mark_uploaded(vault, record, OutboxSeq, server_seq)` | Outbox for the M6 client. |
| `Error`, `Result` | `#[non_exhaustive]`; variants carry only IDs and key generations. `VaultKeyConflict { generation }` was added in the M2 integration review. |

Contracts the apps must keep:

- Show `ApplyOutcome::Merged { stamp_collision: true }`,
  `ApplyOutcome::Split` (find the copies by `Item::conflict_copy_of()`),
  `ApplyOutcome::ReplacedUnreadable` and `Item::deleted_concurrently()` to
  the user, and offer `ItemEdit::resolve_conflict` / `dismiss_conflict` /
  `dismiss_deletion` / `dismiss_conflict_copy`.
- On `Error::Core(mosaic_core::Error::DeviceIdReused)` call
  `rotate_device_id` and apply the record (or the vault metadata) again.
- On `Error::KeyGenerationNotAccepted` apply the vault's newer wrapped key
  first; `KeyGenerationRollback` (for a record, metadata or a wrapped key
  alike) is an attack or a stale server and is dropped, never retried.
- On `Error::VaultKeyConflict` keep going with the held key: another
  device rotated the same vault concurrently. M6 resolves it (the server
  accepts one key per generation, the losing device rotates again,
  ADR-0013); records sealed under the other key fail with `ItemCorrupted`
  meanwhile and change nothing.
- One store per install and per writing process: the desktop app and the
  extension never share one, and the store lock refuses a second session
  on the same file, whatever path or symbolic link names it
  (`StoreInUse`). Every session writes under a fresh writer ID, so a
  restored or copied store never shares one (ADR-0015).
- Keep the `SecretKey` in the OS keystore; the store never holds it.

**Sync API of the session (M6 T1, `session/sync_support.rs`; for the M6 sync
engine and the desktop sync service, ADR-0045).** `UnlockedSession`:
`install_id`; `sync_config` / `set_sync_config` (`SyncConfig`, `SyncKind`);
`reset_sync_state` (one transaction: every record of a synced vault loses its
`server_seq` and is queued, the current vault keys, the metadata and every
attachment are queued, vaults count as not created on the backend, cursors,
local sequence blocks, `sync_cursors` and pending attachment deletions are
cleared; hardware-protected vaults are left alone);
`synced_vaults`; `pending_uploads_in(vault, limit)`;
`pending_vault_uploads(&mut self)` (`PendingVaultUpload`; `&mut` because it
remembers the metadata handed out for a creation) with
`mark_vault_created_remotely`, `mark_vault_key_uploaded`,
`mark_vault_meta_uploaded` (clears the queued flag only if the stored bytes
equal the uploaded ones) and `note_vault_meta_seq`; `vault_key_origin`
(`KeyOrigin`); the two-phase rotation `prepare_vault_rotation` →
`PreparedRotation` (no state change) and `commit_vault_rotation` (the key
counts as generated here and already uploaded; `KeyGenerationRollback` if
the vault accepted that generation meanwhile; `rotate_vault_key` stays for
offline use and queues its key); `checkpoint_mac` / `verify_checkpoint_mac`,
`folder_mac` / `verify_folder_mac` (`FolderMacPurpose`), `seal_device_info` /
`open_device_info`; `next_checkpoint_counter` / `checkpoint_counter`;
`sync_cursor` / `set_sync_cursor`; `allocate_local_seqs`;
`attachment_references` (`AttachmentRef`), `attachment_file_state`
(`AttachmentFileState`), `attachment_ciphertext`, `store_remote_attachment`
(`Error::UnknownAttachmentReference` unless an item of the vault refers to
the attachment), `pending_attachment_uploads`, `mark_attachment_uploaded`,
`pending_attachment_deletions`, `mark_attachment_deletion_sent`;
`remote_attachment_authentic` (whether downloaded bytes open under the file
key of the item that refers to them) and `attachment_ciphertext_len`;
`store_remote_attachment` now opens every chunk first (`Error::Core` for
bytes that do not authenticate); `LockedSession::verify_join_folder_mac` (a
candidate account file's MAC, checked before the join commits to it).
The persisted engine state (store schema v4, CRYPTO_SPEC §11; `reset_sync_state`
clears it except the halt): `own_seq` / `reserve_own_seq` / `confirm_own_seq`
(per vault, `OwnSeq { reserved, written }`), `sync_engine_state`
(`EngineState`: halt, last published counter, strikes), `set_sync_halt` and
`clear_sync_halt` (the user's acknowledgement; for the desktop's T5 action),
`set_last_published_counter`, `set_checkpoint_strikes`, `peer_checkpoints` /
`put_peer_checkpoint` (a lower counter is never stored), `sync_lag` /
`set_sync_lag`, `note_key_observation` / `key_observations` /
`has_key_conflict` / `has_any_unresolved_key_conflict` /
`prune_key_observations`, `marker_first_seen`, `vault_deleted_locally`,
`all_vault_ids`. There is no accessor that hands out a derived key
(`sync_keys` / `SyncKeys` of the first T2 draft are removed): the MACs and
seals above are computed inside the session, one operation at a time.
`account_bootstrap` (`AccountBootstrap`); `opaque_password` (`UnlockFailed`
unless the derived `KEK` opens the stored wrapping);
`prepare_master_password_change` → `PreparedPasswordChange` and
`commit_master_password_change` (`change_master_password` is now both).
Each unlocked session has a random 16-byte tag (never stored); both prepared
values record it, and a commit in a later session (prepare, lock, unlock,
commit) is refused with the new `Error::PreparedStale` (added because no
existing variant describes it). `Store::rewrap_vault_key` clears
`local_origin` and `upload_pending` (the remote-replacement path), and
`Store::accept_uploaded_vault_key` sets them on a row that holds the same
wrapping already.
Deviation from the M6 spec: `pending_vault_uploads` takes `&mut self`.

M6 T4 adds `Store::raise_server_seq(vault, record, seq)` (raises the stored server
sequence number, the base of the next upload, without touching the body or the outbox;
never lowers it) and changes `apply_remote_record`: a received version that the local
record dominates or equals is still `Ignored`, but it now rebases a queued change onto
the received sequence number, or acknowledges it when the received ciphertext is the
queued change itself (the response to its upload was lost). `join_account_with_keys`
takes `(wrapped_ak, keys)`: the parameters travel inside `JoinKeys`.

### `otp` (stable)

`otp_from_config(&OtpConfig, hotp_counter)` → `mosaic_totp::Otp`,
`config_from_otp(&Otp, issuer, label)` and `config_from_otp_auth(&OtpAuth)`
→ `(OtpConfig, counter)`, `otp_auth_from_config(&OtpConfig, counter)` →
`OtpAuth` (for `otpauth://` export). Store the counter with
`ItemEdit::set_otp_counter`. `ItemEdit::otp()` (added in M3, ADR-0028) reads the
configuration inside an edit so the session can keep `hardware_protected` in
step with the vault in the same sync event.

### `storage` (restricted: `session`, the M6 sync client, migrations, tests)

`Store::{open, open_in_memory, close, schema_version, integrity_check,
attachment_dir}` and `LATEST_VERSION` (the schema version this build
writes; 4 since the M6 T2 review: the engine state tables) are stable for every consumer (apps open the store and hand it to
`LockedSession::open`). `Store::open` holds an exclusive lock on the store
until it is dropped or closed; a second open fails with
`storage::Error::StoreInUse` (added in the second M2 integration review,
ADR-0010 item 10), which an app shows as "Mosaic is open elsewhere". Everything else on `Store` (records, outbox,
vaults, wrapped keys, unlock methods, TK wrappings, attachments, sync
cursors, device ID, `rekey_account`), the types `RecordBody`, `RecordKind`,
`Origin`, `StoredRecord`, `CorruptRecord`, `RecordEntry`, `OutboxSeq`,
`VaultState`, `VaultKeyWrap`, `PendingPolicy`, `StoredUnlockMethod`,
`NewUnlockMethod`, `AccountRekey`, `StoredTkWrapping`, `AttachmentInfo` and
`storage::Error` (with `VaultKeyConflict`, ADR-0013), the function
`validate_attachment_ciphertext` (the §7.3 layout check the M6 server
also runs on uploads) and the constant `MAX_UNLOCK_PARAMS_LEN` are
restricted, including `Store::{create_tk_vault, update_password_removing}`
(M3, ADR-0028). Added in the third M2 integration review: `readable_vault_keys`
(the wrapped keys of a vault that decode, damaged rows left out);
`mark_uploaded` returns `false` for a late acknowledgement of a superseded
change instead of `StaleServerSeq`; `Store::open` resolves the path before
naming the lock file and the attachment directory. The store accepts only `mosaic-core` ciphertext types and
enforces the §7.1 and ADR-0010 rules itself. Schema v1 is final as of the
M2 integration commit: every change goes through a migration
(`LATEST_VERSION`), never an in-place edit.

Schema v3 (M6 T1, ADR-0045) is additive: `vault_keys.{upload_pending,
local_origin}`, `vaults.{remote_created, meta_upload_pending,
meta_server_seq}` and the tables `install_identity`, `sync_config`,
`sync_cursor_blobs`, `sync_local_seq`, `sync_checkpoint` (with a trigger that
forbids a lower counter) and `attachment_deletions`. The legacy `sync_cursors`
table stays; M6 does not use it and `reset_sync_state` clears it. New
restricted `Store` items: `install_id`, `sync_config` / `set_sync_config`
(`SyncConfig`, `SyncKind`), `sync_cursor_blob` / `set_sync_cursor_blob`,
`allocate_local_seqs` (reserves blocks of `LOCAL_SEQ_BLOCK` = 1024: a crash
skips numbers, never reuses one), `checkpoint_counter` /
`next_checkpoint_counter`, `pending_attachment_deletions` /
`mark_attachment_deletion_sent` (`delete_attachment`, `delete_record` and
`delete_vault` record uploaded attachments; orphan files left by a crash have
no row and so no deletion), `vault_sync_state` (`VaultSyncState`),
`pending_vault_keys`, `vault_key_origin`, `mark_vault_key_uploaded`,
`mark_vault_created`, `set_vault_meta_queued`, `mark_vault_meta_uploaded`,
`note_vault_meta_seq`, `create_vault_origin`, `accept_vault_key_origin`,
`accept_uploaded_vault_key` (`KeyOrigin`), `outbox_in(vault, limit)`,
`reset_sync_state` and `storage::Error::SyncDataInvalid`. Hardware-protected
vaults never get pending flags.

Internal: `session::fuzzing` (`cfg(fuzzing)` only).

## mosaic-core

Stable for every consumer:

- **IDs:** `AccountId`, `VaultId`, `ItemId`, `AttachmentId`, `DeviceId`,
  `InstallId` (M6 T1: the stable per-install ID of checkpoints, folder
  paths and the server's device list; `DeviceId` stays the writer ID)
  (`new_v7`, `from_bytes`, `from_slice`, `as_bytes`, `Display`),
  `KeyGeneration` (`FIRST`, `new`, `get`, `next`).
- **Secrets** (zeroize on drop, redacted `Debug`, not `Clone`,
  constant-time `==`): `MasterPassword`, `SecretKey` (`generate`, `parse`,
  `to_display`, `last_group`, `matches_last_group`), `SecretString`
  (`from_str_copy`, `From<String>`, `expose_secret`), `SecretVec`,
  `SecretBytes`, `OpaquePassword`.
- **Profiles:** `Argon2Profile` (`DEFAULT`, `ALL`, `id`, `from_id`,
  `m_cost_kib`, `t_cost`, `p_cost`) and `kdf::calibrate` /
  `kdf::measure_profile` to pick one at account creation, with
  `kdf::calibrate_with(measure)` (the same choice over a caller's
  measurement, for tests and progress UI) and its thresholds
  `kdf::CALIBRATION_TARGET` and `kdf::STANDARD_CEILING`.
- **Model** (`mosaic_core::model`, re-exported in part at the root):
  `Item` (getters, `edit`, `merge`, `merge_and_repair`, `undelete`,
  `compare`, `conflicts`, `deleted_concurrently`, `is_deleted`; added in
  the third M2 integration review: `merge_and_repair_after_reuse`,
  `split_oversized`, `conflict_copy_of`; changed in the fourth:
  `split_oversized(received, item_id, &VaultKey, device, reused)` without
  the time argument, and added `Item::conflict_copy_id(&VaultKey, item_id,
  key, Stamp)`, ADR-0016 amendment), `ItemEdit` (every setter,
  `resolve_conflict`, `dismiss_conflict`, `dismiss_deletion`,
  `dismiss_conflict_copy`), `ItemType`, `TextField`, `SecretField`,
  `ElementId`, `HistoryEntry`, `DeletionInfo`, `MergeOutcome`,
  `OversizedSplit` (`item`, `conflict_copies: Vec<(ItemId, Item)>`, was
  `conflict_copy: Option<Item>` before the fourth review),
  `FieldConflict`, `Entry`, `LoginUrl`, `UrlMatch`, `CustomField`,
  `CustomFieldValue`, `AttachmentRef`, `OtpConfig`, `OtpKind`,
  `OtpAlgorithm`, `PasskeyCredential`, `PasskeyPublic`, `SshKeyMaterial`, `VaultMeta`
  (getters, `edit`, `merge`, `merge_and_repair` → `(VaultMeta,
  stamp_collision)`, `merge_and_repair_after_reuse`, `compare`,
  `conflicts`; `merge_and_repair` was added in the second M2 integration
  review, `merge_and_repair_after_reuse` in the third), `VaultMetaEdit`, `Tombstone`, `Text`, `Value`, `Number`, `Object`,
  `Stamp`, `Timestamp`, `VersionVector`, `Causality` and the limits
  (`MAX_HISTORY`, `MAX_TAG_LEN`, `MAX_FIELD_KEY_LEN`, `MAX_JSON_DEPTH`,
  `MAX_OTP_SECRET_LEN`, `MAX_CREDENTIAL_ID_LEN`, `MAX_USER_HANDLE_LEN`,
  `SCHEMA_VERSION`). The plaintext encoding is CRYPTO_SPEC §7.5; the merge
  rules are §10 and ADR-0009.
- **Errors:** `Error` (`#[non_exhaustive]`, no secrets in any variant),
  `Result`.

Restricted (`mosaic-vault`, the M6 server and sync client, M3 unlock-method
setup, tests and fuzzing):

- **Sync keys and MACs** (`mosaic_core::sync_auth`, M6 T1, CRYPTO_SPEC §9.2,
  §10, §10.1): the key types `CheckpointKey`, `FolderMacKey`,
  `DeviceInfoKey` (zeroizing, redacted `Debug`, no `Clone`, constant-time
  `==`; derived by `AccountKey::{checkpoint_key, folder_mac_key,
  device_info_key}`); `checkpoint_mac`, `verify_checkpoint_mac`,
  `folder_mac`, `verify_folder_mac` with `FolderMacPurpose` (`Segment`,
  `Account`, `BlobDelete`); `seal_device_info` / `open_device_info`
  (XChaCha20-Poly1305, exactly 512 bytes of padded plaintext, so every
  file has the same size); and
  `session_token(session_key)` → `SessionToken` (`expose`; stable for the
  sync client, which sends it as the bearer token). The MAC functions return
  `Result<_>` (HMAC cannot fail for a 32-byte key, but there is no silent
  fallback). Known-answer tests read `tests/vectors/sync_kat.txt`, written by
  the independent script `tests/vectors/sync_kat.py`.

- **Ciphertext types** (the methods differ by type):
  - `ItemCiphertext`: `seal`, `from_bytes`, `as_bytes`, `schema_version`,
    `key_generation`, `open`.
  - `TombstoneCiphertext` and `VaultMetaCiphertext`: `seal`, `from_bytes`,
    `as_bytes`, `key_generation`, `open`.
  - `WrappedKey`: `LEN`, `from_bytes`, `as_bytes`, `unwrap_account_key`,
    `unwrap_vault_key`; the wrapping itself is `AccountKey::wrap` /
    `VaultKey::wrap` (no generation of its own: a vault key's generation is
    in its AAD and comes from context).
  - `UnlockMethod` (`Password`, `QuickUnlock`, `HardwareKey`): `id`,
    `hardware_key(credential_id)`, `method_id(account_id)`.
  The server may parse and size them but never holds a key to open them.
- **Key flows:** `create_account`, `unlock_with_password`,
  `change_master_password`, `derive_password_keys`, `NewAccount`,
  `PasswordChange`, `PasswordParams`, `AccountKey`, `VaultKey`,
  `AttachmentKey`, `Kek`, `AccountSalt` (`generate`, `from_bytes`,
  `as_bytes`), `DerivedKeys`, the
  `wrap` / `unwrap_*` methods, `seal_attachment`, `open_attachment`,
  `MAX_ATTACHMENT_LEN`, `attachment::ciphertext_len`.
- **`unlock` module** (M3 unlock-method setup and `mosaic-vault`, ADR-0028):
  `hello_unlock_key`, `prf_salt_input`, `credential_id_hash`,
  `prf_unlock_key`, `prf_hw_vault_key`, `new_prf_salt`, `PrfOutput`,
  `TotpVaultKey`, `Kek::generate`, `Kek::expose_secret`,
  `VaultKey::wrap_under_tk`, `WrappedKey::{unwrap_totp_vault_key,
  unwrap_vault_key_under_tk}`.
- **`browser_channel` module** (M4, additive; CRYPTO_SPEC §15, ADR-0036; used by
  `mosaic-native-messaging` and the desktop's browser service): `PairingKey`
  (`generate`, `from_bytes`, `expose_secret`, `proof`, `verify_proof`,
  `session`; zeroizing, not `Clone`, redacted `Debug`, `ConstantTimeEq`),
  `Role` (`Host`, `Desktop`), `ChannelSession` (`seal`, `open`; strict
  per-direction sequence numbers, one `Decryption` error for every failure; a
  failed `open` poisons the session), `new_pairing_id`, `new_channel_nonce`,
  `new_endpoint_token` (16 random bytes for the Windows pipe name),
  `new_pairing_code_bits` (30 random bits for the pairing code), and the constants `PAIRING_ID_LEN`, `PAIRING_KEY_LEN`,
  `CHANNEL_NONCE_LEN`, `PROOF_LEN`, `FRAME_HEADER_LEN`. The caller never
  supplies a nonce; the frame nonce is derived from the sequence number.
- **Building blocks:** `encoding` (`ctx`, `CTX_PREFIX`, `pad`, `unpad`,
  `padded_len`, `PAD_BLOCK`, `PAD_MIN`; the AAD builder is crate-private),
  `kdf::{prepare_password, stretch, derive_keys}` with the intermediate
  secrets they pass along, `kdf::PreparedPassword` and
  `kdf::StretchedPassword` (zeroizing, opaque). Only for implementing
  CRYPTO_SPEC objects that M5 and M6 add (identity keys, shares,
  checkpoints), each with its spec section and ADR first; new object types
  are added inside `mosaic-core`, not built from these parts elsewhere.

Internal: `VaultKey::from_secret_for_fuzzing`, golden-file helpers,
everything `pub(crate)`. The `envelope` module's format constants
(`FORMAT_VERSION`, `ALG_XCHACHA20_POLY1305`, `MAX_ITEM_PADDED_LEN`,
`MAX_ITEM_PLAINTEXT_LEN`, `MAX_VAULT_META_PADDED_LEN`,
`MAX_VAULT_META_PLAINTEXT_LEN`, `MAX_TOMBSTONE_PLAINTEXT_LEN`) and the
`attachment` module's format constants (`ALG_STREAM_BE32_XCHACHA20_POLY1305`,
`CHUNK_LOG2`, `CHUNK_LEN`, `NONCE_PREFIX_LEN`, CRYPTO_SPEC §7.3) are
restricted like the ciphertext types.

## mosaic-totp (stable)

`Otp { secret: OtpSecret, algorithm: Algorithm, kind: OtpKind }` with
`code_at`, `seconds_remaining` and `verify(candidate, unix_time, window,
last_used)` (constant time, replay guard, `window ≤ MAX_VERIFY_WINDOW`,
the largest matching step wins, ADR-0008); `OtpKind::{Totp, Hotp, Steam}`
with `OtpKind::DEFAULT_TOTP` and `OtpKind::STEAM`; `Digits` (`new`, `get`,
`MIN`, `MAX`, `DEFAULT`), `Period` (`new`, `get`, `time_step`,
`seconds_remaining`, `MIN`, `MAX`, `DEFAULT`), `Algorithm` (`parse`,
`as_str`); `Code` (`expose_secret`, `len`, `is_empty`, `ct_matches`; never
compare with `==`; its characters live in a heap box, ADR-0008
amendment); `OtpSecret` (`from_bytes`, `from_base32`, `to_base32`,
`strength()` → `SecretStrength`, `meets_rfc4226_minimum`); `SecretText`
(zeroizing text such as an exported URI or an OTP label: `new`,
`From<String>`, `expose_secret`, `len`, `is_empty`, constant-time `==`);
`OtpAuth { otp, issuer: Option<SecretText>, account_name: SecretText }`
(the label fields became `SecretText` in the third M2 integration review,
ADR-0008 amendment) with `parse` and `to_uri` for `otpauth://` URIs and `validate_label(issuer, account)` for
the label rules alone (manual entry in the desktop app, ADR-0031; the `uri` module is public only for these items and
its limits, which the root re-exports); `hotp`, `totp`, `steam_guard`,
`unix_time_now`; the limits (`MAX_SECRET_LEN`, equal to the model's
`MAX_OTP_SECRET_LEN`, `MAX_SECRET_TEXT_LEN`, `MAX_URI_LEN`, `MAX_PARAMS`,
`MAX_TEXT_LEN`, `RFC4226_MIN_SECRET_LEN`, `COMMON_MIN_SECRET_LEN`,
`STEAM_ALPHABET`, `STEAM_CODE_LEN`); `Error`, `Result`. The crate does not
depend on `mosaic-core`; `mosaic_vault::otp` converts.

## mosaic-generator (stable)

`generate_password(&PasswordOptions)`, `generate_passphrase(&PassphraseOptions)`;
`PasswordOptions` (fields `length`, `lowercase`, `uppercase`, `digits`,
`symbols`, `symbol_set`, `exclude_ambiguous`; `Default`, `entropy()`) and
`PassphraseOptions` (fields `words`, `separator`, `capitalize`,
`append_digit`; `Default`, `entropy()`), both `#[non_exhaustive]` since the
third M2 integration review: callers start from `Default` and set fields,
so a new option is additive;
`PasswordRules::{parse, generate, generate_default, entropy,
is_satisfied_by, required, allowed, min_length, max_length,
max_consecutive, max_repeating, max_sequential}` and `Display` (canonical,
round-trips), `CharacterClass` (`contains`, `is_unicode`), `rules_for_domain(host)` → `SiteRules`
(`domain`, `rules`, `into_rules`), `GeneratedPassword` (`expose_secret`,
`entropy`), `Entropy` (`bits`, `is_upper_bound`, `strength`), `Strength`
(`from_bits`),
the option and limit constants (`MIN_LENGTH`, `MAX_LENGTH`,
`DEFAULT_LENGTH`, `DEFAULT_SYMBOLS`, `AMBIGUOUS_CHARACTERS`, `MIN_WORDS`,
`MAX_WORDS`, `DEFAULT_WORDS`, `DEFAULT_SEPARATOR`, `MAX_SEPARATOR_CHARS`,
`WORDLIST_LEN`, `BITS_PER_WORD`, `MAX_RULES_LEN`, `MAX_REQUIRED_CLASSES`,
and `MIN_RULES_BITS`, the site-rules entropy floor, added in the fourth M2
integration review, ADR-0011 amendment),
`Error`, `Result`. The `host` module (added in the code audit, additive) holds
the shared host-name helpers `MAX_HOST_LEN`, `normalize`, `is_numeric` and
`candidates`, also used by `mosaic-insights`. Randomness is the OS CSPRNG only;
callers cannot inject an RNG (ADR-0011). Internal: the `fuzzing` module.

## mosaic-passkey (stable, new in M5)

The ES256 software authenticator of CRYPTO_SPEC section 16. Pure functions and one
key type; no vault, no I/O, no origin policy (callers pass an rpId validated by
`mosaic_origin::webauthn_rp_id`, ADR-0040).

- `Es256Key` (not `Clone`, `Debug` prints `Es256Key([REDACTED])`, zeroized on
  drop): `generate`, `from_pkcs8_der`, `to_pkcs8_der` (a zeroizing `SecretVec`),
  `cose_public_key` (77 bytes), `public_key_spki_der` (91 bytes). There is no
  public `sign`: only `assert` signs, over the authenticator-data message, so the
  key is no raw signing oracle.
- `create(&CreateInput) -> Created` always draws a fresh key and credential ID
  (a version with a given key and ID exists for the crate's own tests only).
- `assert(&GetInput, &PasskeyCredential, stored_count) -> PendingAssertion`
  (`#[must_use]`, private fields, so the signature cannot be used before the sign
  count is dealt with, ADR-0039 D5). `count_to_persist() -> Option<u64>`: `Some(n)`
  means store `n` with `ItemEdit::set_passkey_sign_count`, wait for the write, then
  `into_response(CountPersisted) -> AssertionResponse`. `CountPersisted` is a
  zero-sized token built with `CountPersisted::after_store_write()`, to be called
  only after that write succeeded (the type cannot check it). `None` (a counter
  that stays 0): `into_response_without_count() -> Result<AssertionResponse,
  Error>`, which is `InvalidInput` if a count is pending. `AssertionResponse`
  carries `credential_id`, `client_data_json`, `authenticator_data`, `signature`
  and `user_handle`.
- Encodings: `client_data_json`, `ccd_to_string`, `rp_id_hash`,
  `attestation_object_none`, `b64url`, `from_b64url`, `new_credential_id`,
  `select_algorithm`; the `flags` module. (`authenticator_data` is crate-private;
  it returns `InvalidInput` for a credential ID over 65535 bytes.)
- `sanitize_name(&str) -> String` and `MAX_ACCOUNT_NAME_BYTES` (64): removes control and format
  characters and cuts at a character boundary; for every name from a site, an import or sync.
  `pkcs8_kind(&[u8]) -> Pkcs8Kind` (`P256`, `Other`, `NotPkcs8`): the algorithm and curve of a PKCS#8
  private key from its structure (importers tell an unsupported key from a damaged P-256 key).
- Constants: `COSE_ALG_ES256`, `AAGUID`, `CREDENTIAL_ID_LEN`, `MIN_CHALLENGE_LEN`,
  `MAX_CHALLENGE_LEN`, `MAX_ALLOW_LIST`; re-exports `MAX_CREDENTIAL_ID_LEN`,
  `MAX_USER_HANDLE_LEN` from the model. `Error` is `Rng`, `UnsupportedAlgorithm`,
  `InvalidKey`, `InvalidInput`; none carries key material.

`mosaic-origin` gains `webauthn_rp_id(&PageOrigin, Option<&str>) -> Result<String,
RpIdError>` (additive): https only, host a non-loopback domain, rpId canonical,
equal to the host or a dot-suffix of it, not a public suffix, and at or under the
host's registrable domain (so `evil.tenant.platformsh.site` cannot claim
`platformsh.site`, which is registrable under a wildcard rule; `a.b.kawasaki.jp`
cannot claim `kawasaki.jp`).

`mosaic-core` gains `PasskeyPublic` (credential ID, rp ID, user handle, user name, display name,
algorithm, discoverable, creation time: no private key) and `Item::passkey_public() ->
Option<PasskeyPublic>` (additive, M5 T4). It validates like `PasskeyCredential` but never decodes
or copies the key. Code that lists, filters or shows passkeys uses it; `Item::passkey()` is for the
one credential that signs or is exported.

`mosaic-import` gains (additive, M5 T4): `Format::Cxf`, `cxf::parse`, `PasskeyDraft` (credential,
sign count, title), `ItemDraft::passkeys` and `ItemDraft::has_item`, `MAX_PASSKEYS_PER_ENTRY`,
`MAX_IMPORT_PASSKEYS`, `EntryWarning::{PasskeyUnreadable, PasskeyUnsupported, PasskeysLimit}` and
`ImportWarning::{PasskeyUnreadable, PasskeyUnsupported, PasskeysSkipped}`. `commit`
adds each passkey as its own `passkey` item after its entry (counted in `CommitReport::imported`);
a passkey already in the vault is skipped (`CommitReport::skipped`). `mosaic-origin` gains
`canonical_rp_id`; `mosaic-passkey` gains `pkcs8_kind` and `Pkcs8Kind`.

## mosaic-sync (new in M6 T1; restricted: the M6 engine, desktop sync service and server)

`mosaic_sync::wire` holds the formats of both sync backends; every decoder
is length-bounded, verifies its MAC in constant time before it parses and
rejects trailing bytes, unknown versions, nil IDs and IDs that differ from
the path (CRYPTO_SPEC §10.1, ADR-0043). `SyncCrypto` is the keyed side
(`UnlockedSession` implements it; `SessionCrypto<H: SessionHandle>` is the
production provider for a transport, deriving every key per operation from a
shared session handle; `KeyedCrypto` behind `test-util` is for tests and
fuzzing), so the crate never holds `AK` or a key derived from it. Items: `segment`
(`Segment`, `SegmentObject`, `SegmentPath`), `checkpoint` (`Checkpoint`,
`CheckpointEntry`; `decode(bytes, expected_device, crypto)` refuses a body
that names another device than the one it is filed under, `decode_unverified`
for a server without a key),
`account_file` (`AccountFile`, `decode_for_join`, `verify_mac`),
`blob_delete` (`BlobDelete`, `BlobDeletePath`; 93 bytes, one marker per
device), `device_info` (`DeviceInfo`, a constant 512-byte padded plaintext),
`segment::{SnapshotInfo, SnapshotTracker}` (multi-part snapshots), `cursor` (`FolderCursor`), `folder_names` (`parse`,
`FolderName`, name builders), `email::normalize` (shared with the server) and
`json` (every request and response body of the hosted protocol, `parse` with a
size cap, `B64` with strict base64url, `codes`). `mosaic_sync::transport` has
the blocking, object-safe `Transport` trait and `TransportError`
(`put_checkpoint(&[u8])` and `checkpoints() -> Vec<(InstallId, Vec<u8>)>`:
transports carry checkpoints as opaque bytes). `json::TotpSetupResponse` holds
its secret and URI in `SecretText` (redacted `Debug`, no `Clone`). Features:
`test-util`; `opaque` and `http` are reserved (the OPAQUE module and the HTTP
transport come with the server and client tracks, ADR-0046).

**M6 T2 (engine and folder transport).** `engine::SyncEngine<T: Transport>`
(`new`, `with_jitter`, `sync_once(&mut impl SessionAccess, &impl Clock)`,
`rotate_vault`, `wait_for_change`, `next_retry_at`, `halted` / `clear_halt`,
`transport_mut`), `SessionAccess` (short locked steps; `Err(Locked)`),
`EngineConfig::for_kind`, `SyncReport`, `SyncWarning`, `SyncError`,
`RotationOutcome`, `Jitter` / `SystemJitter`, `Backoff`, `consts`.
`clock::{Clock, SystemClock}`.
`folder::{FolderTransport<C: FolderCrypto>, FolderConfig, init, inspect,
list_accounts, FolderContents, FolderError, current_account_file,
join_from_folder, JoinError, JoinFailure}`; `FolderTransport::write_account_file`,
`highest_account_version` (verified files only) and `next_account_version` for
the first publication and a password change. `FolderCrypto` is
`SyncCrypto + OwnLogState + Send` (blanket impl): `wire::SessionCrypto<H>`
implements both traits over a `wire::SessionHandle` (`with_session` /
`with_session_mut`; implemented for `Mutex<Option<UnlockedSession>>` and
`Arc` of a handle, which is the desktop's shape). A locked handle makes every
operation fail closed: `WireError::Locked`, `TransportError::Locked`,
`SyncError::Locked`. `wire::KeyedCrypto` is test-only again (`new(ak, ..)`
behind `test-util`, in-memory own-log state). Transport trait changes:
`PullPage::metas` (a `Vec`), `checkpoint_high_seq`, `maintain(&mut dyn
MaintainContext)` (which gained `marker_first_seen`), `replace_attachment`
(default `put_attachment`; the folder overwrites a blob that fails
authentication) and `own_logs_rolled_back(&[VaultId])` (default `false`).
Engine: `halted(&mut access)` and `clear_halt(&mut access)` read and clear
the persisted halt; `SyncReport` gained `attachments_rejected` and
`attachments_healed`. Feature `test-util`: `testutil::{CloudSim, SimConfig,
ChaosTransport, ChaosConfig, MaliciousTransport, ManualClock, FixedJitter,
SimRng, LocalSession, SessionRef, SessionMut}` (`LocalSession` is a shared,
clonable handle: the transport gets a clone as its `SessionHandle`). The
server (T3) follows.

**M6 T4 review follow-ups (additive).** `UnlockedSession::adopt_remote_vault_key(vault,
generation, &WrappedKey) -> Result<AdoptOutcome>` with `AdoptOutcome::{AlreadyHeld,
Adopted { resealed }}`: for the hosted backend, replaces the key this device generated for
the vault's current generation and the backend never accepted by the backend's key. The
swap, the re-seal of every record and of the metadata sealed under the replaced key and
their queueing are one store transaction (`Store::adopt_vault_key`); the replaced key is
dropped. Errors: `VaultKeyConflict` if the held key came from the backend or was confirmed
by it, `KeyGenerationRollback` / `KeyGenerationNotAccepted` if `generation` is not the
vault's current one, `Core(Decryption)` if the wrapping does not open, `VaultUnavailable`,
`WrongVaultProtection`. The caller then resets the vault cursor (`set_sync_cursor(vault,
&[])`) and pulls again. Also on the session (hosted engine state, schema v5):
`note_served`, `served_seq`, `served_view` (`ServedRow`), `mark_vault_key_confirmed`,
`vault_key_confirmed`, `set_vault_flag`, `vault_flags` (`VaultFlag::{ServerInconsistent,
ServerRolledBack}`), `log_conflict_rotation`, `recent_conflict_rotations`. `clear_sync_halt`
clears only the halt, the strikes and `written` (as before); the findings against the server
go through `clear_vault_findings(&[VaultId])` (engine: `clear_findings`), which clears them
and `NoCommonView`, drops the served view, the cursor and the current-generation key
candidates of those vaults and marks them for a rebuild (`rebuild_vaults`,
`finish_rebuild`; after the full re-read the engine queues local records the server lacks
with `requeue_record`) and marks the stored peer checkpoints as acknowledged
(`PeerCheckpoint::digests_acknowledged`). `sync_findings() -> SyncFindings { halt,
vault_flags, no_common_view }` lists both kinds (`is_clear`, `vaults`), so the desktop can
offer one acknowledge action and call both clears depending on what is set.
`mark_vault_key_accepted` records an accepted upload and its confirmation in one transaction
(the creation and `commit_vault_rotation` confirm in theirs); `view_compare` and
`put_view_compare` hold the comparison times. `mosaic_core::sync_auth` gains
`view_key_commit`, `view_digest` and `ServedEntry`; the view digest has the key commitment
(CRYPTO_SPEC §10). `HttpTransport::connect` takes `&SessionToken`.
`PeerEntry` and `CheckpointEntry` gain `view_digest: [u8; 32]` (`checkpoint::NO_VIEW`,
`checkpoint::view_digest`, `CheckpointEntry::view`); the checkpoint is 60 bytes per entry
and 245,833 bytes at most. `apply_remote_record`: a received version of equal content
settles a queued change of the record (its ciphertext replaces the row and the change
leaves the outbox), and a dominated one no longer raises the stored `server_seq`.
`SyncWarning` gains `ServerInconsistent { vault }`, `ServerRollback { vault }` and
`NoCommonView { device, vault, since_ms }`;
`TransportError` gains `HeadBelowCursor`; `HttpTransport::connect(base, token, install)`
takes the engine's install ID; `wire::origin::canonical_origin` is the shared origin
normalizer; `PasswordFinishRequest` loses `expected_account_version` (the server uses the
account version of the session's login).

## Expected additions (pre-approved as additive)

- M3: done (ADR-0028): quick unlock and hardware-key unlock on
  `LockedSession` (§6.2, §6.3), TK unlock of a hardware-protected TOTP vault
  (§7.4). Still to come: a §6.6 re-key method on top of
  `Store::rekey_account`, import helpers, a session method that hands an
  item's OTP code and fields to the extension one credential at a time
  (brief §4).
- M5: `mosaic-passkey` on top of `PasskeyCredential` and
  `ItemEdit::set_passkey_sign_count`.
- M6: the upload queue for wrapped vault keys and vault metadata, the
  checkpoint keys and the sync session API landed in T1 (above); the engine,
  the folder transport, the server, the HTTP transport and the desktop
  integration follow in T2 to T5. Sharing and emergency access (§8) move to
  M6b (ADR-0042).
