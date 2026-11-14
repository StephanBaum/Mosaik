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
| `UnlockedSession::adopt_remote_password(JoinKeys, &WrappedKey, PasswordParams)` → `PasswordChanged` (M6 T5) | Recovers a master password change that the backend holds and this device missed (CRYPTO_SPEC §9.4, "Change applied elsewhere"). The `JoinKeys` come from `derive_join_keys` over the backend's parameters; the wrapping must open under them to this session's own account key (constant-time equality), else `UnlockFailed` and nothing is stored. Stores like `commit_master_password_change` (quick unlock removed in the same transaction). Refuses a salt this device retired (`Error::PasswordRollback`). |
| `UnlockedSession::{folder_account_version, raise_folder_account_version}` (M6 T5) | The highest folder account-file version this device wrote or adopted (monotonic); a folder file at or below it is a rollback. |
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
  accepts one key per generation; on the hosted backend a device whose own
  key the server never accepted adopts the server's key with
  `adopt_remote_vault_key`, in a folder the key of the lowest writer wins and
  the loser rotates again; ADR-0013, ADR-0045, CRYPTO_SPEC §10.2); records
  sealed under the other key fail with `ItemCorrupted` meanwhile and change
  nothing.
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
writes; 6 since the M6 T5 verify follow-up: v6 adds `retired_salts` (salts of replaced master passwords, public data) and `folder_account_version`, additive and STRICT) are stable for every consumer (apps open the store and hand it to
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
  `into_json_response(CountPersisted) -> Result<AssertionResponse>` (the deprecated `into_response` returns the
  response without the hash-mode check). `CountPersisted` is a
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
`test-util`; `opaque` (the OPAQUE ciphersuite module `wire::opaque`, shared by the
server and the client) and `http` (the HTTP transport and the hosted client
functions, enables `opaque`) are real since T3 and T4, ADR-0046.

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
`put_view_compare` hold the comparison times, with the peer's and this device's own checkpoint counter in
`counter` and `own_counter` (store v12; the migration unpacks the two counters an earlier build packed into `counter`). `mosaic_core::sync_auth` gains
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

**M6 T3 to T5 (hosted client and desktop additions, additive).** `wire::opaque`
(feature `opaque`: `Suite`, the context and identifiers of CRYPTO_SPEC §9.1, an
`OsRng` over `getrandom`, `opaque_ke` re-exported as `ke`), shared by
`mosaic-server` and the client. `wire::origin` is the one origin normalizer.
`http::{HttpTransport, Token, StoredToken, canonical_origin}` (feature `http`):
`Token` is implemented by `SessionToken` and `StoredToken` (a `SessionToken`
cannot be rebuilt from bytes, so the desktop's keystore holds a `StoredToken`;
`connect` and the hosted functions take either). `http::hosted`: `prelogin`,
`register`, `login` (`LoginGrant`), `login_totp`, `logout`, `join_login` and
`join_fetch_account` (`JoinLogin`, `JoinAccount`), `password_change`
(`PasswordChanged`), `recover_password` (`Recovered`; the hosted half of
`UnlockedSession::adopt_remote_password`), `account_info` (`AccountInfo`),
`totp_setup` (`TotpSetup`), `totp_confirm`, `totp_disable`, `device_rename` and
`device_remove`. `FolderTransport::verified_account_file` returns the newest
account file whose MAC verifies (the folder half of the password recovery).
`SyncEngine::{halted, clear_halt, clear_findings, finding_vaults}` wrap the
session's `clear_sync_halt`, `clear_vault_findings` and `sync_findings`; the desktop reads
`sync_findings()` and acknowledges both behind one action.

## M6b: sharing and emergency access (core, vault, sync; additive)

Everything below is new in M6b and additive. Nothing of the M2 freeze changes
meaning, with one exception that is not part of the frozen surface:
`sync_auth::view_digest` was retired by `view_digest2` (below) and removed in the M6b review.

### `mosaic-core` (T1)

- `identity`: `IdentitySecret` (zeroizing, `generate`, `public_key`, `wrap`),
  `IdentityPublicKey` (32 bytes; `derived_from(ikm)` is the public half of
  `DeriveKeyPair(ikm)`, used by the server for the directory's fake keys),
  `Fingerprint` (`of`, `from_bytes`,
  `as_bytes`, `safety_code`, `matches_code`; constant-time equality),
  `WrappedKey::unwrap_identity`.
- `share`: `ShareSeal` (82 bytes, strict `from_bytes`), `seal_vault_key`,
  `open_vault_key`, `seal_emergency_vault_key`, `open_emergency_vault_key`
  (HPKE mode Auth, no new primitive; CRYPTO_SPEC §8.2, §8.4).
- `envelope`: `VaultRosterCiphertext` (`seal`, `open`, `key_generation`),
  `ContactsCiphertext`, `AccountKey::contacts_key`.
- `model`: `VaultRoster` (bound to its generation; `merge_and_repair`,
  `validate`, `authorizes_rotation`: only the vault owner with the pinned
  fingerprint may issue a rotation share), `RosterMember`, `RosterEdit`,
  `Role`; `ContactBook`, `Contact`, `ContactState`.
- `sync_auth`: `view_digest2`, `view_meta_commit`, `ServedObject`,
  `ABSENT_OBJECT` (the digest of CRYPTO_SPEC §10 with the metadata and roster
  commitment, R70). The label `sync/view` stays in the registry (retired,
  never reused); the old `view_digest` function is gone.
- New labels in `encoding::label` (CRYPTO_SPEC §3).

### `mosaic-vault` (T2; `session`, restricted to the sync client and the desktop)

`UnlockedSession`: `ensure_identity_key`, `identity`, `mark_identity_published`,
`adopt_remote_identity`, `own_fingerprint`; `record_sharing_finding`,
`sharing_findings`, `clear_sharing_findings` (`SharingFinding`);
contacts: `contact_state`, `pin_contact`, `verify_contact`,
`accept_key_change`, `contacts` (`verify_contact` by the owner also writes the member's new fingerprint into the owned vaults' rosters), `apply_remote_contacts`,
`pending_contacts_upload`, `mark_contacts_uploaded`; roster and roles:
`vault_shared`, `vault_own_role`, `vault_owner`, `vault_access_lost`,
`roster_seq`, `vault_roster`, `apply_remote_roster` (takes the server's
owner), `pending_roster_upload`, `mark_roster_uploaded`, `set_member_role`,
`mark_access_lost`; invites: `prepare_invite` (`PreparedInvite`),
`commit_invite_sent`, `open_invite` (`IncomingInvite`, `InvitePreview`),
`accept_invite`, `commit_invite` (`AcceptedInvite`), `check_vault_unknown`
(`Error::VaultKnown`: no key from an invite or release for a vault the device
knows), `sweep_pending_invites`, `ensure_owner_roster` (the owner-only roster
and sharing row a grant needs; M6b review); `InvitePreview` carries
`owner_code` and `owner_state`; `SharingFinding::MemberUnverified`; revocation:
`request_revocation`, `pending_revocations` (`RevocationReason`),
`prepare_shared_rotation` (`RotationPlan`, `PreparedSharedRotation`; owner only),
`commit_shared_rotation` (`SharedRotationOutcome`); the claim rule:
`open_share_chain` (`PulledShareView`, `OpenedChain`), `claim_wrap`,
`commit_claimed_key`; emergency access: `seal_emergency`,
`open_emergency_release`. `InviteId` is a UUIDv7. Store schema v8
(`identity_key`, `contacts_doc`, `vault_sharing`, `vault_roster`,
`pending_revocations`, `sharing_findings`); `reset_sync_state` never clears the
shared flag, and a backend change is refused while shared vaults are live
(`storage::Error::HasSharedVaults`). `adopt_remote_vault_key` refuses a shared
vault (`Error::AdoptionForbidden`) and `prepare_vault_rotation` too
(`Error::VaultShared`).

### `mosaic-sync` (T4)

- `sharing` (new module): `SharingTransport`, one method per route of
  `docs/SERVER.md` (identity, directory, contact book, invites, members,
  roster, rotation, claim, leave, blocks and the emergency routes), with
  `DirectoryEntry`, `IdentityPut`, `CasResult`, `ListedInvite`,
  `OutgoingInvite` (`InviteState`), `MemberInfo` (`MemberOrigin`),
  `PendingInvite`, `MembersView`, `AcceptResult`, `NewInvite`,
  `RotateRequest`, `RotateResult`, `GrantInfo`, `ReceivedGrant`
  (`GrantScope`, `GrantState`).
- `transport`: `Transport::sharing(&mut self) -> Option<&mut dyn SharingTransport>`
  (default `None`; the folder has none, CRYPTO_SPEC §8.6; wrappers pass the
  inner answer on). `PullPage` gains `shares`, `rosters` and `role`
  (`PulledShare`, `PulledRoster`) and derives `Default`. `Rejection` gains
  `RoleForbidden`, `MembersMismatch`, `IdentityExists`, `Conflict`,
  `InviteExists`, `InviteStale`, `InviteExpired`, `InviteLimit`,
  `AlreadyMember`, `ReinviteBlocked`, `OwnerCannotLeave`, `NoShare`,
  `EmergencyWaiting`, `EmergencyCooldown`, `EmergencyReleased`,
  `EmergencyNotRequested` and `EmergencyOwnerOnly`. A removed member's pull,
  push and meta calls are `Forbidden` or `NotFound` (there is no `not_member`
  code on the wire).
- `wire::json::sharing`: the JSON bodies of the sharing routes (mirrors the
  server's `api/wire.rs`), `SharingChangesResponse` (the page of
  `GET .../changes?sharing=1`), and the new error codes in `wire::json::codes`.
  `HttpTransport` always asks for `?sharing=1` and `GET /v1/account?identity=1`.
- `wire::checkpoint`: `view_digest(vault, high_seq, key_commit, meta_commit, rows)`
  is the version-2 digest (`sync/view2`); `view_meta_commit`, `ServedObject` and
  `ABSENT_OBJECT` are re-exported. The metadata and the roster the server
  served or accepted are noted in the served view under two reserved record
  IDs (`engine::served`), because a merge of equal content may re-seal the
  metadata under a new nonce.
- `engine`: `SyncReport` gains `invites_incoming`, `emergency_requests`,
  `rotated_vaults`, `claimed_vaults` and `unreachable_attachments`;
  `SyncWarning::RotationBlocked`; `SyncError::SharingUnavailable`. The run
  order is: identity and contact book, pull (each vault pull ends with the
  sharing step: rosters of the held generation, the claim rule for key shares),
  invites, queued roster uploads, the owner's revocation rotations, emergency
  seals, then push. `engine::keys::decide` never adopts and never ranks for a
  shared vault. A shared vault is rotated only by its owner's device
  (`SyncEngine::rotate_vault` calls the owner path and fails with
  `Session(NotOwner)` elsewhere).
- `SyncEngine` user operations (hosted only; `SharingUnavailable` on a folder):
  `lookup_contact` (`ContactLookup`), `invite_member` (`InviteSentInfo`),
  `list_invites`, `preview_invite`, `accept_invite`, `decline_invite`,
  `cancel_invite`, `vault_members` (`VaultMembers`, `MemberView`,
  `MemberState`), `set_member_role`, `remove_member`, `leave_vault`,
  `emergency_overview`, `grant_emergency`, `revoke_emergency`,
  `request_emergency`, `respond_emergency`, `claim_emergency` and
  `sharing_findings`.

## M7: key rotation (core, vault; additive)

Everything below is new in M7 and additive, apart from the three changes called
out (`change_credentials` takes the stored wrapping, `adopt_remote_rekey` takes
one struct, and the claim rule compares the owner entry of the new roster).

### `mosaic-core` (T1 and its review follow-ups)

- `keys`: `CredentialChange`, `change_credentials(ak, current_password,
  current_sk, current_params, current_wrapped, change)` (one prepare path for a
  password change, a Secret Key rotation and a KDF profile upgrade; always a new
  salt; verifies the current password and Secret Key against the stored wrapping
  first, `Error::Decryption` otherwise; weaker profile: `ProfileDowngrade` before
  any Argon2id work). `change_master_password` stays and does **not** verify the
  current password. `RekeyedAccount`, `rekey_account`.
- `SecretKey::retired_commitment(account)` and `SecretKey::is_retired(account,
  &[commitment])` (constant time): the retired-Secret-Key commitment of CRYPTO_SPEC
  §6.7, label `secret-key/retired`. `SecretKey` has constant-time equality.
- `sync_auth`: `FolderMacPurpose::Rekey`, `folder_rekey_mac`,
  `verify_folder_rekey_mac`.

### `mosaic-vault` (T2; `session`, restricted to the sync client and the desktop)

Store schema **v9** (strict, additive; `LATEST_VERSION = 9`): `account.ak_epoch`
(trigger: never decreases), `identity_key.epoch` (a published identity key changes
only in a statement that raises it: the v8 freeze triggers are recreated),
`rotation_job` (one row, never a secret), `reseal_attachments`,
`folder_rekey_floor`, `retired_secret_keys` (permanent), `rekey_dismissed_files`,
`sync_engine_state.rekey_pending`, and `pending_revocations` rebuilt with reason 6
(`RevocationReason::MemberRekeyed`). `reset_sync_state` clears the floors and the
dismissed files, never `ak_epoch` or the retired commitments. `set_sync_config`
refuses a backend change while a job exists.

`UnlockedSession`:

- Epoch and versions: `ak_epoch`, `check_backend_epoch(offered)`
  (`Error::EpochRollback` below the device's), `highest_account_version`,
  `check_account_version` (`Error::AccountVersionRollback`),
  `note_account_version`, `retired_secret_keys`, `rekey_pending`,
  `set_rekey_pending`. `LockedSession::ak_epoch`.
- Credentials: `prepare_credential_change(current_password, current_sk,
  &CredentialChange)` → `PreparedPasswordChange` (the commit is
  `commit_master_password_change`: quick unlock deleted, hardware-key wrappings
  kept, the replaced Secret Key's commitment kept, a kind 1 or 3 job moved to phase
  5). A new Secret Key that is the current or a retired one: `Error::SecretKeyUnchanged`.
  `prepare_master_password_change` calls it.
- Re-key: `rekey_review` → `RekeyReview`; `prepare_rekey(RekeyRequest)` →
  `PreparedRekey` (`RekeyPlan` with `seen_epoch`: the target is `max(own, seen) + 1`; `FolderRekeyPlan`; accessors `old_opaque_password`,
  `new_opaque_password`, `new_params`, `new_wrapped_ak`, `to_epoch`, `identity`,
  `vault_keys`, `shared_rotations` (`SharedRekeyOut`), `member_rewraps`,
  `emergency_seals`, `folder_marker`); `commit_rekey` → `RekeyCommitted` (one
  store transaction; the session holds the new AK afterwards).
  Errors: `PasswordUnchanged`, `SecretKeyUnchanged`, `RekeyNotClean`, `PlanMismatch`,
  `EpochExhausted`, `PreparedStale`.
- Adoption: `adopt_remote_rekey(RekeyAdoption)` → `RekeyAdopted`. Epoch
  `current + 1` (hosted), or any epoch above the current one when
  `any_higher_epoch` is set (folder; equal: `UnlockFailed`, lower: `EpochRollback`,
  above `own + 16`: `EpochJumpTooLarge`, else `EpochNotNext`); every owned vault a
  backend has seen is held at its generation until the next
  one arrives (`rekey_held(vault)`, table `rekey_hold`);
  refuses a retired salt (`PasswordRollback`) and a typed Secret Key that is the
  current or a retired one (`SecretKeyUnchanged`); stores the backend's own wrapping
  of each member vault's current key after checking it opens to the held key. There
  is no adoption at an equal epoch.
- Join: `LockedSession::join_account_with_keys_at(wrapped, keys, epoch)` stores the
  backend's epoch (only rises); `raise_epoch_to(epoch)` (test-only, ADR-0064). `member_vault_generations()`
  lists the held vaults shared to the account without a clean-sync check.
- Attachments: `reseal_next_attachment(Option<&[u8]>)` → `ResealStep` (`Unreadable`
  marks the attachment unreachable and goes on), `queue_adoption_reseal` (resume),
  `abandon_committed_rotation_job` (after a confirmation),
  `mark_attachment_unreachable`, `unreachable_attachments`,
  `queue_vault_attachment_reseal`.
- Journal: `rotation_job` (`RotationJob`, `RotationKind`, `RotationPhase`),
  `begin_rotation_job`, `set_rotation_phase`, `finish_rotation_job`,
  `abandon_rotation_job`; folder: `folder_rekey_floors`, `note_folder_rekey_floor`,
  `mark_folder_rekey_snapshot`, `dismiss_unverifiable_file`, `file_dismissed`.
- Errors added: `EpochRollback`, `EpochNotNext`, `EpochExhausted`,
  `PasswordUnchanged`, `SecretKeyUnchanged`, `RotationInProgress`, `RekeyNotClean`,
  `AccountVersionRollback`; storage: `RotationInProgress`, `NoRotationJob`,
  `RotationNotFinished`, `InvalidRotationPhase`, `RotationTooFar`, `EpochMismatch`.
- Behaviour change in the claim rule (`open_share_chain`): the owner entry of the
  **new** roster must name the key that sealed the share (before: the held roster's
  entry), because an owner's re-key replaces that key; the verified contact pin is
  still required (THREAT_MODEL R84).
- `storage` (restricted): `Store::{ak_epoch, rotation_job, begin_rotation_job,
  set_rotation_phase, finish_rotation_job, abandon_rotation_job,
  queue_attachment_reseal, reseal_rows, next_reseal_todo, mark_reseal_unreachable,
  mark_reseal_obsolete, swap_attachment, commit_rekey, adopt_rekey,
  folder_rekey_floors, note_folder_rekey_floor, mark_folder_rekey_snapshot,
  rekey_pending, set_rekey_pending, retired_secret_keys, dismiss_unverifiable_file,
  file_dismissed, update_password_retiring}`.

`SyncHalt` is unchanged: the re-key marker sets the `rekey_pending` flag, not a halt.

### `mosaic-sync` (T4: account re-key in the engine, hosted and folder)

- `wire::account_file`: `AccountFile` gains `ak_epoch`; `encode` writes v2 (172
  bytes, `LEN`), the decoders also read v1 (168 bytes, `LEN_V1`) as epoch 1.
  `wire::rekey_marker` (new): `RekeyMarker`, `RekeyMarkerPath`, `LEN` (93 bytes);
  `decode` verifies the MAC under the keys behind the `SyncCrypto` it is given (the
  epoch the marker leaves). `wire::folder_names`: `FolderName::RekeyMarker`,
  `rekey_marker_name`. `SyncCrypto::ak_epoch()` (default 1; the session
  reports its epoch); `KeyedCrypto::with_epoch`. Byte vectors: the script and checked-in
  output in `crates/sync/tests/vectors/` (`sync_kat.py [--check]`, `sync_kat.txt`).
- `wire::json::rekey` (new): `RekeyStartRequest`, `RekeyFinishRequest`
  (`RekeyVaultEntry`, `RekeyMemberRewrap`, `RekeyEmergencySeal`),
  `RekeyFinishResponse`, `AccountWithEpoch` (`GET /v1/account?epoch=1`,
  `akEpoch` and optional `identity`), `EpochMismatchDetail`, `RekeyMismatchDetail`;
  codes `EPOCH_MISMATCH`, `PROFILE_DOWNGRADE`, `REKEY_MISMATCH`.
  `SharingChangesResponse.rotation_requested` (`?sharing=1&rotation=1`, the owner's
  page). `transport::Rejection` gains `EpochMismatch { current_epoch }`,
  `ProfileDowngrade` and `RekeyMismatch { set: RekeySet }`; `PullPage` gains
  `rotation_requested`.
- `transport`: `Transport::{rekey_signals(epoch), rekey_candidates(epoch, limit),
  next_folder_account_version, publish_rekey(cx, &RekeyPublish)}` (defaults: nothing;
  the folder implements them; wrappers pass them on) with `RekeySignals`,
  `RekeyMarkerSeen`, `UnverifiedAccountFile`, `RekeyCandidate`, `RekeyPublish`.
  `MaintainContext` gains `rekey_floors` and `mark_rekey_snapshot` (defaults: none).
- `http::hosted`: `rekey(base, address, &PreparedRekey, device, name, totp, on_stage)`
  (fresh login, `rekey/start`, atomic `rekey/finish`, lost-answer probe by byte-equal
  wrapped AK at the target epoch; `RekeyStage::{Started, Accepted}` drive the
  journal; `RekeyOutcome::{Done(RekeyFinished), Refused(RekeyRefusal)}`),
  `recover_rekey(&RecoverRequest)` (login with the typed new password and Secret
  Key, `GET /v1/account?epoch=1&identity=1`, the key rows the server serves for the
  member vaults; `RekeyRecovered`), `account_epoch`.
- `engine`: `SyncError::AccountRekeyed` (a verified `.mrek` for this device's epoch;
  the persisted re-key-pending flag, not a `Halt`; nothing is written or purged while
  it is set; checked before any write), `SyncWarning::UnverifiedAccountFile { name,
  sha256, epoch }` (an account file that does not verify and names this epoch (after a
  re-key) or a higher one: a finding, never a halt; `SyncEngine::dismiss_account_file`
  remembers it by name and SHA-256), `RekeyError`, `RekeySecrets`, `RekeyProgress`.
  `SyncEngine::rekey_hosted(access, clock, &HostedRekey, &RekeySecrets)` →
  `HostedRekeyDone` (run to head; a strict plan that locks nobody out; prepare; send;
  commit; at most three attempts on `head_moved`/`generation_taken`; the transport
  must be connected with the returned grant afterwards), `rekey_folder(access, clock,
  &RekeySecrets)` (prepare builds the marker, commit, then the v2 account file at the
  highest verified version + 1, a snapshot set of every vault, the marker and the
  device info), `rekey_continue` (finishes a folder publication, re-seals attachments
  within the download budget, uploads, closes the job; `RekeyProgress`),
  `rekey_finish_if_clean`, `resume_hosted_rekey(access, &HostedRekey, new_password,
  pending_secret_key, current_secret_key, typed_last_group)` → `ResumeOutcome::{Adopted,
  NotApplied}` (**the user retypes the last group of the new Secret Key**:
  `RekeyError::KitGroupMismatch` otherwise; the desktop owns that screen),
  `adopt_hosted_rekey(access, &HostedRekey, password, typed_sk, current_sk, device)`
  and `SyncEngine::adopt_folder_rekey(access, clock, password, typed_sk, current_sk)`
  (candidates ranked: the file a verified marker names first, then by version, at
  most eight Argon2id derivations; the MAC is checked under the key the typed
  credentials open before the vault adopts; floors of the own logs are recorded
  first; this device then writes its own snapshot sets). Adoption is only at epoch
  + 1; an equal-epoch conflict is resolved by re-keying again from a device that
  holds the current AK.
- Folder retention (CRYPTO_SPEC §10.1): own segments up to a vault's re-key floor
  are never removed by the 24-hour rule or by compaction; they go when the vault's
  `snapshot_done` is set and `BLOB_RETENTION_MS` (30 days) passed since `rekey_at`.
  The folder transport drops its verification cache when the epoch changes, and
  the own-log rollback check skips vaults whose floor has no snapshot yet.
- `SyncWarning::RotationBlocked` is also raised for a rotation a member's re-key
  asked for (`rotation_requested`) when a member's key is unverified or does not match
  the roster (the plan is strict: nobody is locked out, no invite is cancelled).

### M7 verify follow-ups (ADR-0064)

- `mosaic_core::sync_auth::MAX_EPOCH_JUMP` (16). `RekeyMarker::{encode, decode}`
  reject a `to_epoch` more than 16 above `from_epoch` (`WireError::Field`).
- `mosaic_vault::session::Error::EpochJumpTooLarge { stored, offered }`: a folder
  adoption above `own + 16`, or a re-key target above it (`prepare_rekey`; the
  engine never plans one). The app offers to set the device up again (join).
- `UnlockedSession::raise_epoch_to` is test support only (feature `test-util` of
  `mosaic-vault`, which `mosaic-sync/test-util` enables); a join uses
  `join_account_with_keys_at`. `UnlockedSession::undismiss_file(path, sha256) -> bool`.
- `abandon_committed_rotation_job` needs phase 5 or later and, for a re-key, the
  store at the target epoch (`RotationTooFar` otherwise; phases 1 to 4 are
  resume-only).
- An adoption holds only owned vaults a backend has seen (`Store::vault_backend_known`,
  column `vaults.backend_known` in schema v9, never cleared).
- `SyncEngine::dismissed_rekey_markers(access) -> Vec<PendingRekeyMarker>` and
  `SyncEngine::undismiss_rekey_marker(access, name, sha256) -> bool` (sets the
  re-key-pending flag again).
- Folder scans: up to 1024 marker files of the epoch are MAC-checked (at most 64
  verified kept); the adoption fallback reads up to 1024 account files and offers
  only epochs in `(own, own + 16]`.

## M7b: credential rotation (vault; additive)

### `mosaic-vault` (T2; `session`, restricted to the desktop and the extension channel)

Store schema **v10** (strict, additive; `LATEST_VERSION = 10`): `breach_state`
(item, field 1 login or 2 Wi-Fi password, the Lamport value mark, `checked_at`,
`found`), `breach_scan` (one row: the last scan's times, count, error 1 to 4) and
`rotation_queue_state` (snooze time, dismissed mark). No password, hash or hash
prefix is stored (THREAT_MODEL R97). Rows go with a purged item (triggers on
`records`: a deleted row or a tombstone, local or received) and with a removed
vault (cascade); `reset_sync_state` and the M7 re-key keep them. A row counts only
while its mark equals the item's current `Item::secret_mark`; a mark means
something only per vault, item and field.

New dependencies of the crate: `mosaic-origin` (exact-origin matching) and
`mosaic-insights` (the breach types). `Error::ReadOnly` (a viewer of a shared
vault; every item change below) and `Error::NoPassword` (dismissing an item
without a password).

`UnlockedSession`, all through `update_item` (one edit, one sealed record, one
outbox entry, the normal conflict rules; each returns after the local commit):

- `begin_rotation(vault, item, &SecretString, Option<RulesSource>) ->
  RotationAttemptId`: stores the new password in the item before any site sees
  it; the password is untouched. `Error::Core(NotALogin | Malformed |
  TooManyAttempts)` come from the item.
- `mark_rotation_delivered(vault, item, attempt)`: no write when already
  delivered; `Error::Core(RotationNotFound)`.
- `close_rotation(vault, item, attempt, RotationOutcome::{Succeeded, Failed})`:
  `confirm_rotation` or `fail_rotation` as one edit; no value that was ever an
  attempt's is deleted (CRYPTO_SPEC §7.4).
- `restore_password_history(vault, item, &HistoryId)`: the rollback.
- `set_rotation_policy(vault, item, Option<u16>)`: 30, 90, 180, 365 or `None`.
- `rotation_attempts_for_origin(&PageOrigin) -> Vec<RotationRef>`: logins with a
  URL for which `mosaic_origin::fill_allowed` gives `MatchKind::ExactOrigin`
  (no eTLD+1, no plain http without the item's override), outside the trash,
  newest first, at most `MAX_ATTEMPTS_FOR_ORIGIN` (5). `RotationRef` holds IDs,
  title, user name, stage and start time, never the value.

Local state (viewers may use it; it never touches an item):

- `breach_states() -> BreachStates`: the cached answers for the current values;
  stale rows and rows of gone items are dropped.
- `record_breach_results(&[BreachResult], ScanSummary)`: one transaction; an
  answer whose mark no longer matches is skipped. `breach_scan() ->
  Option<ScanSummary>`; `ScanSummary { started_at, finished_at, checked, error:
  Option<ScanError> }` (`ScanError` converts from `RangeError`).
- `snooze_rotation(vault, item, until)`, `dismiss_rotation(vault, item)` (stores
  the current password mark) and `queue_states() -> QueueStates`
  (`QueueState { snoozed_until, dismissed }`, `hides(now)`; a dismissal whose
  mark changed is not reported).

`storage` (restricted): `Store::{breach_rows, put_breach_results, breach_scan,
queue_rows, set_snooze, set_dismissed}`, `BreachRow`, `ScanRow`, `QueueRow`.

## M8: CLI session and encrypted export (vault, import, sync; additive)

The `mosaic` CLI opens a store directly, under the store's exclusive lock
(ADR-0071). `Store::open` already takes the advisory lock on `<path>-lock` and
fails at once with `storage::Error::StoreInUse` while the desktop (or another
process) holds the store: the CLI maps that to exit code 5. Nothing else in the
store API changes.

### `mosaic-vault` (`session`, `export`)

CLI session (CRYPTO_SPEC §6.8, ADR-0072). The vault layer neither stores nor
deletes the two parts (the OS keystore entry and the runtime-directory file are
`mosaic-platform`'s):

- `LockedSession::unlock_cli_session(self, &CliSessionKey, &CliSessionFile,
  now_ms: u64) -> Result<UnlockedSession, UnlockError>`: checks every rule of
  §6.8 (expiry, lifetime, clock skew, `session_id`, account, install ID,
  `ak_epoch` and salt of the store, the AEAD, and that a held vault key opens
  under the recovered key; a store without a vault key has nothing to check).
  Any refusal is `Error::CliSessionRejected` (one variant for every reason): the
  caller then deletes **both** parts. `Error::Store` and `Error::NoAccount`
  leave the parts alone. The returned session does **not** count as a password
  unlock (`password_unlocked` is false), so the quick-unlock rules that ask for
  the password still do.
- `UnlockedSession::create_cli_session(&self, ttl: Duration, now_ms: u64) ->
  Result<(CliSessionKey, CliSessionFile)>`: the **only** way to seal a session,
  and only for a session unlocked with the master password in this process
  (`create_account`, `unlock`, `join_account`). Everything else (a session
  opened from a CLI session, `unlock_quick`, `unlock_with_hardware_key`, a
  session that was locked and reopened any other way than with the password)
  gets `Error::ReauthenticationRequired`: a session never renews itself (review
  decision L3). A `ttl` outside 1 to 30 minutes is `Error::Core(InvalidTtl)`.
  The CLI calls it right after it prompted for the password.
- `LockedSession::account_id() -> Result<AccountId>`: names the keystore entry
  and the file before unlocking.
- The binding is computed from the store (account, install ID, `ak_epoch`,
  salt), so a password change, Secret Key rotation, profile upgrade or re-key
  ends every session by itself. The CLI and the desktop must still delete every
  `mosaic-cli-session-*` entry and file (`CliSessionStore::delete_all`) after
  any credential change they perform or adopt (R105, review decision H1b).

Encrypted export (CRYPTO_SPEC §11.2, ADR-0073):

- `UnlockedSession::write_export(&self, &ExportSelection, &mut dyn Write,
  &ExportPassphrase) -> Result<ExportReport>` streams the items of the selected
  vaults through `mosaic_core::ExportWriter`, one item at a time (an attachment
  in 64 KiB pieces); the export is never held in memory. Items in the trash and
  deleted items are left out. Write to a temporary file and move it into place
  only after `Ok` and a flush; after an error the output is a truncated stream
  that no reader accepts, so delete it.
- `ExportSelection { vaults: Option<Vec<VaultId>>, attachments: bool, profile:
  Argon2Profile }` (default: every openable vault, no attachment data, the
  standard profile; a profile below standard is `Error::Core(UnknownProfile)`).
  A named vault the session cannot open is `Error::VaultUnavailable`; with
  `vaults: None` such a vault is skipped and counted.
- `ExportReport { vaults, items, attachments, attachments_left_out,
  unsupported_passkeys, unavailable_vaults }`. Attachment data goes in only with
  `attachments: true` (OD-M8-7); otherwise each file is listed by name, type and
  size with `data: null`.
- `UnlockedSession::export_preview(&self, &ExportSelection) ->
  Result<ExportPreview>` (`vaults`, `items`, `items_with_attachments:
  Vec<(VaultId, ItemId)>`, `attachments`, `attachment_bytes`,
  `unavailable_vaults`): what the CLI shows before it warns about attachments.
- `mosaic_vault::export` holds the plaintext schema shared with the importer:
  `ExportItem`, `ExportField`, `ExportPasskey`, `ExportHistory`,
  `ExportAttachment`, `SecretText` (zeroizing, prints `[REDACTED]`), `FORMAT`,
  `VERSION`, `CUSTOM_PREFIX`. Its module documentation is the field-by-field
  reference. Deviations from the §11.2 sketch, all additive: `sshKey` is
  accompanied by `sshPublicKey` and `sshFingerprint`; the passkey credential
  carries `signCount`, `discoverable` and `createdAt` next to the CXF members;
  only ES256 passkeys are exported (another algorithm leaves `passkey` null and
  is counted); URLs are exported as text (an import uses exact-origin, HTTPS
  only); only the login password's history is exported. New dependencies of the
  crate: `serde` (derive), `serde_json` and `base64ct`, all already in the
  workspace.

### `mosaic-import`

- `mosaic_import::mosaic_export::{parse_reader(R: Read, &ExportPassphrase) ->
  Result<Import>, parse_bytes(&[u8], &SecretString) -> Result<Import>}`: reads
  the file as a stream, parses it item by item and returns the `Import` only
  after `ExportReader` reported the verified end of the stream, so a truncated,
  extended, reordered or modified file commits nothing (it is an error and no
  entry leaves the module). Errors: `Error::WrongCredentials` (wrong passphrase
  or damaged file: the CLI's `exportUnreadable`), `Malformed` (not an export),
  `UnsupportedVersion` (an unknown version, algorithm or profile), `TooLarge`,
  `LimitExceeded`, `PasswordRequired` (empty passphrase). Preview, dedupe and
  `commit` are unchanged. Vaults are flattened into one list; `createdAt` is
  ignored (new items are stamped now). An attachment without data adds
  `EntryWarning::AttachmentMissing`.
- **No `Format::MosaicExport` variant yet.** `Format` is matched exhaustively by
  the desktop (`apps/desktop/src-tauri/src/adapters/import.rs`), which is
  outside this track; `Import::format` of an export is `Format::Cxf`. Add the
  variant together with the desktop arms and the `parse` dispatch when the
  desktop gets the export format.

### `mosaic-sync` (`engine`)

- `engine::OwnedSession`: an `Arc<Mutex<Option<UnlockedSession>>>` that is both
  the engine's `SessionAccess` and the transport's `SessionHandle`, for a caller
  that owns its session for the length of one sync (the CLI syncing its own
  store as its own device, ADR-0071). `new(UnlockedSession)`, `take() ->
  Option<UnlockedSession>` (locks every clone: later engine and transport calls
  fail with `Locked`), `is_unlocked()`. A poisoned lock fails closed.

## M10: mobile (core, vault, passkey, origin, sync, qr; new crates `mosaic-mobile` and `mosaic-mobile-ffi`)

Written by M10 T1 (spec-first) and updated by T2 with the signatures as built. T3a and T3b update the
`mosaic-mobile` and `mosaic-mobile-ffi` parts. CRYPTO_SPEC §6.2, §6.9 and §16 and ADR-0089 to ADR-0093 are
normative. Everything below is additive. The one exception the spec admits (ADR-0093), the `mosaic-passkey` input
change, was built **additively** in T2 (see below); the breaking form is still to come.

### `mosaic-core`
- `setup_code` (stable): `SetupCode { server, email, secret_key }`, `build(&SetupCode) -> Result<Zeroizing<String>>`,
  `parse(&[u8], canonical) -> Result<SetupCode>`, `MAX_SETUP_CODE_LEN` (1024), `SETUP_CODE_PREFIX`
  (`"mosaic-setup:1:"`).
  - The format follows CRYPTO_SPEC §6.9.7. The parser is strict, and a wrong checksum is `SecretKeyMistyped`; every
    other failure is `Malformed`. The Secret Key must be in display form.
  - Deviation from the spec: the server origin rule (`canonical_origin`) and the email normalization (§9.2) live in
    `mosaic-sync`, which depends on `mosaic-core`. `parse` therefore takes `canonical(server, email) ->
    Option<(String, String)>` and requires the code to carry exactly the returned forms. Use
    `mosaic_sync::setup_code::parse(bytes)` in apps. `build` refuses a server or email with `"`, `\` or a control
    character (`Malformed`), which is stricter than JSON needs.
  - `SetupCode` does not implement `Clone`, and `Debug` redacts `secret_key`.
- `random_public_bytes::<N>() -> Result<[u8; N]>` (additive, G1-14): `N` bytes from the OS CSPRNG for values that
  are not secret (pending-slot IDs, key-alias nonces). `mosaic-mobile` draws all its randomness through it and has
  no `getrandom` dependency.
- `encoding::label`: `MOBILE_SECURE_STORE` (`mobile/secure-store`) and `QUICK_UNLOCK_ANDROID`
  (`quick-unlock/android`), both in `label::ALL`.

### `mosaic-vault` (`session`, `storage`)
- `BootId::new(&[u8]) -> Result<BootId>`: 1 to 32 bytes, else `Error::InvalidInput` (a new variant of the
  non-exhaustive `session::Error`). `BootId::as_bytes()`.
- `UnlockedSession::record_boot_marker(&mut self, &BootId, now_ms) -> Result<()>`: only right after a
  master-password unlock of this session, else `ReauthenticationRequired`.
- `LockedSession::adopt_boot_marker(&mut self, &BootId) -> Result<bool>`: sets the app-start flag if and only if
  the marker matches (constant time) and a write transaction (the marker rewritten unchanged) succeeded; it
  refuses once five wrong re-authentications are stored (N2, N6). It never resets the failed-attempt counter.
- `LockedSession::require_password(&mut self)`: counter to the limit and the marker deleted, for a strict lock that
  finds the session locked already (N4).
- `UnlockedSession::note_failed_reauth(&mut self) -> Result<u8>` and `reset_failed_reauth(&mut self)`: the stored
  count of wrong master passwords at a re-authentication (`Store::reauth_failures`, `note_reauth_failure`,
  `reset_reauth_failures`; table `reauth_failures` of the unreleased v11, N2).
- `Store::boot_marker() -> Result<Option<StoredBootMarker>>`, `put_boot_marker`, `delete_boot_marker`,
  `StoredBootMarker { boot_id, recorded_at_ms }`, `MAX_BOOT_ID_LEN` (32).
- Store schema v11 (`boot_marker`), fixture `crates/vault/tests/fixtures/store_v10.sqlite`. The marker is deleted
  in the same transaction as every quick-unlock wrapping deletion (`remove_unlock_method` of a quick method, a
  password change, a Secret Key rotation, a profile upgrade, a re-key, an adoption), by
  `lock_requiring_password`, and when the failed-attempt counter reaches its limit. Enrolling a quick unlock keeps
  it. `lock_requiring_password` and `note_failed_hardware_attempt` return nothing; their delete falls back to
  overwriting the marker with a 32-byte ID that matches no boot (`Store::clear_boot_marker`), and
  `LockedSession::enforce_boot_marker_limit() -> Result<()>` retries it at the limit (G1-13). The hardware-key
  unlock path returns the store error when the marker can be neither deleted nor overwritten.
- Restricted to `mosaic-mobile`: the desktop and the CLI never call `record_boot_marker` or `adopt_boot_marker`.
  Their behaviour is unchanged.

### `mosaic-passkey` (additive in T2; ADR-0093)
- `ClientData<'a> { Json { origin: &'a str, challenge: &'a [u8] }, Hash(&'a [u8; 32]) }`.
- `CreateRequest { rp_id, client_data, user_handle, algorithms, user_verified }` and
  `GetRequest { rp_id, client_data, user_verified }`, with `create_request(&CreateRequest) -> Result<ClientCreated>`
  and `assert_request(&GetRequest, &PasskeyCredential, stored_count) -> Result<PendingAssertion>`.
- `PendingAssertion::into_json_response(CountPersisted) -> Result<AssertionResponse>` and
  `into_response_without_count()` refuse a hash-mode assertion (`InvalidInput`, G1-16); `into_response` is
  `#[deprecated]` (it returns an empty `clientDataJSON` in hash mode); the desktop and the import tests use
  `into_json_response`.
- `ClientCreated.client_data_json` and `ClientAssertionResponse.client_data_json` are `Option<Vec<u8>>` (`None` in
  hash mode). A pending assertion is released with `into_client_response(CountPersisted)` or
  `into_client_response_without_count()`; the sign-count discipline of ADR-0039 D5 is unchanged.
- The signed message is `authenticatorData ‖ hash`, where the hash is the given one or `SHA-256(clientDataJSON)`.
- `android_app_origin(&[u8; 32]) -> String`.
- **Deviation from ADR-0093:** the spec changes `CreateInput`, `GetInput`, `Created` and `AssertionResponse`
  in place. That breaks `apps/desktop` (`browser/passkey.rs` and two test files) and `crates/import/tests`, which
  are outside the T2 write scope. T2 therefore left the four types and `create`/`assert` exactly as they were
  (they now run through the new path in `Json` mode, with identical bytes: `json_mode_vectors_unchanged`) and added
  the types above next to them. The breaking form of the ADR is a follow-up that must change the desktop callers in
  the same commit.

### `mosaic-origin` (additive)
- `app`: `AndroidAppUrl::{parse, new, package, cert_sha256, canonical}`, `is_valid_package_name`,
  `app_fill_allowed(saved, package, certs) -> Result<MatchKind, Refusal>` (`certs` is the one current signer: more
  than one never matches, G1-9). A match is reported as
  `MatchKind::ExactOrigin` (no new variant: the desktop matches `MatchKind` exhaustively). Parsing is stricter than
  the `url` crate: the canonical form only.
- `android_browsers`: `is_trusted_browser(package, certs) -> bool` (exactly one current signer, which must be one
  of the browser's listed certificates), `privileged_browsers_json() -> &'static str`.
  The data is `data/android-privileged-browsers.json`: the release signatures of 14 packages (Chrome, Firefox, Edge,
  Brave, Samsung Internet, DuckDuckGo, Vivaldi, Opera) from Google's privileged-apps list, fetched 2026-11-11.
- `target`: `android_fill_target(&AndroidCaller) -> Result<FillTarget, TargetError>`,
  `ios_fill_target(IosIdentifier) -> Result<FillTarget, TargetError>`. `FillTarget::Web { origin, idn_unicode }` or
  `AndroidApp { package, certs }`. The look-alike check needs the saved hosts, so the caller runs `lookalike_of`.
  An iOS `Url` identifier keeps scheme, host and port and drops userinfo, path, query and fragment (the spec said
  `PageOrigin::parse`, which refuses a path).
- `fields`: `select_fill_fields(&[FieldNode]) -> FieldSelection { root_domain, fields, refused }` with
  `FieldRole::{Username, Password, NewPassword, Otp}` and `FieldRefusal::ForeignFrame`; at most 4096 nodes.
  `select_scoped_fill_fields(&[FieldNode], FieldScope::{Web, App})` is what the mobile core calls with the fill
  target's kind: `App` selects native nodes only, `Web` starts at the first node with a `web_domain` (M10-G2 round 2, N1).
  `mosaic_mobile::Core::select_fill_fields` takes the `FillTarget` as a second argument.
- `assetlinks`: `parse_asset_links(&[u8]) -> Result<Vec<Statement>, AssetLinksError>`,
  `app_is_delegated(statements, package, cert_sha256, Relation) -> bool`,
  `app_is_delegated_for_passkeys(..)` (either relation).
- New dependencies: `serde_json` and `base64ct` (both already in the tree).

### `mosaic-sync` (`http`, restricted to the engine, the desktop, the CLI and `mosaic-mobile`)
- `http::TlsTrust { Platform, Anchors(Vec<CertificateDer<'static>>) }` and
  `http::set_tls_trust(TlsTrust) -> Result<(), TransportError>`. The setting is process-wide and set once; the same
  value again is accepted, another one is refused (`Io`), as are no anchors, more than 1024 or an invalid anchor.
  `Platform` is not available on Android. `http::CertificateDer` is re-exported.
- `http::assetlinks::fetch(host) -> Result<Vec<u8>, TransportError>`: `NotFound` for a 404, `Protocol` for any other
  non-200, a redirect or a body over 64 KiB, `Unreachable` for a failed connection.
- `setup_code::parse(&[u8]) -> Result<SetupCode>` (the core parser with the sync origin and email rules).
- `rustls-platform-verifier` is a non-Android dependency.

### `mosaic-qr` (additive)
- `scan_setup_frames(&[Frame]) -> Result<Zeroizing<String>, ScanError>` returns the one `mosaic-setup:` payload
  (not parsed here) and drops every other payload unread. New `ScanError::NotSetupCode`. `Payload` and `classify`
  are unchanged (the desktop matches `Payload` exhaustively).

### `mosaic-mobile` (M10 T3; restricted: `mosaic-mobile-ffi` and its tests)
The logic of both phone apps behind plain Rust types. No UniFFI dependency, `#![forbid(unsafe_code)]`. One
[`Core`] per process; every method blocks. The method list mirrors the FFI surface of `docs/MOBILE.md` §2.
- **Construction and state.** `Core::open(CoreConfig, Arc<dyn SecureStorage>)`, `state() -> SessionState`
  (`NoAccount`, `Locked { email, secret_key_stored, quick_unlock }`, `Unlocked { account_id, email,
  quick_unlock_enrolled }`, `StoreClosed`, `StoreBusy`, `Busy`; `Busy` is an addition to the five states of the
  plan, so `state()` is total while a long call runs), `lock()`, `close_store()`, `reopen_store()`,
  `required_unlock_memory_bytes()`.
- **Locking rule.** The core's mutex is never held across Argon2id, the network, a `SecureStorage` callback or
  another core call. A long call takes the session out and the core reports `Busy`; a `lock()` meanwhile sets a flag
  and wins when the call stores its result. The lock order is always the session holder first, then the core's
  mutex. The lock flag is read and cleared under the mutex (G1-1). `lock()` wipes every `SecretValue` handed out
  (a value read before a lock and handed out after it is wiped and the call answers `locked`, G1-6), and drops the
  pending slots (quick-unlock enrolment, OTP setups, passkey ceremonies, fill sessions) and a pending join;
  `close_store()` alone keeps an unexpired join (G1-11). `state()` and `sync_overview()` never call the platform:
  presence of the Secret Key and the token is cached and asked with `SecureStorage::contains`, never `get` (G1-2).
  Five wrong master passwords at a re-authentication end the session and require the password (G1-3).
- **Onboarding and unlock.** `prelogin_check`, `join_hosted` (`JoinOutcome::{Unlocked, SecondFactor}`),
  `join_totp`, `join_webauthn_begin`, `join_webauthn_status`, `join_cancel`, `parse_setup_code`, `scan_setup_qr`,
  `unlock_password`, `unlock_quick`. The pending join holds no store and lives 5 minutes. A join over an existing
  account is `accountExists`.
- **Quick unlock.** `quick_unlock_enroll_begin` (verifies the master password, returns `W` once in a 120 s slot),
  `quick_unlock_enroll_finish`, `quick_unlock_remove`, `quick_unlock_platform_failed`; the platform parameters are
  in `quick_unlock::params` (strict decoder, `key_alias`, `quick_unlock_aad`). An enrolment that ran out, was
  invalidated or was removed by a password recovery shows as `QuickUnlockAvailability::Expired { key_alias }` until
  the platform deletes the key (`quick_unlock_remove` returns it).
- **Vault.** `vaults`, `items`, `search`, `item`, `reveal`, `otp_code`, `save_login`, `save_note`, `trash_item`,
  `set_favorite`, `generate_password`, `otp_from_text`, `scan_otp_qr`, `otp_pending_discard`. Lists and details
  hold no concealed value (`FieldView.value` is `None`); a secret leaves only as `Arc<SecretValue>`.
- **Sync.** `sync_overview`, `sync_now`, `recover_password`, `reset_device(ResetConfirm) -> ResetOutcome { state,
  key_aliases }`. Deviations: `reset_device` returns the platform keys to delete (the plan returned only the
  state), and `recover_password` has no second-factor argument, so an account with a second factor cannot recover
  on the phone (`syncTwoFactorRequired`).
- **Autofill (T3b).** `select_fill_fields`, `fill_target_android`, `fill_target_ios`, `fill_warning` (an addition:
  the look-alike warning needs the saved hosts), `fill_candidates`, `fill_values`, `fill_otp`,
  `privileged_browsers_json`, `link_android_app`, `asset_links_status`. `fill_candidates` returns `FillCandidates {
  session_id, candidates }`: Rust holds the resolved target for 5 minutes (dropped on lock), and `fill_values(
  session_id, item)` and `fill_otp(session_id, item)` take that ID instead of a target (G1-12; an unknown or expired
  session is `notFound`). They check the match again for the chosen login. `AndroidCaller.current_signers` holds
  the one current signer (G1-9). The iOS entry points refuse an Android core and the other way round (G1-5), and
  the asset-links fetch is refused in the extension context (G1-8). A login in a hardware-protected vault, a trashed login and an item that is not
  a login are never candidates. Matching on plain `http` without `allow_http` answers `insecureOrigin`.
- **Passkeys (T3b).** `passkey_android_{get,create}_{begin,finish}`, `passkey_ios_{candidates,assert,register}`,
  `passkey_identities`, `password_identities`; `passkeys::json` parses the Credential Manager request JSON strictly
  (at most 64 KiB, only `public-key` descriptors, a fuzz target) and builds the response JSON. An Android caller
  that is not on Rust's own browser allowlist cannot pass a platform origin, and any other app must be delegated
  by the relying party's `assetlinks.json` (cached 24 h in memory). A ceremony without user verification is
  `reauthRequired`, on iOS too (G1-4).
- **Settings.** `settings`, `update_settings(SettingsUpdate)`: `settings.json` with the MAC of CRYPTO_SPEC §11.1.
  Raising the quick-unlock age or turning the password identity store on needs `reauth_password`.
- **Setup code (T3b).** `parse_setup_code`, `scan_setup_qr` over `mosaic_sync::setup_code::parse` and
  `mosaic_qr::scan_setup_frames`.
- **Test hooks (feature `test-hooks`).** `Core::test_hooks()`: a local Baseline account with one vault, a clock
  offset, a work delay (makes the lock races deterministic) and a replacement for the asset-links fetch. The crate's
  own dev-dependency turns the feature on for `cargo test -p mosaic-mobile`; `scripts/mobile/check-no-test-hooks.sh`
  keeps it out of release libraries.
- **Errors.** `MobileError` has unit variants only; `code()` is the camelCase code and `error_codes()` lists them.
  The codes of §3.4 of the plan plus the sync transport codes (`syncServerUnreachable`, `syncServerInvalid`,
  `syncServerInsecure`, `syncServerMismatch`, `syncServerIncompatible`).

### `mosaic-mobile-ffi` (new; the binding shell, consumed only by the generated Kotlin and Swift code)
- `SecureStorage` (foreign trait) has `contains(name) -> Result<bool>` next to `get`, `put` and `delete`: the
  platform implements it as an existence check that reads and decrypts nothing (G1-2).
- `src/lib.rs` (the `MosaicCore` object, `SecureStorage` foreign trait, `SecretValue`, `mobile_error_codes`,
  `core_version`), `src/types.rs` (the mirror records and enums), `src/convert.rs` (the conversions) and
  `src/bin/uniffi-bindgen.rs`. Nothing else may be added (`scripts/mobile/check-ffi-crate.sh`).
- It keeps `#![forbid(unsafe_code)]` and the workspace lints (UniFFI 0.32.2 compiles under them, ADR-0089 probe a).
  It declares `zeroizing-alloc` as the global allocator, so every freed Rust heap block is wiped.
- `SyncOverview.last_error` crosses as the camelCase code (a string). `SecretValue` also exports `is_empty`.
- `scripts/mobile/bindgen.sh [--out DIR] [--release] [kotlin|swift|all]` generates the bindings into `$TEMP`
  (the generator needs feature `bindgen`); nothing generated is committed.
- It is not a Rust API for other crates.

## Expected additions (pre-approved as additive)

- M3: done (ADR-0028): quick unlock and hardware-key unlock on
  `LockedSession` (§6.2, §6.3), TK unlock of a hardware-protected TOTP vault
  (§7.4). Still to come: a §6.6 re-key method on top of
  `Store::rekey_account`, import helpers, a session method that hands an
  item's OTP code and fields to the extension one credential at a time
  (brief §4).
- M5: `mosaic-passkey` on top of `PasskeyCredential` and
  `ItemEdit::set_passkey_sign_count`.
- M6: done (T1 to T5, all described above): the upload queue for wrapped
  vault keys and vault metadata, the checkpoint keys and the sync session API,
  the engine, the folder transport, the server, the HTTP transport and the
  desktop integration. Sharing and emergency access (§8) are M6b (ADR-0042,
  described above).
