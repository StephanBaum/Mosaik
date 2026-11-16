# Onboarding Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new user moves their logins into Mosaic from any browser or common manager in a few guided steps, sees what will not come across, and is helped to remove the plaintext export afterwards.

**Architecture:** `crates/import` gains six sources (LastPass, NordPass, RoboForm, KeePassXC and Dashlane-credentials as CSV presets on the existing CSV reader; Dashlane ZIP, Keeper JSON/CSV and Proton Pass as new formats), a shared bounded ZIP reader and content-based `detect`. The desktop shell gets `import_detect`, `import_delete_file` and a Rust-side drop handler; the webview gets a source grid, per-source guides, a "will not come across" list and a cleanup step. Parsing, preview and commit stay the existing pipeline (`ItemDraft` → `Import::preview` → `commit`).

**Tech Stack:** Rust (mosaic-import, mosaic-origin, `zip` 9.5 already in the tree, Tauri v2), React + TypeScript (vitest), `packages/i18n` (EN/DE).

**Spec:** `docs/superpowers/specs/2026-11-15-onboarding-import-design.md` (read §4, §6, §7, §11 before any task).

## Global Constraints

- No new dependency. `zip` (=9.5.0, `deflate-flate2`), `serde_json`, `mosaic-origin` are already dependencies of `mosaic-import`.
- No new cryptography. PGP-encrypted Proton Pass, Dashlane `.dash` and password-protected Bitwarden exports return `Error::Encrypted` (spec §6, §11.2).
- Library code: no `unwrap`/`expect`/`panic!`/`println!`/indexing that can panic; `thiserror` errors; no value from the file in any error, warning or log; plaintext in `Str`/`Buf`/`SecretString` (zeroizing); `#![forbid(unsafe_code)]` stays.
- Every parser is bounded: `MAX_FILE_LEN` (256 MiB), `MAX_ENTRIES`, `csv::MAX_COLUMNS` (256), `json::MAX_DEPTH`/`MAX_NODES`, `MAX_FIELDS_PER_ENTRY`; no recursion on attacker-controlled depth.
- Each new preset/parser carries the comment `// unverified against a current export (launch gate)` on its header/field table (spec §11.3). Parsers are written from public format descriptions only; no code copied from GPL importers.
- Fixtures are synthetic (made-up names, `pw1`-style passwords, test card `4111111111111111`, TOTP secret `GEZDGNBVGY3TQOJQ`).
- UI strings only via `packages/i18n` (EN source, DE informal "du"); styling only via design-token CSS variables. No third-party logos: product names in text plus a generic icon.
- No Downloads watcher, no deep link or URL scheme, no import on mobile (spec §4.3, §11.1, §11.4).
- Build env for every Rust command: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2`. Scope tests with `-p <crate>` and a test-name filter; the full gate runs once, in Task 20.
- Per-task commits are WIP on the feature branch (Conventional Commits); the milestone lands as one squashed commit after Task 20's gate.

## Review Focus

1. **Header drift in a real export** (an extra, renamed or reordered column): the preset still matches (subset match) and every unknown column is kept as a custom field, never dropped. Test: Task 3 `header_drift_keeps_unknown_columns`.
2. **A CSV saved as UTF-16 or Windows-1252** (Excel re-save, old Windows tools): detection returns `None` and parsing returns `Error::Malformed` (shown as "format invalid"), never garbage items or a panic. Test: Task 10 `utf16_and_latin1_are_not_detected`.
3. **Deleting an export that is read-only, still open in the browser, or on a network share:** the delete fails, the file is still there, the UI says so with the file name and never claims success. Tests: Task 14 `overwrite_and_delete_refuses_read_only_and_keeps_the_file`, Task 17 `cleanup reports a failed delete`.
4. **The user picks one source but chooses another product's file** (or an unknown file): the detected source wins, a mismatch notice names it, and an unknown file is refused with the format error before any preview. Tests: Task 16 `isMismatch` cases, Task 17 `shows a mismatch notice`.
5. **No Downloads folder** (Linux without XDG dirs, redirected or deleted folder) or Downloads inside OneDrive (Windows Known Folder Move): the dialog still opens (at the OS default), and the cleanup screen names OneDrive. Tests: Task 13 `start_dir_ignores_a_missing_folder`, Task 14 `onedrive_known_folder_move_is_detected`.

---

## File map

| File | Responsibility |
|---|---|
| `crates/import/src/archive.rs` (new) | Bounded ZIP open/read shared by 1PUX, Dashlane, Proton Pass |
| `crates/import/src/csv.rs` | CSV reader made crate-visible (`rows`, `Named`), new presets, preset dispatch |
| `crates/import/src/lastpass.rs`, `nordpass.rs` (new, private) | Row builders for the two presets with per-row item types |
| `crates/import/src/dashlane.rs`, `keeper.rs`, `proton_pass.rs` (new, `pub mod`) | New formats |
| `crates/import/src/detect.rs` (new) | `detect(&[u8]) -> Option<Detected>` |
| `crates/import/src/draft.rs` | URL origin check, `full_name`, new `EntryWarning::UrlKeptAsText` |
| `crates/import/src/lib.rs` | `Format` variants, dispatch, re-exports |
| `crates/import/fuzz/` | Six new targets + seeds |
| `crates/cli/src/{spec.rs,cmd/transfer.rs}` | New `FORMAT` choices incl. `auto` |
| `apps/desktop/src-tauri/src/contract/import.rs`, `contract/events.rs`, `contract/errors.rs` | Wire types |
| `apps/desktop/src-tauri/src/{commands/import.rs,adapters/import.rs,dialogs.rs,state.rs,lib.rs}` | Commands, Downloads dialog, tokens, drop handler |
| `apps/desktop/src-tauri/src/export_cleanup.rs` (new) | Overwrite-once delete, synced-folder detection |
| `apps/desktop/src/screens/import/` | `sources.ts`, `SourceGrid.tsx`, `SourceSteps.tsx`, `NotComing.tsx`, `CleanupStep.tsx`, `ImportFlow.tsx` |
| `packages/i18n/src/catalog/{en,de}/importGuide.ts` (new) | Source names, guides, lost items, cleanup, checklists |

---

### Task 1: Shared bounded ZIP reader

**Files:**
- Create: `crates/import/src/archive.rs`
- Modify: `crates/import/src/onepux.rs` (move `read_member` and the header scan out; call `archive::*`), `crates/import/src/lib.rs` (`mod archive;`)

**Interfaces:**
- Produces:
  - `pub(crate) const MAX_RATIO: u64 = 1_000;`
  - `pub(crate) const RATIO_FLOOR: usize = 1024 * 1024;` (the ratio applies only above 1 MiB of output)
  - `pub(crate) fn open(input: &[u8], max_members: usize) -> Result<ZipArchive<Cursor<&[u8]>>>`: counts `PK\x01\x02` before `ZipArchive::new` (existing 1PUX logic), `LimitExceeded` above `max_members`, `Malformed` if not a zip.
  - `pub(crate) fn read_member(zip: &mut ZipArchive<Cursor<&[u8]>>, index: usize, limit: usize) -> Result<Buf>`: existing 1PUX body (error mapping, stored/deflate only, read `limit + 1`), plus: after reading, `LimitExceeded` if `out.len() > RATIO_FLOOR` and `out.len() as u64 > compressed_size.saturating_mul(MAX_RATIO)` (capture `compressed_size()` before `take`).
  - `pub(crate) fn find(zip: &ZipArchive<Cursor<&[u8]>>, pred: impl Fn(&str) -> bool) -> Option<usize>`: lowest index whose name is UTF-8 and matches; non-UTF-8 names are skipped.

- [ ] **Step 1: Write the failing unit tests in `archive.rs`**

```rust
#[test] fn open_refuses_more_members_than_the_cap()       // 3 members, open(&z, 2) == Err(LimitExceeded); open(&z, 3).is_ok()
#[test] fn read_refuses_a_member_over_its_limit()          // 10-byte member: read_member(.., 9) == Err(TooLarge); (.., 10) is Ok
#[test] fn read_refuses_deflate_ratio_over_1000_above_1_mib() // 2 MiB zeros deflated: Err(LimitExceeded); same stored: Ok, len 2 MiB
#[test] fn find_skips_non_utf8_names_and_takes_the_lowest_index() // ["a/x.csv", b"\xff.csv", "b/x.csv"]: find(ends_with("x.csv")) == Some(0)
```

Build archives with `zip::ZipWriter` as `onepux.rs` tests do.

- [ ] **Step 2: Run, expect compile failure** — `cargo test -p mosaic-import --lib archive::`
- [ ] **Step 3: Implement the three functions; make `onepux::parse` call `archive::open(input, MAX_ARCHIVE_MEMBERS)` and `archive::read_member`**
- [ ] **Step 4: Run** `cargo test -p mosaic-import --lib archive::` and `cargo test -p mosaic-import --test it onepassword` — expect PASS (1PUX behaviour unchanged).
- [ ] **Step 5: Commit** `refactor(import): shared bounded zip reader with a ratio limit`

---

### Task 2: Imported login URLs must be web origins

Spec §7: a URL that is not a valid origin is kept as a field and never used for matching.

**Files:**
- Modify: `crates/import/src/draft.rs` (`Builder::url`, `EntryWarning`), `crates/import/src/preview.rs` (`summary_warning`)

**Interfaces:**
- Produces: `EntryWarning::UrlKeptAsText` (doc: "A URL that is not a web address or an Android app link was kept as a text field; it is not used for filling."), rolled up to `ImportWarning::UnmappedFields`.
- Rule in `Builder::url` for `ItemType::Login`: value trimmed; a scheme-only value (`^[A-Za-z][A-Za-z0-9+.-]*://$`, e.g. `http://`) is ignored; kept in `urls` (original string, not normalised) if `mosaic_origin::saved_origin(v).is_some()` or `mosaic_origin::app::AndroidAppUrl::parse(v).is_ok()`; otherwise `self.custom(label, Kind::Text, v)` + `warn(UrlKeptAsText)`. Non-login types unchanged.

