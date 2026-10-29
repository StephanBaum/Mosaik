# Import fixtures

Synthetic export files for `crates/import/tests`. They hold no real data:
every name, password, card number and key is made up.

| File | Written by | Notes |
|---|---|---|
| `enpass.json` | hand | Enpass 6 JSON layout: every category the importer maps, field types, deleted and section fields, folders, tags, attachments (one valid, one without data, one with bad base64), a non-object and an empty item |
| `bitwarden.json` | hand | Bitwarden unencrypted JSON: types 1 to 5 and an unknown one, custom field kinds, password history, folders and collections, a passkey, a Steam TOTP |
| `1pux-export.data.json` | hand | The `export.data` of a 1PUX archive; `tests/formats.rs` zips it with two attachment files |
| `aes-argon2d.kdbx` | pykeepass 4.2.0 | KDBX 4.0, AES-256-CBC, Argon2d (1 MiB, 2 passes), gzip, ChaCha20 inner stream; password `fixture-password` |
| `chacha-argon2id.kdbx` | pykeepass 4.2.0 | The same database with ChaCha20 and Argon2id |

The KDBX files come from an independent implementation, so they check the
reader against more than this crate's own test writer
(`src/kdbx/tests.rs`). They were made with `create_database`, the KDF
parameters lowered in `kdf_parameters`, then `add_group`, `add_entry`
(with tags, URL, notes, `otp`, a protected and a plain custom property,
`save_history` twice, an attachment) and `trash_entry`, and saved once.
