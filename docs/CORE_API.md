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
| `LockedSession::unlock(&MasterPassword, &SecretKey)` | Password unlock (§6.1). Failures return `UnlockError { session, error }`, the locked session unchanged; `Error::UnlockFailed` does not say which secret was wrong. |
| `UnlockedSession::lock(self)` → `LockedSession` | Drops AK, every VK, the decrypted items and the search index (all zeroize). |
| `account_id`, `device_id`, `store`, `corrupt_records` | State and diagnostics. `device_id` is this session's writer ID, fresh for every unlock (ADR-0015, third M2 integration review). |
| `change_master_password(current, new, &SecretKey)` → `PasswordChanged` | §6.4; returns the new params and `OPW'` for M6. |
| `rotate_device_id` | After `DeviceIdReused` (ADR-0009 item 7); then apply the record again. The old ID stays suspect for the session, so the re-applied record keeps both writers' values (ADR-0015). |
| `create_vault(name)`, `vaults()` → `Vec<VaultSummary>`, `vault_meta`, `rename_vault`, `rotate_vault_key` | Vaults; names live only in encrypted metadata (`VaultSummary`'s `Debug` does not print them); key generations only go up, and a rotation re-seals and queues every readable record and the metadata (§7.1). |
| `item`, `items(&ItemFilter)` → `Vec<ItemRef>`, `search(query)` | Reading. `ItemFilter { vault, trash: TrashFilter, favorites_only }`. `ItemFilter` is `#[non_exhaustive]` (third M2 integration review): start from `ItemFilter::default()` and set fields, so a new filter is additive. Search covers title, user name, email, URLs and tags, in memory only. |
| `add_item(vault, &ItemType, edit)`, `update_item(vault, item, edit)` | Writing through `mosaic_core::ItemEdit` closures; one sync event per call; a failing closure stores nothing; password changes go to the history. |
| `set_favorite`, `trash_item`, `restore_item`, `purge_item`, `move_item` | Favorites, trash, "delete forever" (a tombstone), move to another vault (re-encrypted, new item ID, attachments re-sealed). |
| `add_attachment`, `attachment`, `remove_attachment`, `attachment_usage`, `MAX_ACCOUNT_ATTACHMENT_BYTES` | Encrypted files, 50 MiB each, 1 GiB per account (§7.3). |
| `otp_code(vault, item, unix_time)` → `OtpCode { code, seconds_remaining }` | Current code of an item's OTP configuration; HOTP advances and saves the counter first. |
| `apply_remote_record(&RemoteRecord)` → `ApplyOutcome` | Merge a received record (§10, ADR-0012): `New`, `FastForward`, `Merged { restored_fields, stamp_collision }`, `Ignored`, and (added in the third M2 integration review) `Split { conflict_copies }` for a merge above the item limit (ADR-0016; the count of conflict copies, each marked with `conflict_copy_of`; changed from `Split { conflict_copy: Option<ItemId> }` in the fourth M2 integration review, ADR-0016 amendment) and `ReplacedUnreadable` for a local row that could not be read and so had no rollback anchor (THREAT_MODEL R17). |
| `apply_remote_vault_key(vault, generation, &WrappedKey)`, `apply_remote_vault_meta(vault, &VaultMetaCiphertext)` | New vaults, rotations, renames from other devices. A newer generation re-seals and queues every readable record still below it; the key of a held generation again is a no-op, a different one is `Error::VaultKeyConflict` (ADR-0013), a lower one `Error::KeyGenerationRollback`. A vault unavailable because its key row is damaged is repaired by a wrapping that opens (ADR-0012 amendment). Metadata merges with `VaultMeta::merge_and_repair` (`DeviceIdReused`, `Merged { stamp_collision }` as for records), and metadata that replaces an unreadable local metadata row is `ReplacedUnreadable` (fourth M2 integration review, ADR-0012 amendment). |
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

### `otp` (stable)

`otp_from_config(&OtpConfig, hotp_counter)` → `mosaic_totp::Otp`,
`config_from_otp(&Otp, issuer, label)` and `config_from_otp_auth(&OtpAuth)`
→ `(OtpConfig, counter)`, `otp_auth_from_config(&OtpConfig, counter)` →
`OtpAuth` (for `otpauth://` export). Store the counter with
`ItemEdit::set_otp_counter`.

### `storage` (restricted: `session`, the M6 sync client, migrations, tests)

`Store::{open, open_in_memory, close, schema_version, integrity_check,
attachment_dir}` and `LATEST_VERSION` (the schema version this build
writes) are stable for every consumer (apps open the store and hand it to
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
restricted. Added in the third M2 integration review: `readable_vault_keys`
(the wrapped keys of a vault that decode, damaged rows left out);
`mark_uploaded` returns `false` for a late acknowledgement of a superseded
change instead of `StaleServerSeq`; `Store::open` resolves the path before
naming the lock file and the attachment directory. The store accepts only `mosaic-core` ciphertext types and
enforces the §7.1 and ADR-0010 rules itself. Schema v1 is final as of the
M2 integration commit: every change goes through a migration
(`LATEST_VERSION`), never an in-place edit.

Internal: `session::fuzzing` (`cfg(fuzzing)` only).

## mosaic-core

Stable for every consumer:

- **IDs:** `AccountId`, `VaultId`, `ItemId`, `AttachmentId`, `DeviceId`
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
  `OtpAlgorithm`, `PasskeyCredential`, `SshKeyMaterial`, `VaultMeta`
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
ADR-0008 amendment) with `parse` and `to_uri` for `otpauth://` URIs (the `uri` module is public only for these items and
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
`Error`, `Result`. Randomness is the OS CSPRNG only; callers cannot inject
an RNG (ADR-0011). Internal: the `fuzzing` module.

## Expected additions (pre-approved as additive)

- M3: quick unlock and hardware-key unlock on `LockedSession` (§6.2, §6.3;
  the store already keeps the wrappings), TK unlock of a hardware-protected
  TOTP vault (§7.4), a §6.6 re-key method on top of
  `Store::rekey_account`, import helpers, a session method that hands an
  item's OTP code and fields to the extension one credential at a time
  (brief §4).
- M5: `mosaic-passkey` on top of `PasskeyCredential` and
  `ItemEdit::set_passkey_sign_count`.
- M6: an upload queue for wrapped vault keys and vault metadata (today only
  records and attachments are queued; ADR-0012 consequences), checkpoints
  (§10), sharing and emergency access (§8).