- [ ] **Step 1: Write the failing unit test in `draft.rs` tests**

```rust
#[test]
fn login_urls_must_be_web_origins_or_android_apps() {
    let mut b = Builder::new(ItemType::Login);
    for u in ["https://a.example/login", "example.org", "androidapp://com.example.app",
              "http://", "ftp://files.example", "my bank"] { b.url(u, "URL"); }
    let (d, w) = b.finish().unwrap();
    // d.urls as strings:
    assert_eq!(url_strs(&d), ["https://a.example/login", "example.org", "androidapp://com.example.app"]);
    // custom field labels; both values are CustomFieldValue::Text ("ftp://files.example", "my bank"):
    assert_eq!(label_strs(&d), ["URL", "URL"]);
    assert!(w.contains(&EntryWarning::UrlKeptAsText));
}
```
And in `preview.rs` tests: `assert_eq!(summary_warning(EntryWarning::UrlKeptAsText), Some(ImportWarning::UnmappedFields));`

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --lib login_urls_must_be`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run the whole crate once** — `cargo test -p mosaic-import` — PASS. Existing assertions on non-origin URLs in fixtures (for example `iosapp://` in Bitwarden) are updated to the new rule, not the rule bent to them.
- [ ] **Step 5: Commit** `feat(import): keep non-origin login URLs as text fields`

---

### Task 3: CSV plumbing, RoboForm and KeePassXC presets

**Files:**
- Modify: `crates/import/src/csv.rs`
- Create: `crates/import/tests/it/support.rs` (shared helpers), `crates/import/tests/it/csv_presets.rs`; Modify `crates/import/tests/it/main.rs` (`mod support; mod csv_presets;`)

**Interfaces:**
- Produces (used by Tasks 4 to 8):
  - `pub(crate) fn rows(input: &[u8]) -> Result<Rows<'_>>`: size check, UTF-8 (`Malformed` otherwise), BOM strip, delimiter detection; `Rows::next_row` becomes `pub(crate)`.
  - `pub(crate) fn normalized(h: &str) -> String` (existing, made crate-visible).
  - `pub(crate) struct Named<'a>` with `pub(crate) fn new(labels: &'a [Str], names: &'a [String], row: &'a [Str]) -> Self`, `pub(crate) fn get(&self, name: &str) -> &'a str` (by normalized name; `""` if absent or the row is short), `pub(crate) fn others(&self, known: &[&str]) -> impl Iterator<Item = (&'a str, &'a str)>` (original label and value of every non-blank column whose normalized name is not in `known`).
  - `fn entry(row: &[Str], mapping: &CsvMapping, preset: Option<CsvPreset>) -> Builder` (preset added; folder values go through `folder_tag`).
  - `fn folder_tag(preset: Option<CsvPreset>, v: &str) -> Option<&str>`: RoboForm: strip leading `/`; KeePassXc: the text after the first `/` (`None` if there is no `/`: the root group); others: `Some(v)`.
  - `CsvPreset::RoboForm`, `CsvPreset::KeePassXc`.
- tests/it/support.rs: `pub fn draft(&Import, u32) -> &ItemDraft`, `labels`, `tags`, `urls`, `secret`, `kinds(&Import) -> Vec<ItemType>`, `zip_of(members: &[(&str, &[u8])]) -> Vec<u8>` (move the private helpers from `formats.rs`, which then uses `support::*`).

Header tables (put each behind `// unverified against a current export (launch gate)`):

| Preset | Detected when the header contains all of (normalized) | Column overrides |
|---|---|---|
| RoboForm | `name, url, matchurl, login, pwd` | `matchurl`→Url, `pwd`→Password, `rffieldsv2`→`Hidden("RoboForm fields")` |
| KeePassXc | `group, title, username, password, url, notes` | `icon`, `last modified`, `created`→Ignore (`totp`→Otp, `group`→Folder already) |

`detect_preset` order becomes: Bitwarden, Firefox, (LastPass, NordPass from Tasks 4 and 5), RoboForm, Dashlane (Task 6), KeePassXc, Safari, Chrome.

- [ ] **Step 1: Write the failing tests in `csv_presets.rs`**

```rust
#[test] fn roboform_logins() {
  // Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2
  // GitHub,https://github.com/login,https://github.com/,alice,pw1,n1,/Work/Dev,"x,y"
  // CsvMapping::detect(header).1 == Some(CsvPreset::RoboForm)
  // urls == ["https://github.com/login","https://github.com/"]; Username "alice"; Password "pw1"; Notes "n1"
  // tags == ["Work/Dev"]; labels == ["RoboForm fields"] and that field is CustomFieldValue::Hidden
}
#[test] fn keepassxc_csv() {
  // "Group","Title","Username","Password","URL","Notes","TOTP","Icon","Last Modified","Created"
  // row 1 Group "Root/Banking", TOTP otpauth://totp/Bank:dave?secret=GEZDGNBVGY3TQOJQ → tags ["Banking"], otp.is_some(), custom_fields empty
  // row 2 Group "Root" → tags empty
}
#[test] fn header_drift_keeps_unknown_columns() {
  // RoboForm header + ",Rf_Extra" column, value "kept" → preset still RoboForm; labels contain "Rf_Extra"
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it csv_presets`
- [ ] **Step 3: Implement** (`csv::parse` uses `rows()`; behaviour of existing presets unchanged)
- [ ] **Step 4: Run** `cargo test -p mosaic-import --test it csv` and `cargo test -p mosaic-import --lib csv::` — PASS
- [ ] **Step 5: Commit** `feat(import): RoboForm and KeePassXC CSV presets`

---

### Task 4: LastPass CSV

**Files:**
- Create: `crates/import/src/lastpass.rs` (private module), `crates/import/tests/it/lastpass.rs`
- Modify: `crates/import/src/csv.rs` (preset + dispatch), `lib.rs` (`mod lastpass;`), `tests/it/main.rs`

**Interfaces:**
- Consumes: `csv::Named`, `csv::entry` (Task 3), `Builder::{text, secret, notes, expiry, content, fallback}`.
- Produces: `CsvPreset::LastPass`, detected when the header contains `url, username, password, extra, name, grouping` (generic column mapping already fits: `totp`→Otp, `extra`→Notes, `grouping`→Folder, `fav`→Favorite). `csv::parse` dispatches rows to `pub(crate) fn entry(n: &Named<'_>, row: &[Str], mapping: &CsvMapping) -> Builder`.
- Header: `url,username,password,totp,extra,name,grouping,fav` `// unverified against a current export (launch gate)`.

Row rules:
- `url` trimmed equal to `http://sn` → a note: `extra` starting with `NoteType:` is a structured note, otherwise a `SecureNote` with `extra` as notes and `name` as title. Not `http://sn` → `csv::entry(row, mapping, Some(LastPass))`.
- Structured note parsing (`fn structured(extra: &str) -> Option<Structured<'_>>`, `struct Structured<'a> { note_type: &'a str, fields: Vec<(&'a str, Str)>, notes: Option<&'a str> }`): first line `NoteType:<type>`; each next line `key:value` splits at the first `:`; a line without `:` continues the previous value (joined with `\n`); the `Notes:` line takes the rest of the text, following lines included. A value of only `,` and whitespace is blank. `Language` is ignored.
- Mapping table:

| NoteType | Item | Keys → fields (others → `content`/`fallback`) |
|---|---|---|
| `Credit Card` | CreditCard | `Name on Card`→CardholderName, `Type`→CardBrand, `Number`→CardNumber, `Security Code`→CardVerificationCode, `Expiration Date` (`<English month name>,<YYYY>`)→`expiry("MM/YYYY")` |
| `Address` | Identity | `First Name`, `Middle Name`, `Last Name`, `Company`, `Address 1`→AddressLine1, `Address 2`→AddressLine2, `City / Town`→City, `State`→Region, `Zip / Postal Code`→PostalCode, `Country`, `Email Address`→Email, `Phone` (JSON `{"num":…}` → `num`, else as is)→Phone, `Birthday`→BirthDate if ISO |
| `Wi-Fi Password` | Wifi | `SSID`→WifiSsid, `Password`→WifiPassword, `Connection Type`→WifiSecurity |
| any other | SecureNote | every key as a custom field: `Kind::Hidden` if the key contains (case-insensitive) `password`, `number`, `pin`, `code`, `key` or `secret`, else `Kind::Text` |

- [ ] **Step 1: Write the failing tests in `tests/it/lastpass.rs`** (one CSV with seven rows; `parse(Format::Csv, ..)`)

