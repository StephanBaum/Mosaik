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

The importers below were written from public format descriptions and are unverified against a current export of the product (launch gate). Their synthetic files live inline in the tests named here, not in this folder:

| Export | Where | Covers |
|---|---|---|
| LastPass CSV | `tests/it/lastpass.rs` (inline) | login, plain secure note, structured notes (card, address, Wi-Fi, passport), `http://` URL |
| NordPass CSV | `tests/it/nordpass.rs` (inline) | password, note, credit_card, identity, folder and unknown types; additional URLs; custom fields as JSON |
| RoboForm and KeePassXC CSV | `tests/it/csv_presets.rs` (inline) | folder paths, root group, TOTP, a header with an extra column |
| Dashlane zip | `tests/it/dashlane.rs` (built in the test) | credentials, secure notes, payments (card and bank), ids, personal info, an unknown CSV member |
| Keeper JSON and CSV | `tests/it/keeper.rs` (inline) | typed `custom_fields`, folders and shared folders; the CSV has no header row |
| Proton Pass zip and JSON | `tests/it/proton_pass.rs` (inline) | two vaults, login with extra fields, alias, passkey, trashed, card, identity |