```rust
#[test] fn lastpass_export() {
  // 0 login GitHub: otp.is_some(), Notes "my note", tags ["Work\\Dev"], favorite
  // 1 http://sn plain: ItemType::SecureNote, Notes "just text", urls empty
  // 2 Credit Card: CreditCard; CardholderName "Alice Doe"; CardBrand "Visa"; secret CardNumber "4111111111111111";
  //   secret CardVerificationCode "123"; CardExpiry "01/2030"; Notes "line1\nline2"; labels empty ("Start Date:," is blank)
  // 3 Address: Identity; FirstName "Alice"; Phone "123" from {"num":"123","ext":"","cc3l":"DEU"}; labels ["Gender"]; warning UnmappedFields
  // 4 Wi-Fi Password: Wifi; WifiSsid "home"; secret WifiPassword "wifipw"; WifiSecurity "WPA2"
  // 5 NoteType:Passport: SecureNote; labels ["Number","Name"]; "Number" is Hidden, "Name" is Text
  // 6 login with url "http://": urls empty, no UrlKeptAsText
}
#[test] fn lastpass_header_with_an_extra_column_still_detects() // + ",last_touch" → preset LastPass, label "last_touch" kept
```
Plus a unit test in `lastpass.rs`: `structured_continuation_lines_and_notes` (`"NoteType:X\nA:1\n2\nNotes:n\nm"` → fields `[("A","1\n2")]`, notes `Some("n\nm")`) and `structured("plain")` is `None`.

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import lastpass`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** `cargo test -p mosaic-import lastpass` — PASS
- [ ] **Step 5: Commit** `feat(import): LastPass CSV with secure notes and structured note types`

---

### Task 5: NordPass CSV

**Files:**
- Create: `crates/import/src/nordpass.rs` (private), `crates/import/tests/it/nordpass.rs`
- Modify: `csv.rs` (preset + dispatch), `draft.rs` (`full_name`), `lib.rs`, `tests/it/main.rs`

**Interfaces:**
- Produces:
  - `CsvPreset::NordPass`, detected when the header contains `name, url, username, password, note, folder, type, cardnumber`.
  - `pub(crate) fn entry(n: &Named<'_>) -> Option<Builder>` (`None` for a folder row: not an entry, not counted as skipped).
  - `Builder::full_name(&mut self, value: &str)` in `draft.rs` (used again by Tasks 6 and 9): with two or more whitespace-separated words, FirstName = all but the last word, LastName = the last; one word → FirstName.
- Header: `name,url,additional_urls,username,password,note,cardholdername,cardnumber,cvc,pin,expirydate,zipcode,folder,full_name,phone_number,email,address1,address2,city,country,state,type,custom_fields` `// unverified against a current export (launch gate)`.

`type` → item: `password`→Login, `note`→SecureNote, `credit_card`→CreditCard, `identity`→Identity, `folder`→`None`, anything else → SecureNote + `UnknownCategory`. Every type: `name`→Title, `note`→Notes, `folder`→tag. Login: `url` + each string of `additional_urls` (a JSON array; otherwise one URL), `username`, `password`. Card: `cardholdername`, `cardnumber`, `cvc`, `pin`→CardPin, `expirydate`→`expiry`, `zipcode`→fallback. Identity: `full_name`→`full_name()`, `email`, `phone_number`, `address1`, `address2`, `city`, `state`→Region, `zipcode`→PostalCode, `country`. `custom_fields`: JSON array of `{label, type, value}` (`hidden`→Hidden, `date`→Date, else Text); a value that is not such an array → `fallback("Custom fields", Kind::Text, raw)`. Every other non-blank column (`Named::others`) → `content`.

- [ ] **Step 1: Write the failing test**

```rust
#[test] fn nordpass_export() {
  // rows: password, note, credit_card, identity, folder, "passkey"
  // import.entries.len() == 5 and import.skipped.is_empty()  (the folder row is neither)
  // login: urls ["https://a.example","https://b.example"] from additional_urls ["https://b.example"];
  //        custom field "PIN" is Hidden from [{"label":"PIN","type":"hidden","value":"1234"}]
  // card: secret CardNumber, CVV "123", CardPin "0000", CardExpiry "03/2029" from "03/29"
  // identity: FirstName "Alice", LastName "Doe" from "Alice Doe"; Email; Phone
  // "passkey" row: SecureNote with UnknownCategory
}
#[test] fn nordpass_bad_custom_fields_are_kept_as_text() // custom_fields "not json" → label "Custom fields", UnmappedFields
```
Unit test in `draft.rs`: `full_name_splits_at_the_last_word` (`"Maria de la Cruz"` → First `"Maria de la"`, Last `"Cruz"`; `"Cher"` → First only).

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import nordpass` and `--lib full_name`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run both** — PASS
- [ ] **Step 5: Commit** `feat(import): NordPass CSV with item types and custom fields`

---

### Task 6: Dashlane ZIP

**Files:**
- Create: `crates/import/src/dashlane.rs` (`pub mod`), `crates/import/tests/it/dashlane.rs`
- Modify: `csv.rs` (`CsvPreset::Dashlane`), `lib.rs` (`Format::Dashlane`, dispatch), `tests/it/main.rs`

**Interfaces:**
- Consumes: `archive::{open, find, read_member}` (Task 1), `csv::{rows, Named, normalized}` (Task 3), `Builder::full_name` (Task 5).
- Produces: `Format::Dashlane` (doc: "Dashlane CSV export: a zip of CSV files"), `pub fn parse(input: &[u8]) -> Result<Import>`, `pub const MAX_MEMBERS: usize = 64;`, `pub const MAX_MEMBER_LEN: usize = 64 * 1024 * 1024;`; total inflated bytes across members ≤ `crate::MAX_FILE_LEN` (pass `MAX_MEMBER_LEN.min(left)` as the limit). `CsvPreset::Dashlane` (detected when the header contains `username, username2, username3, title, password, note, url, category`), so an extracted `credentials.csv` also imports through `Format::Csv`.

Members are matched by lowercase file name (any folder), in this order; all tables `// unverified against a current export (launch gate)`:

| Member | Header | Item and mapping |
|---|---|---|
| `credentials.csv` | `username,username2,username3,title,password,note,url,category,otpSecret` (also `otpUrl`) | Login via `csv::entry(.., Some(Dashlane))`; overrides `username2`→`Text("Username 2")`, `username3`→`Text("Username 3")`, `category`→Folder, `otpsecret`/`otpurl`→Otp |
| `securenotes.csv` | `title,note` (+ `category`) | SecureNote; `category`→tag |
| `payments.csv` | `type,account_name,account_holder,cc_number,code,expiration_month,expiration_year,routing_number,account_number,country,issuing_bank` | `payment_card`→CreditCard (`account_name` Title, `account_holder` CardholderName, `cc_number` CardNumber, `code` CVV, expiry `MM/YYYY` from month/year); `bank`→SecureNote with `account_number`/`routing_number` Hidden, others Text |
| `ids.csv` | `type,number,name,issue_date,expiration_date,place_of_issue,state` | Identity, Title = `name` or `type`; `passport`→PassportNumber, `driver_license`→LicenseNumber, other types→NationalId; `name`→`full_name`; dates→`Kind::Date` |
| `personalinfo.csv` | `type,title,first_name,middle_name,last_name,login,date_of_birth,place_of_birth,email,email_type,item_name,phone_number,address,country,state,city,zip,…` | Identity, Title = `item_name` or `type`; first/middle/last, `date_of_birth`→BirthDate if ISO, `email`, `phone_number`, `address`→AddressLine1, `city`, `state`→Region, `zip`→PostalCode, `country` |
| any other `*.csv` | any | SecureNote titled by the first non-blank value; every column a custom field; `UnknownCategory` |

Non-CSV members are not read. No CSV member → `Error::Malformed`; an encrypted `.dash` backup is not a zip and is `Error::Malformed` as well (the guide asks for the CSV export).

- [ ] **Step 1: Write the failing tests** (archives from `support::zip_of`)

```rust
#[test] fn dashlane_export() {
  // members: credentials.csv (two usernames), securenotes.csv, payments.csv (card + bank), ids.csv (passport),
  //          personalInfo.csv, notes/other.csv
  // kinds == [Login, SecureNote, CreditCard, SecureNote, Identity, Identity, SecureNote]
  // login labels ["Username 2"]; card CardExpiry "07/2031" from 7 and 2031; bank field "Account number" Hidden;
  // passport secret PassportNumber "X1234567"; other.csv entry has UnknownCategory
}
#[test] fn dashlane_limits_and_errors() {
  // 65 members → Err(LimitExceeded); b"not a zip" → Err(Malformed); zip with only readme.txt → Err(Malformed)
}
#[test] fn extracted_credentials_csv_imports_as_csv() // parse(Format::Csv, credentials) → 1 Login; detect preset Dashlane
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it dashlane`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** — PASS
- [ ] **Step 5: Commit** `feat(import): Dashlane CSV zip export`

---

### Task 7: Keeper JSON

**Files:**
- Create: `crates/import/src/keeper.rs` (`pub mod`), `crates/import/tests/it/keeper.rs`
- Modify: `lib.rs` (`Format::Keeper`, dispatch), `tests/it/main.rs`

**Interfaces:**
- Consumes: `json::parse`, `Json::{get, str, array, members, as_str, is_object}`.
- Produces: `Format::Keeper` (doc: "Keeper JSON export"), `pub fn parse_json(input: &[u8]) -> Result<Import>`.
- Shape `// unverified against a current export (launch gate)`: `{"shared_folders":[…], "records":[{ "title", "login", "password", "login_url", "notes", "$type", "custom_fields": { key: value }, "folders": [{"folder"} | {"shared_folder", "folder"}] }]}`. `records` missing or not an array → `Malformed`; a non-object record → `skip(Malformed)`.

Mapping:
- `$type`: absent, `login`, `general`, `databaseCredentials`, `serverCredentials` → Login; `bankCard` → CreditCard; `encryptedNotes` → SecureNote; `contact`, `address` → Identity; `wifiCredentials` → Wifi; anything else → SecureNote + `UnknownCategory`.
- Top-level: `title`, `login`→Username, `password`, `login_url`→url, `notes`.
- `custom_fields` keys (a value may be a string, an object, or an array of either; arrays and objects are walked one level each; values nested deeper are not read and the entry gets `FieldsDropped`):
  - `$oneTimeCode` → `otp_text`; `$paymentCard` `{cardNumber, cardExpirationDate, cardSecurityCode}` → card fields + `expiry`; `$pinCode` → CardPin on cards, else Hidden; `$name` `{first, middle, last}` → name fields; `$email` → Email; `$phone` `{number}` → Phone; `$address` `{street1, street2, city, state, zip, country}` → address fields; `$keyPair`/`$secret`/`$password` → Hidden.
  - `$<kind>:<label>` → custom field labelled `<label>` (Hidden for `secret`, `password`, `pinCode`; else Text); a plain key → Text labelled by the key; the same Hidden-by-name rule as LastPass for keys without a kind.
- `folders`: `{folder}` → tag `folder`; `{shared_folder, folder}` → tag `"{shared_folder}/{folder}"`; `{shared_folder}` → tag `shared_folder`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn keeper_json_export() {
  // login: otp.is_some() from $oneTimeCode; label "Recovery" from "$text:Recovery"; tags ["Personal","Team/Ops"]
  // bankCard: CreditCard; CardExpiry "04/2030"; secret CardVerificationCode "123"; secret CardPin "0000"
  // contact: Identity; FirstName "Alice"; LastName "Doe"; Email "alice@example.com"; Phone "123" from [{"number":"123"}]; City
  // encryptedNotes: SecureNote with Notes
  // softwareLicense: SecureNote + UnknownCategory; field "$licenseNumber" kept (Hidden by name)
}
#[test] fn keeper_json_errors() // {"records":5} → Malformed; b"[" → Malformed; [1] in records → one SkippedEntry Malformed
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it keeper_json`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** — PASS
- [ ] **Step 5: Commit** `feat(import): Keeper JSON export`

---

### Task 8: Keeper CSV (no header row)

**Files:**
- Modify: `crates/import/src/keeper.rs`, `lib.rs` (`Format::KeeperCsv`, dispatch), `crates/import/tests/it/keeper.rs`

**Interfaces:**
- Consumes: `csv::rows` (Task 3).
- Produces: `Format::KeeperCsv` (doc: "Keeper CSV export (no header row; never detected from content, chosen by the user's source)"), `pub fn parse_csv(input: &[u8]) -> Result<Import>`.
- Columns `// unverified against a current export (launch gate)`: 0 folder, 1 title, 2 login, 3 password, 4 website address, 5 notes, 6 shared folder, then name/value pairs. A pair value starting with `otpauth://` (any name, typically `TFC:Keeper`) → `otp_text`; other pairs → Text custom field (Hidden-by-name rule); a trailing name without value → ignored if blank, else `fallback("Field", Kind::Text, name)`. Folder and shared folder → one tag each. Every record is an entry (no header is skipped).

- [ ] **Step 1: Write the failing test**

```rust
#[test] fn keeper_csv_export() {
  // "Personal,GitHub,alice,pw1,https://github.com,n,,TFC:Keeper,otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ,Recovery,abc"
  // ",Team note,,,,body,Team"
  // entries 2; [0] title GitHub, tags ["Personal"], otp.is_some(), labels ["Recovery"]
  // [1] tags ["Team"], Notes "body"
  // parse_csv(b"") == Err(Malformed)
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it keeper_csv`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** — PASS
- [ ] **Step 5: Commit** `feat(import): Keeper CSV export`

---

### Task 9: Proton Pass

**Files:**
- Create: `crates/import/src/proton_pass.rs` (`pub mod`), `crates/import/tests/it/proton_pass.rs`
- Modify: `lib.rs` (`Format::ProtonPass`, dispatch), `tests/it/main.rs`

**Interfaces:**
- Consumes: `archive::*` (Task 1), `json::parse`, `Builder::full_name` (Task 5).
- Produces: `Format::ProtonPass` (doc: "Proton Pass export: a zip with `Proton Pass/data.json`, or that `data.json` itself; unencrypted only"), `pub fn parse(input: &[u8]) -> Result<Import>`, `pub const MAX_DATA_JSON_LEN: usize = 64 * 1024 * 1024;`.
- Input: starts with `-----BEGIN PGP MESSAGE-----` → `Encrypted`; starts with `PK\x03\x04` → zip opened with `archive::open(input, onepux::MAX_ARCHIVE_MEMBERS)`: a member ending `data.pgp` and no `data.json` → `Encrypted`; the member `Proton Pass/data.json` (else the first member whose name ends `/data.json` or equals `data.json`) → JSON; none → `Malformed`. Otherwise the input is the JSON itself.
- JSON `// unverified against a current export (launch gate)`: `{"version", "userId", "encrypted": bool, "vaults": { id: { "name", "items": [ { "data": { "metadata": {"name","note"}, "type", "content": {…}, "extraFields": [{"fieldName","type","data":{"content"}}] }, "state", "pinned", "aliasEmail" } ] } } }`. `encrypted: true` → `Encrypted`.

Mapping: `metadata.name`→Title, `metadata.note`→Notes, `pinned`→favorite, `state == 2`→trashed, vault `name`→tag when there is more than one vault. `type`: `login` (`itemUsername`→Username, else `itemEmail`→Username and `itemEmail` as Email field if both set; `password`; each of `urls`; `totpUri`→otp; non-empty `passkeys`→`PasskeysNotImported`), `note`→SecureNote, `creditCard` (`cardholderName`, `cardType`→CardBrand, `number`, `verificationNumber`→CVV, `expirationDate` `YYYY-MM`→expiry, `pin`→CardPin), `identity` (`fullName`→`full_name`, or `firstName`/`middleName`/`lastName`; `email`, `phoneNumber`, `organization`→Company, `streetAddress`→AddressLine1, `city`, `stateOrProvince`→Region, `zipOrPostalCode`→PostalCode, `countryOrRegion`→Country, `birthdate`→BirthDate if ISO; other string members → `content`), `alias`→Login with Username = `aliasEmail`; other types → SecureNote + `UnknownCategory`, string members of `content` as `content`. `extraFields`: `text`→Text, `hidden`→Hidden, `totp`→`custom_otp`. An item with a non-empty `files` or `attachments` array → `AttachmentMissing`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn proton_pass_zip_export() {
  // vaults Personal (login pinned with 2 urls, totpUri, extraFields hidden+totp; trashed login state 2; alias;
  //                  login with passkeys [{}]) and Work (creditCard expirationDate "2030-05"; identity fullName "Alice Doe")
  // tags of every entry == [vault name]; favorite; trashed; alias Username "a.b@alias.example";
  // passkey login has PasskeysNotImported; CardExpiry "05/2030"; FirstName "Alice"
}
#[test] fn proton_pass_raw_data_json_matches_the_zip()  // same JSON unzipped → same entries.len()
#[test] fn proton_pass_encrypted_and_malformed() {
  // {"encrypted":true,"vaults":{}} → Encrypted; zip with only "Proton Pass/data.pgp" → Encrypted;
  // b"-----BEGIN PGP MESSAGE-----\n..." → Encrypted; zip with readme.txt only → Malformed
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it proton_pass`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** — PASS
- [ ] **Step 5: Commit** `feat(import): Proton Pass unencrypted export`

---

### Task 10: Content-based detection

**Files:**
- Create: `crates/import/src/detect.rs`, `crates/import/tests/it/detect.rs`
- Modify: `lib.rs` (`mod detect; pub use detect::{Detected, Source, detect};`, crate doc lists the new formats), `tests/it/main.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum Source { Chromium, Firefox, Safari, BitwardenCsv, Bitwarden, Enpass, KeePass, KeePassXcCsv,
                  OnePassword, Cxf, LastPass, Dashlane, Keeper, NordPass, ProtonPass, RoboForm, Csv }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Detected { pub format: Format, pub source: Source }
/// What the file is, from its content only (never its name). `None`: not an export Mosaic reads.
#[must_use] pub fn detect(input: &[u8]) -> Option<Detected>;
```

Rules, first match wins (bounded: zip via `archive::open(input, onepux::MAX_ARCHIVE_MEMBERS)` names only, JSON via `json::parse`, CSV via the header row of `csv::rows` only):
1. Over `MAX_FILE_LEN` or empty → `None`.
2. Starts with KDBX signature `03 D9 A2 9A 67 FB 4B B5` → `Keepass`/`KeePass`.
3. Starts with `-----BEGIN PGP MESSAGE-----` → `ProtonPass`/`ProtonPass` (parsing then says `Encrypted`).
4. Starts with `PK\x03\x04`: member `export.data` → `OnePassword`; a `data.json` or `data.pgp` member as in Task 9 → `ProtonPass`; a member ending `credentials.csv` (lowercase) → `Dashlane`; else `None`.
5. First non-whitespace byte after an optional UTF-8 BOM is `{`: object with key `encrypted` or `passwordProtected` and an `items` array → `Bitwarden`; `records` array → `Keeper`; `vaults` object → `ProtonPass`; `accounts` array → `Cxf`; `items` array → `Enpass`; else `None`.
6. Valid UTF-8 with a header row of at least two columns and no NUL byte → `Csv` with source by preset: Chrome→Chromium, Firefox, Safari, Bitwarden→BitwardenCsv, LastPass, NordPass, RoboForm, KeePassXc→KeePassXcCsv, Dashlane; no preset → `Source::Csv`.
7. Else `None`.

- [ ] **Step 1: Write the failing table test**

```rust
#[test] fn detects_every_format_from_content() {
  // (bytes, Some((Format, Source))) for: the KDBX fixture, a 1PUX zip, a Proton zip, a Dashlane zip, PGP armor,
  // bitwarden.json fixture, {"passwordProtected":true,"items":[]}, a Keeper JSON, {"vaults":{}}, a CXF JSON,
  // enpass.json fixture, each CSV preset header, a German generic CSV (Source::Csv),
  // a headerless Keeper CSV row (Source::Csv, Format::Csv)
}
#[test] fn unknown_files_are_not_detected()      // b"", b"hello\nworld", random 64 bytes with NUL, zip of readme.txt, {"x":1}, b"[1]" → None
#[test] fn utf16_and_latin1_are_not_detected() {
  // b"\xff\xfeN\0a\0m\0e\0" → None; b"name,url\nM\xfcller,x" → None;
  // parse(Format::Csv, same latin1 bytes) == Err(Error::Malformed)
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-import --test it detect`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** `cargo test -p mosaic-import` (whole crate once: presets, formats and detection together) — PASS
- [ ] **Step 5: Commit** `feat(import): detect the export format from content`

---

### Task 11: Fuzz targets and fixture notes

One target per new parser entry point (the four CSV presets share one target that picks the header from the first byte).

**Files:**
- Create: `crates/import/fuzz/fuzz_targets/{csv_presets,dashlane,keeper_json,keeper_csv,proton_pass,detect}.rs`, seeds under `crates/import/fuzz/seeds/<target>/` (synthetic, from the Task 3 to 10 tests)
- Modify: `crates/import/fuzz/Cargo.toml` (six `[[bin]]` entries, same shape as `csv`), `crates/import/tests/fixtures/README.md` (a row per new synthetic export, marked "from public format descriptions, unverified")

**Interfaces:**
- `csv_presets`: `HEADERS: [&[u8]; 5]` = LastPass, NordPass, RoboForm, KeePassXC, Dashlane header lines (with `\n`); `data.split_first()` → `HEADERS[usize::from(b) % 5]` + rest → `common::check(mosaic_import::csv::parse(&input, None))`.
- `dashlane`, `keeper_json`, `keeper_csv`, `proton_pass`: `common::check(mosaic_import::parse(Format::<X>, data, None))`.
- `detect`: `if let Some(d) = mosaic_import::detect(data) { common::check(mosaic_import::parse(d.format, data, None)); }` (KDBX then ends in `PasswordRequired`, a documented error).
- CI needs no change: `cargo fuzz list` picks the new bins up.

- [ ] **Step 1: Write the six targets and seeds**
- [ ] **Step 2: Check they build** — `RUSTFLAGS="--cfg fuzzing" CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target/fuzz-check CARGO_BUILD_JOBS=2 cargo check --bins --manifest-path crates/import/fuzz/Cargo.toml` — expect success (stable toolchain; separate target dir so the main cache is not invalidated)
- [ ] **Step 3: Commit** `test(import): fuzz targets for the new importers and detection`

---

### Task 12: CLI formats

**Files:**
- Modify: `crates/cli/src/spec.rs` (`FORMAT` choices), `crates/cli/src/cmd/transfer.rs` (`parse_file`), `crates/cli/man/mosaic.1` and `crates/cli/completions/*` (regenerated), `crates/cli/tests/it/transfer.rs`

**Interfaces:**
- Consumes: `mosaic_import::{detect, Format}`.
- New choices, in this order after `csv`: `dashlane`, `keeper`, `keeper-csv`, `protonpass`, `auto`. `auto` → `detect(&bytes)` (read once with the existing `read_file`); `None` → the existing `import_failed()` error. LastPass, NordPass, RoboForm and KeePassXC CSV need no choice (`csv` detects them). The KDBX password check runs on the resolved format, so `auto` on a `.kdbx` still requires `--file-password-prompt`.

- [ ] **Step 1: Write the failing test in `tests/it/transfer.rs`**

```rust
#[test] fn import_auto_detects_a_lastpass_csv() {
  // write a 2-row LastPass CSV; `mosaic --json import auto <file> --dry-run` succeeds; JSON "count" == 2
  // `mosaic --json import auto <file with b"hello">` fails with the import-failed error
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-cli --features test-util --test it import_auto -- --test-threads=2`
- [ ] **Step 3: Implement; regenerate docs** — `UPDATE_DOCS=1 cargo test -p mosaic-cli --features test-util docs_are_current`
- [ ] **Step 4: Run** the Step 2 command and `cargo test -p mosaic-cli --features test-util docs_are_current` — PASS
- [ ] **Step 5: Commit** `feat(cli): import Dashlane, Keeper, Proton Pass and auto-detected files`

---

### Task 13: Desktop IPC: detection, Downloads dialog, new formats

**Files:**
- Modify (Rust): `apps/desktop/src-tauri/src/contract/import.rs`, `contract/fixtures.rs`, `commands/import.rs`, `commands/mod.rs`, `commands/names.rs`, `adapters/import.rs`, `dialogs.rs`, `capabilities/main.json`, `handler_tests.rs`
- Modify (TS): `apps/desktop/src/ipc/contract/import.ts`, `ipc/contract/commands.ts`, `ipc/mock/import.ts`, `ipc/contract.fixtures.ts` (regenerated), `screens/import/ImportFlow.tsx` (call sites only)
- Modify: `docs/DESKTOP_IPC.md` (Import rows and the Import paragraph)

**Interfaces:**
- Wire (`wire_enum!`, camelCase):
  - `ImportFormat` += `Dashlane`, `Keeper`, `KeeperCsv`, `ProtonPass`.
  - `ImportEntryWarning` += `UrlKeptAsText` (mapped from `EntryWarning::UrlKeptAsText`).
  - new `ImportDetectedSource` { `Chromium, Firefox, Safari, BitwardenCsv, Bitwarden, Enpass, Keepass, KeepassxcCsv, OnePassword, Cxf, Lastpass, Dashlane, Keeper, Nordpass, ProtonPass, Roboform, Csv` } mapped 1:1 from `mosaic_import::Source`.
- Commands:
  - `import_choose_file`: request `null` (the `format` field and `ImportChooseRequest` go away) → unchanged response. Filter `FileFilter { name: "Export files", extensions: &["csv", "json", "zip", "1pux", "kdbx"] }` (a constant in `commands/import.rs`; `filter(format)` is removed).
  - `import_detect` `{ fileToken }` → `ImportDetected { source: ImportDetectedSource | null, format: ImportFormat | null }`; errors `importFileUnreadable`, `importTooLarge`. `ImportService::detect(&self, file: &Path) -> IpcResult<ImportDetected>` reads with the existing `read_export` (zeroizing) and calls `mosaic_import::detect`.
- Dialog: `Dialogs::open_import_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>>`, default body `self.open_file(filter)`. `NativeDialogs` overrides it with `set_directory(dir)` when `start_dir(self.app.path().download_dir().ok())` is `Some(dir)`. `fn start_dir(candidate: Option<PathBuf>) -> Option<PathBuf>`: `Some` only for an existing directory.

- [ ] **Step 1: Write the failing Rust tests**

```rust
// handler_tests.rs
#[test] fn import_detect_reports_source_and_format() {
  // ENPASS fixture → {source: Enpass, format: Enpass}; a LastPass CSV → {Lastpass, Csv};
  // b"hello" → {None, None}; unknown token → ImportFileUnreadable
}
#[test] fn import_choose_file_uses_the_import_dialog() // FixedDialog records "Export files" via open_import_file
// dialogs.rs
#[test] fn start_dir_ignores_a_missing_folder()   // start_dir(Some(tmp.join("nope"))) == None; start_dir(Some(tmp)) == Some(tmp); start_dir(None) == None
```
Existing helpers `chosen(b, format)` / `preview_of` are updated to the new request shapes.

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-desktop import_`
- [ ] **Step 3: Implement Rust; regenerate fixtures** — `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`
- [ ] **Step 4: Update TS contract, commands map and mock** (`import_detect` mock returns `{ source: "enpass", format: "enpass" }` unless `MockOptions.importDetected` is set; add `importDetected?: ImportDetected` to `MockOptions`); `ImportFlow.tsx` calls `import_choose_file` with `null` and, after choosing, `import_detect`, then previews with the detected format (the guided UI comes in Task 17).
- [ ] **Step 5: Run** `cargo test -p mosaic-desktop import_`, `cargo test -p mosaic-desktop contract_fixtures`, `pnpm --filter @mosaic/desktop typecheck`, `pnpm --filter @mosaic/desktop test src/ipc src/screens/import` — PASS
- [ ] **Step 6: Commit** `feat(desktop): detect the import format and open the dialog in Downloads`

---

### Task 14: Desktop: delete the export file, synced-folder note

**Files:**
- Create: `apps/desktop/src-tauri/src/export_cleanup.rs`
- Modify: `state.rs` (`FileTokens`), `adapters/import.rs` (`Held.file_token`), `commands/import.rs`, `commands/{mod,names}.rs`, `capabilities/main.json`, `contract/{import,errors,fixtures}.rs`, `lib.rs` (`mod export_cleanup;`), `handler_tests.rs`; TS `ipc/contract/{import,common,commands}.ts`, `ipc/mock/import.ts`, `contract.fixtures.ts`; `packages/i18n/src/catalog/{en,de}/errors.ts`; `docs/DESKTOP_IPC.md`

**Interfaces:**
- `export_cleanup.rs`:
  - `pub fn overwrite_and_delete(path: &Path) -> std::io::Result<()>`: `symlink_metadata` must be a regular file (else `InvalidInput`); open for writing without truncation; write zeros over the current length in 64 KiB chunks; `sync_all`; close; `remove_file`.
  - `pub fn sync_roots(home: Option<&Path>, var: &dyn Fn(&str) -> Option<OsString>) -> Vec<(ImportSyncedFolder, PathBuf)>`: env `OneDrive`, `OneDriveConsumer`, `OneDriveCommercial` → OneDrive; `<home>/Dropbox` → Dropbox; `<home>/Google Drive`, `<home>/My Drive` → GoogleDrive; `<home>/Box` → Box; `<home>/Library/Mobile Documents` → Icloud; each entry of `<home>/Library/CloudStorage` classified by `cloud_storage_service`.
  - `fn cloud_storage_service(dir_name: &str) -> Option<ImportSyncedFolder>`: prefixes `OneDrive`, `Dropbox`, `GoogleDrive`, `Box`, `iCloud`.
  - `pub fn synced_folder(file: &Path, roots: &[(ImportSyncedFolder, PathBuf)]) -> Option<ImportSyncedFolder>`: `file` (canonicalized when possible) starts with a root; on Windows compared case-insensitively.
- Wire: `ImportSyncedFolder` { `OneDrive, Dropbox, GoogleDrive, Icloud, Box` }; `ImportDetected` += `synced_folder: Option<ImportSyncedFolder>`; `ErrorCode` += `ImportDeleteNotAllowed`, `ImportDeleteFailed`.
- `FileTokens`: `pub fn mark_committed(&mut self, token: &str)`, `pub fn is_committed(&self, token: &str) -> bool`, `pub fn remove(&mut self, token: &str) -> Option<PathBuf>`; `clear()` also clears the committed set.
- `ImportService::hold(&self, import, session, file_token: &str)`; `ImportService::commit(..) -> IpcResult<(ImportResult, String)>` (the file token); `Backend::import_commit` marks the token committed.
- Command `import_delete_file` `{ fileToken }` → `null`. Unlocked required; unknown token → `importFileUnreadable`; token without a committed import → `importDeleteNotAllowed` (an export is only destroyed once its content is in the vault); I/O error → `importDeleteFailed` (token kept for a retry); success removes the token.
- Error strings (EN; DE informal): `error.importDeleteNotAllowed`: "Import the file first. Mosaic only deletes an export after its items are in your vault."; `error.importDeleteFailed`: "The file couldn't be deleted. It may be open in another app or read-only. Delete it yourself, then empty the recycle bin."

- [ ] **Step 1: Write the failing tests**

```rust
// export_cleanup.rs
#[test] fn overwrite_and_delete_removes_the_file()
#[test] fn overwrite_and_delete_refuses_read_only_and_keeps_the_file() // set_readonly(true) → Err, file exists, content unchanged
#[cfg(unix)] #[test] fn overwrite_and_delete_refuses_a_symlink()      // Err(InvalidInput); link target untouched
#[test] fn onedrive_known_folder_move_is_detected() // var("OneDrive") = <tmp>/OneDrive; <tmp>/OneDrive/Downloads/x.csv → Some(OneDrive)
#[test] fn cloud_storage_names_are_classified()     // "OneDrive-Personal"→OneDrive, "GoogleDrive-a@b"→GoogleDrive, "Other"→None
#[test] fn files_outside_sync_roots_are_not_synced()
// handler_tests.rs
#[test] fn import_delete_only_after_commit() {
  // choose + preview: delete → ImportDeleteNotAllowed, file exists; commit; delete → Ok, file gone;
  // delete again → ImportFileUnreadable
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-desktop export_cleanup` and `cargo test -p mosaic-desktop import_delete`
- [ ] **Step 3: Implement; regenerate fixtures** (`MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`); TS contract + mock (`import_delete_file` mock: ok, or `fail("importDeleteFailed")` when `MockOptions.importDeleteFails`); i18n error strings
- [ ] **Step 4: Run** Step 2 commands, `cargo test -p mosaic-desktop contract_fixtures`, `pnpm --filter @mosaic/desktop typecheck`, `pnpm --filter @mosaic/i18n test` — PASS
- [ ] **Step 5: Commit** `feat(desktop): delete the export after import and name synced folders`

---

### Task 15: Desktop: drop a file onto the window

The webview may not listen to `tauri://drag-drop` (paths; refused in `redteam_surface_tests.rs`), so Rust takes the drop and hands the webview a token.

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs` (`on_window_event`), `commands/import.rs` (`Backend::import_file_dropped`), `contract/events.rs`, `contract/fixtures.rs`, `handler_tests.rs`; TS `ipc/contract/events.ts` (or wherever `AppEvent` lives), `app/EventsProvider.tsx` (+ test), `contract.fixtures.ts`; `docs/DESKTOP_IPC.md` (events table)

**Interfaces:**
- `AppEvent::ImportFileDropped { file_token: String, file_name: String }`.
- `pub fn import_file_dropped(&self, paths: &[PathBuf])` on `Backend`: does nothing unless unlocked, `paths.len() == 1` and `symlink_metadata(path)` is a regular file; then inserts the path into `import_files` and emits the event.
- `lib.rs`: `tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. })` → `state.backend().import_file_dropped(paths)`.
- EventsProvider: `importDrop: { fileToken: string; fileName: string } | null` and `clearImportDrop: () => void` in `EventsContextValue`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn a_dropped_file_gets_a_token_while_unlocked() {
  // unlocked, one file → recorded ImportFileDropped; its token resolves in import_files
  // two files → no event; a directory → no event; locked → no event
}
```
TS (`EventsProvider.test.tsx`): `importFileDropped` event sets `importDrop` to `{ fileToken: "file-9", fileName: "x.csv" }`; `clearImportDrop()` resets it to `null`.

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p mosaic-desktop dropped_file` and `pnpm --filter @mosaic/desktop test src/app/EventsProvider`
- [ ] **Step 3: Implement; regenerate fixtures**
- [ ] **Step 4: Run** Step 2 commands, `cargo test -p mosaic-desktop redteam_surface` (the listen refusal still holds) and `pnpm --filter @mosaic/desktop typecheck` — PASS
- [ ] **Step 5: Commit** `feat(desktop): accept a dropped export file through Rust`

---

### Task 16: Source catalog and guide strings

**Files:**
- Create: `apps/desktop/src/screens/import/sources.ts`, `sources.test.ts`, `packages/i18n/src/catalog/{en,de}/importGuide.ts`
- Modify: `packages/i18n/src/{en,de}.ts` (register `importGuide`)

**Interfaces:**

```ts
export type SourceId = "chrome" | "edge" | "brave" | "opera" | "vivaldi" | "arc" | "firefox" | "safari"
  | "applePasswords" | "onePassword" | "bitwarden" | "keepass" | "keepassxcCsv" | "enpass" | "lastpass"
  | "dashlane" | "keeper" | "nordpass" | "protonPass" | "roboform" | "otherFile";
export type LostKey = "passkeys" | "attachments" | "history" | "roboformExtras";
export type ChecklistKey = "browserSaving" | "firefoxSaving" | "safariSaving" | "oldExtension" | "deleteOld";
export interface ImportSourceInfo {
  id: SourceId; group: "browser" | "manager" | "other";
  steps: number;                       // i18n keys import.guide.<id>.1 … .<steps>
  internalUrl: string | null;          // shown with a copy button (navigator.clipboard.writeText, as SshSettings does)
  detects: readonly ImportDetectedSource[]; // [] = anything (otherFile)
  lost: readonly LostKey[]; checklist: readonly ChecklistKey[];
}
export const importSources: readonly ImportSourceInfo[];   // in the order of the table below
export function sourceById(id: SourceId): ImportSourceInfo;
/** The format to preview with: the detected one, except a headerless Keeper CSV. `null` when nothing was detected. */
export function formatFor(source: ImportSourceInfo, detected: ImportDetected): ImportFormat | null;
/** True when something was detected and it is not what this source exports. */
export function isMismatch(source: ImportSourceInfo, detected: ImportDetected): boolean;
```

`formatFor`: `source.id === "keeper" && detected.source === "csv"` → `"keeperCsv"`; else `detected.format`.

Source table (EN guide text, menu paths `// unverified against the current product (launch gate)` as a comment above the table in `sources.ts` and in `importGuide.ts`):

| id | group | internalUrl | detects | steps (EN) | lost | checklist |
|---|---|---|---|---|---|---|
| chrome | browser | `chrome://password-manager/settings` | chromium | 1 Open this address in Chrome (copy it, paste it into the address bar). 2 Next to "Export passwords", click "Download file" and confirm with your computer's password. 3 Chrome saves "Chrome Passwords.csv" in Downloads. | passkeys | browserSaving, deleteOld |
| edge | browser | `edge://settings/passwords` | chromium | 1 Open this address in Edge. 2 Click "⋯" next to "Saved passwords", then "Export passwords", and confirm. 3 Edge saves the CSV in Downloads. | passkeys | browserSaving, deleteOld |
| brave / opera / vivaldi / arc | browser | `brave://password-manager/settings` / `opera://password-manager/settings` / `vivaldi://password-manager/settings` / `arc://password-manager/settings` | chromium | as Chrome, with the product name | passkeys | browserSaving, deleteOld |
| firefox | browser | `about:logins` | firefox | 1 Open this address in Firefox. 2 Click "⋯" at the top right, then "Export passwords…", and confirm. 3 Save "logins.csv" in Downloads. | passkeys | firefoxSaving, deleteOld |
| safari | browser | null | safari | 1 Safari → Settings → Passwords (macOS 14 and earlier; on macOS 15 use Apple Passwords). 2 Click "⋯" below the list, then "Export All Passwords…". 3 Save the CSV in Downloads. | passkeys | safariSaving, deleteOld |
| applePasswords | browser | null | safari | 1 Open the Passwords app. 2 File → "Export All Passwords to File…". 3 Save the CSV in Downloads. | passkeys | safariSaving, deleteOld |
| onePassword | manager | null | onePassword | 1 Open the 1Password app and unlock it. 2 File → Export → your account, then enter your password. 3 Pick "1PUX" and click "Export Data". | passkeys | oldExtension, deleteOld |
| bitwarden | manager | null | bitwarden, bitwardenCsv | 1 In the web vault or the app: Tools → Export vault. 2 File format ".json" (not "encrypted"). 3 Confirm with your master password. | attachments | oldExtension, deleteOld |
| keepass | manager | null | keepass | 1 Use your database file (.kdbx) as it is. 2 You'll enter its password in the next step; it stays on this device. | (none) | deleteOld |
| keepassxcCsv | manager | null | keepassxcCsv | 1 KeePassXC: Database → Export → CSV File… 2 Save it in Downloads. (The .kdbx file keeps more; prefer "KeePass".) | attachments, history | deleteOld |
| enpass | manager | null | enpass | 1 Enpass: Menu → File → Export. 2 Pick the vault, format ".json", enter your master password. 3 Save it in Downloads. | (none) | oldExtension, deleteOld |
| lastpass | manager | null | lastpass | 1 Sign in at lastpass.com and open Advanced Options → Export. 2 Enter your master password; the CSV downloads. 3 If the browser shows text instead, save the page as "lastpass.csv". | attachments, passkeys, history | oldExtension, deleteOld |
| dashlane | manager | null | dashlane | 1 Open Dashlane in your browser → My account → Settings → Export data. 2 Pick "CSV"; Dashlane downloads a ZIP. 3 Leave the ZIP as it is. | attachments, passkeys, history | oldExtension, deleteOld |
| keeper | manager | null | keeper, csv | 1 Keeper Web Vault → Settings → Export. 2 Pick "JSON" (keeps custom fields and 2FA codes) and export. | attachments, passkeys | oldExtension, deleteOld |
| nordpass | manager | null | nordpass | 1 NordPass → Settings → Export Items. 2 Confirm with your master password and save the CSV in Downloads. | attachments, passkeys | oldExtension, deleteOld |
| protonPass | manager | null | protonPass | 1 Proton Pass → Settings → Export. 2 Pick "ZIP" and leave encryption off (Mosaic can't read the PGP-encrypted export). 3 Unlock and save it in Downloads. | attachments | oldExtension, deleteOld |
| roboform | manager | null | roboform | 1 RoboForm → Settings → Account & Data → Export. 2 Pick "CSV" and "Logins", save in Downloads. | roboformExtras, passkeys | oldExtension, deleteOld |
| otherFile | other | null | (any) | 1 Export a CSV with a header row, or one of the formats above. 2 Choose the file; Mosaic maps the columns. | (none) | deleteOld |

Catalog keys in `importGuide.ts` (EN and DE, informal "du"; product and menu names stay as the product shows them, DE menu names as the German UI shows them):
`import.source.<id>` (plain names; `otherFile` = "Other CSV or file"), `import.grid.title` ("Where are your passwords now?"), `import.grid.browsers`, `import.grid.managers`, `import.grid.other`, `import.guide.<id>.<n>`, `import.guide.copyUrl` ("Copy address"), `import.guide.urlCopied`, `import.guide.choose` ("Choose the export file"), `import.guide.dropHint` ("…or drop one file here. It's read on this device only."), `import.detect.mismatch` ("This file looks like an export from {source}. Mosaic will read it as that."), `import.detect.detected` ("Recognised: {source}"), `import.detected.<ImportDetectedSource>` (display names), `import.lost.title` ("Won't come across"), `import.lost.<LostKey>` (e.g. passkeys: "Passkeys: {source} doesn't put them in its export. Keep {source} until you've set up new passkeys."), `import.lost.entriesSkipped` ({count}), `import.lost.fieldsDropped`, `import.lost.notesTruncated`, `import.lost.attachmentsSkipped`, `import.cleanup.title` ("Clean up"), `import.cleanup.delete` ("Delete {name} now"), `import.cleanup.deleted` ("{name} was overwritten and deleted."), `import.cleanup.deleteFailed` ("{name} couldn't be deleted. Delete it yourself."), `import.cleanup.ssdNote` ("On SSDs and in synced folders, older copies can survive: in version history, backups or the recycle bin. Check those too."), `import.cleanup.synced` ("This file is in your {service} folder. {service} may keep earlier versions; delete them there as well."), `import.synced.<ImportSyncedFolder>` (service names), `import.checklist.title`, `import.checklist.browserSaving` ("Turn off password saving and auto sign-in in {source}, so it doesn't offer to save alongside Mosaic."), `import.checklist.firefoxSaving` (about:preferences#privacy, "Ask to save passwords"), `import.checklist.safariSaving` (Settings → AutoFill → "User names and passwords"), `import.checklist.oldExtension` ("Turn off {source}'s browser extension so only one app fills."), `import.checklist.deleteOld` ("Delete your data in {source} once you've checked everything in Mosaic.").

- [ ] **Step 1: Write the failing tests in `sources.test.ts`**

```ts
it("lists browsers, then managers, then other", …)   // groups in order; ids unique; 21 sources
it("has every guide step in EN and DE", …)           // for n in 1..steps: catalogs.en/de have `import.guide.${id}.${n}`
it("picks the detected format", …)                   // formatFor(chrome, {source:"bitwarden",format:"bitwarden"}) === "bitwarden"
it("reads a headerless Keeper CSV as keeperCsv", …)  // formatFor(keeper, {source:"csv",format:"csv"}) === "keeperCsv"
it("returns null when nothing was detected", …)      // formatFor(chrome, {source:null,format:null}) === null
it("flags a mismatch only for another product", …)   // isMismatch(chrome, chromium) false; (chrome, bitwarden) true;
                                                      // (otherFile, keeper) false; (chrome, {source:null}) false
```

- [ ] **Step 2: Run, expect FAIL** — `pnpm --filter @mosaic/desktop test src/screens/import/sources`
- [ ] **Step 3: Implement table and strings**
- [ ] **Step 4: Run** Step 2 command and `pnpm --filter @mosaic/i18n test` (parity, placeholders) — PASS
- [ ] **Step 5: Commit** `feat(desktop): import sources and guides (EN, DE)`

---

### Task 17: Guided import flow

**Files:**
- Create: `apps/desktop/src/screens/import/{SourceGrid,SourceSteps,NotComing,CleanupStep}.tsx`
- Modify: `ImportFlow.tsx` (step machine), `ImportFlow.test.tsx`, `apps/desktop/src/styles/pages.css` (or `auth.css` next to `.dropzone`; tokens only)

**Interfaces:**
- Consumes: Tasks 13 to 16 (`import_choose_file` null, `import_detect`, `import_delete_file`, `useEvents().importDrop`, `importSources`, `formatFor`, `isMismatch`).
- Step machine in `ImportFlow.tsx`:

```ts
type Step =
  | { name: "source" }
  | { name: "steps"; source: SourceId }
  | { name: "file"; source: SourceId; token: string; fileName: string; detected: ImportDetected }
  | { name: "preview"; source: SourceId; token: string; fileName: string; detected: ImportDetected; preview: ImportPreview }
  | { name: "cleanup"; source: SourceId; token: string; fileName: string; synced: ImportSyncedFolder | null; result: ImportResult };
```
- `export function notComing(preview: ImportPreview, source: ImportSourceInfo): { key: MessageKey; values?: Record<string, string | number> }[]` in `NotComing.tsx`: the source's `lost` keys (with `{source}`), then from the preview: `entriesSkipped` (count = `preview.skipped.length`) if any, `attachmentsSkipped` if that warning is present, `fieldsDropped`/`notesTruncated` if any entry carries them, and the existing passkey warnings' texts; no duplicates.
- Behaviour: grid of buttons (name + generic `Icon`; no `<img>`); steps screen numbers the guide, shows `internalUrl` in a code-styled field with a copy button, then "Choose the export file" (calls `import_choose_file`) and the drop hint; an `importDrop` while on the steps screen is taken like a chosen file and cleared. After choosing: `import_detect`; `formatFor` null → the `importFormatInvalid` error message, stay on steps; mismatch → `import.detect.mismatch` callout (info tone), not blocking; `keepass` → password field (existing). Preview step = existing `PreviewList` + target select + the `NotComing` list above the commit button. Commit → cleanup step: result line (existing `import.result`), delete button, `ssdNote` always, `synced` note when set, checklist from `source.checklist` (plain list; `internalUrl` copy button again for `browserSaving`), "Done". Cancel keeps calling `import_cancel` for a held preview.

- [ ] **Step 1: Write the failing tests in `ImportFlow.test.tsx`** (mock IPC, `languages={["en"]}`)

```ts
it("shows browsers and managers by name without logos", …)       // buttons "Chrome", "LastPass", "Other CSV or file"; container.querySelector("img") === null
it("guides Chrome and copies its internal address", …)            // click Chrome → "chrome://password-manager/settings" visible; "Copy address" → navigator.clipboard.writeText called with it
it("shows a mismatch notice when the file is another product's", …) // importDetected {source:"enpass",format:"enpass"} after picking Chrome → "This file looks like an export from Enpass."
it("refuses an unrecognised file before the preview", …)           // importDetected {source:null,format:null} → format-invalid error, no "Preview" button
it("lists what won't come across", …)                              // Chrome → preview → "Won't come across" with the passkeys line
it("commits and offers to delete the export", …)                   // commit → "Delete enpass-export.json now" → click → "…was overwritten and deleted."
it("cleanup reports a failed delete", …)                           // importDeleteFails → "enpass-export.json couldn't be deleted" and no "deleted" text
it("names a synced folder and shows the checklist", …)             // importDetected.syncedFolder "oneDrive" → OneDrive note; Chrome checklist line visible
it("takes a dropped file on the steps screen", …)                  // emit importFileDropped → detection runs, file name shown
```
Unit tests for `notComing` (in `NotComing.test.ts`): keeper source + preview with 2 skipped and `attachmentsSkipped` → keys `[lost.attachments, lost.passkeys, lost.entriesSkipped(2), lost.attachmentsSkipped]` in that order, no duplicates.

- [ ] **Step 2: Run, expect FAIL** — `pnpm --filter @mosaic/desktop test src/screens/import`
- [ ] **Step 3: Implement** (components small; CSS only `var(--…)` tokens, checked by `styles.test.ts`)
- [ ] **Step 4: Run** `pnpm --filter @mosaic/desktop test src/screens/import src/styles`, `pnpm --filter @mosaic/desktop typecheck`, `pnpm --filter @mosaic/desktop lint` — PASS
- [ ] **Step 5: Commit** `feat(desktop): guided import with a "won't come across" list and cleanup`

---

### Task 18: First run and Settings → Import

**Files:**
- Modify: `apps/desktop/src/screens/onboarding/OnboardingFlow.tsx` (`ImportOffer` copy only), `packages/i18n/src/catalog/{en,de}/onboarding.ts`, `apps/desktop/src/screens/SettingsScreen.tsx` (+ test), `apps/desktop/src/screens/vault/VaultLayout.tsx` (pass `onImport`), `packages/i18n/src/catalog/{en,de}/settings.ts`

**Interfaces:**
- The onboarding order stays kit → verify → import offer (the first optional step, skippable) → browser. `onboarding.import.body` names the sources: "From Chrome, Edge, Firefox, Safari and other browsers, or from 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass and Enpass. It takes a few minutes." (DE informal).
- `SettingsScreen({ onImport }: { onImport: () => void })`: a `SettingsSection` titled `settings.import` ("Import") with body `settings.import.body` and a button `settings.import.open` ("Import passwords") calling `onImport`; `VaultLayout` passes `() => shell.go("import")`.

- [ ] **Step 1: Write the failing tests**

```ts
// SettingsScreen.test.tsx
it("opens the import from settings", …)   // click "Import passwords" → onImport called once
// OnboardingFlow.test.tsx (extend the existing import-offer case)
it("offers the import first after the kit, and skipping goes on", …) // heading "Bring your passwords along", body mentions "LastPass"; "Skip for now" → browser step
```

- [ ] **Step 2: Run, expect FAIL** — `pnpm --filter @mosaic/desktop test src/screens/SettingsScreen src/screens/onboarding`
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** Step 2 command and `pnpm --filter @mosaic/i18n test` — PASS
- [ ] **Step 5: Commit** `feat(desktop): import in first run and settings`

---

### Task 19: Website text

Text only, no deep link (spec §11.4); claim only what Tasks 1 to 18 support.

**Files:**
- Modify: `marketing/landing-copy.en.md` (Import row near line 141, FAQ 6), `marketing/landing-copy.de.md` (same places), `marketing/comparison.md` (import row, if it lists sources). If the switch-guide or checklist pages exist under `apps/web` at execution time (branch `feat/web`), apply the same text there.

- [ ] **Step 1: Replace the source list** with: browsers (Chrome, Edge, Brave, Opera, Vivaldi, Arc, Firefox, Safari, Apple Passwords) and managers (1Password, Bitwarden, KeePass/KeePassXC, Enpass, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, and CSV); add "Open Mosaic, then Settings → Import" and "Mosaic helps you delete the export afterwards". No "audited"/"bank-grade" wording; Proton Pass and Bitwarden are named with "unencrypted export".
- [ ] **Step 2: Check** — `git diff --stat marketing` shows only these files; grep the changed files for `audited` and `bank-grade` returns nothing.
- [ ] **Step 3: Commit** `docs(marketing): list the import sources and the in-app path`

---

### Task 20: ADR, launch-gate checklist, full gate

**Files:**
- Modify: `docs/DECISIONS.md` (next free ADR number), `docs/PROGRESS.md` (new section "Onboarding import: launch gate"), `docs/THREAT_MODEL.md` §4.1 "D" if it lists the import formats

**ADR "Onboarding import: detection, deletion and unverified presets"** records: format chosen by content (`detect`), the picked source only as a hint, except the headerless Keeper CSV; delete only after a commit from the same file token, overwrite once then delete, with the SSD/synced note; non-origin login URLs kept as text fields (`UrlKeptAsText`); drops handled in Rust; presets written from public descriptions and marked unverified; PGP Proton, Dashlane `.dash` and protected Bitwarden exports refused (`Encrypted`); one fuzz target per parser entry point.

**Launch-gate checklist** (copy into PROGRESS.md as unchecked items; each needs one real, current export with synthetic content, from the owner or a test account):

- [ ] LastPass CSV: header `url,username,password,totp,extra,name,grouping,fav`; `http://sn`; NoteType names and keys for Credit Card, Address, Wi-Fi Password; expiry format; grouping separator; HTML entities or not; the export menu path.
- [ ] Dashlane: ZIP member names and headers (`credentials.csv` incl. `otpSecret` vs `otpUrl`, `securenotes.csv`, `payments.csv`, `ids.csv`, `personalInfo.csv`); the web-app menu path.
- [ ] Keeper JSON: `records`, `$type` values, `custom_fields` key forms (`$text:Label`, `$paymentCard`, `$name`, `$phone`, `$address`, `$oneTimeCode`), `folders`/`shared_folder`; Keeper CSV column order and whether a header row now exists; the menu path.
- [ ] NordPass CSV: header incl. `additional_urls` and `custom_fields` encodings, `type` values, folder rows; the menu path.
- [ ] Proton Pass: ZIP layout (`Proton Pass/data.json`), item `type` values, `content` keys per type, `extraFields`, `state`, `pinned`, passkeys and files; the export options (ZIP vs PGP vs CSV); the menu path.
- [ ] RoboForm: logins CSV header and `RfFieldsV2` content, folder form, encoding (UTF-8 or UTF-16); identity and safenote exports; the menu path.
- [ ] KeePassXC CSV: header incl. `TOTP`, `Icon`, `Last Modified`, `Created`; root group name; recycle bin entries.
- [ ] Every browser guide (Chrome, Edge, Brave, Opera, Vivaldi, Arc, Firefox, Safari, Apple Passwords) and every manager guide checked against the current product, EN and DE UI; every internal URL opens the right page.
- [ ] Each browser's "turn off saving" path in the checklist.
- [ ] Synced-folder detection checked on Windows (OneDrive Known Folder Move) and macOS (`~/Library/CloudStorage`).
- [ ] Red-team pass on the importers (spec §7): zip bombs, homoglyph URLs, huge notes, malformed UTF-8, duplicate JSON keys; fuzz targets run in CI without findings.
- [ ] Remove each `// unverified against a current export (launch gate)` comment only after its item above is checked.

- [ ] **Step 1: Write the ADR and the PROGRESS section**
- [ ] **Step 2: Run the full gate once** (CLAUDE.md "Commands"; prefix the Rust workspace runs with `FULL_RUN_OK=1`, use the build env from Global Constraints, `--test-threads=2` for the CLI tests). Expected: all pass.
- [ ] **Step 3: Run the relevant review agents once** (crypto-reviewer is not needed: no crypto changed; run test-engineer on `crates/import` and ux-reviewer on the import screens) and fix findings in one round.
- [ ] **Step 4: Commit** `docs: onboarding import ADR and launch gate` (the orchestrator squashes the milestone)
