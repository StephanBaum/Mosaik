# Onboarding import: every browser and the common password managers

Status: approved direction, 2026-11-15 (owner: "lets develop all the features we discussed"); builder-check answers in §11
Owner direction: "we should care about the import from all browser and common passwordmanagers to make onboarding a breeze" (launch scope, `launch-direction`).

## 1. Intent

A new user moves everything into Mosaic in a few minutes, from wherever their logins live today, without
reading a manual and without leaving a plaintext export lying around afterwards. Switching is the moment
people decide whether to stay, so it has to feel effortless and safe.

Success looks like: the user picks where they come from, follows two or three concrete steps, sees a preview
of what will arrive (with anything that will not come across named), confirms, and is told how to clean up.

## 2. Goals, non-goals, anti-goals

**Goals**
- Import from the browsers: Chrome, Edge, Brave, Opera, Vivaldi, Arc (all Chromium CSV), Firefox, Safari
  and Apple Passwords. (All already parse; this adds the guided flow.)
- Import from the common managers: 1Password, Bitwarden, KeePass/KeePassXC, Enpass (already parse) plus
  **LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm** (new).
- A guided first-run flow on desktop (and the existing CLI import). Import on the phone follows after launch (§5).
- Cleanup: offer to delete the export file, and a short per-source checklist (turn off the browser's own
  password saving, remove the old manager's data when ready).

**Non-goals**
- Reading a browser's password store directly from its profile. Chrome on Windows binds its encryption to
  Chrome itself (app-bound encryption, since 2024), macOS needs the user's keychain consent per item, and
  Firefox would need its NSS key handling re-implemented. A guided export is reliable everywhere and needs no
  new cryptography. (My call; see §9.)
- Live sync or a "bridge" with another manager. One-shot import only.
- Importing from a manager's cloud API with the user's account credentials. Mosaic never asks for another
  service's password.

**Anti-goals** (would make this a failure even if it works)
- A plaintext export left on disk because we did not say so, or a temp copy we made ourselves.
- Silent loss: an item, field, passkey or TOTP that does not come across without being named in the preview.
- Any new cryptography of our own, or a crypto dependency without an ADR.
- A parser an attacker can crash, hang or make allocate unbounded memory with a crafted file.

## 3. Constraints (from CLAUDE.md and existing ADRs)

- Never invent cryptography; decryption of protected exports only with CRYPTO_SPEC §2 primitives or an ADR.
- Secrets in zeroizing types, never logged, never in errors.
- All UI strings through `packages/i18n` (DE informal "du", EN); styling through design tokens.
- Existing import pipeline stays the spine: `crates/import` → `ItemDraft` → preview → commit, with the
  existing limits (`MAX_ATTACHMENTS_PER_ENTRY`, `MAX_FIELDS_PER_ENTRY`, size caps) and `EntryWarning`s.
- Every new parser gets fuzz targets and the red-team round planned before launch.

## 4. The flow (desktop)

1. **Where are you coming from?** A grid of sources with their names (no third-party logos: trademark
   risk; plain names and a generic icon). Browsers first, then managers, then "Other CSV / file".
2. **Steps for that source.** Two to four numbered steps with the exact menu path for the current version of
   that product (for example Chrome: Settings → Passwords and autofill → Google Password Manager → Settings →
   Export passwords). A "copy" button for internal URLs such as `chrome://password-manager/settings`, which a
   page cannot open by link.
3. **Pick the file.** A file dialog opened in the user's Downloads folder, plus drag and drop. The format is
   detected from content, not from the extension or the source the user picked; a mismatch is shown
   ("This looks like a Firefox export").
   - My call: no automatic watching of the Downloads folder. A watcher reading every new download for a few
     minutes is a surprise for a security product, and the file dialog is one click.
4. **Preview.** The existing preview: counts per type, items with warnings, duplicates of existing items,
   and an explicit list of what will not come across (unsupported field types, attachments over the limit,
   passkeys the source exports without private keys).
5. **Import.** One commit; a failure leaves the vault unchanged (existing behaviour).
6. **Clean up.**
   - "Delete the export file now": overwrite once, then delete. The screen says plainly that on SSDs and
     synced folders a copy may survive elsewhere (Dropbox/OneDrive version history, the recycle bin), and
     names the folder if it is inside a known synced folder.
   - The per-source checklist: turn off the browser's password saving and offering (so the browser and
     Mosaic do not both prompt), and "delete your data in the old manager once you have checked everything".

Re-import of the same file is safe: duplicates are detected by the existing rules and shown, not doubled.

## 5. Android (after launch)

Owner decision 2026-11-15: at launch the import runs on desktop; sync brings the items to the phone. The
phone import below follows later, together with account creation on the phone.


- "Import" in settings opens the system file picker (Storage Access Framework); the same core parsers run
  through `mosaic-mobile-ffi`; the same preview and warnings, the same cleanup step (the file is deleted
  through the picker's URI only if the user agrees and the provider allows it).
- The guided steps are the same text, shortened, with the mobile path where the source has an app
  (Bitwarden, Proton Pass, NordPass mobile exports) and "do this on a computer" where it does not.

## 6. New formats

For each: the current export path, the file shape, what maps to what, and what is lost. Header names and
field lists must be verified against a real, current export before the parser is written (exports change);
fixture files are synthetic, never real user data.

| Source | Export | Notes |
|---|---|---|
| LastPass | CSV (`url,username,password,totp,extra,name,grouping,fav`) | `extra` holds secure notes and "NoteType:" structured items (cards, addresses): map the known note types, keep the rest as a note. `http://sn` URL marks a secure note. |
| Dashlane | CSV set in a ZIP (credentials, secure notes, payments, ids, personal info) | Read the ZIP with the existing size limits; the encrypted `.dash` backup is out of scope (undocumented format). |
| Keeper | JSON (preferred) or CSV | JSON keeps custom fields, TOTP and folders; shared folders become folders. |
| NordPass | CSV with a `type` column (password, note, credit card, identity) | |
| Proton Pass | ZIP with `Proton Pass/data.json` (unencrypted) | The PGP-encrypted export needs OpenPGP: out of scope for launch unless an audited crate passes an ADR; the guide tells the user to pick the unencrypted export and delete it afterwards. |
| RoboForm | CSV (`Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2`) | Identities and safenotes from their own CSVs if present. |
| KeePassXC CSV | CSV (`Group,Title,Username,Password,URL,Notes,TOTP,...`) | KDBX stays the recommended path; the CSV preset is for users who already exported. |
| Bitwarden protected JSON | `passwordProtected: true` | Not at launch: PBKDF2 is not in CRYPTO_SPEC §2. The guide asks for the plain JSON export (and deletes it afterwards); the existing `Error::Encrypted` message says so. |

## 7. Security requirements

- Parsers are pure functions over bytes with hard limits (file size, entries, fields, nesting, ZIP entry
  count and decompressed size, ratio), no recursion on attacker-controlled depth, no panics.
- One fuzz target per new format, run in CI's fuzz job; a red-team pass on the importers before launch
  (zip bombs, CSV injection into later CSV exports, homoglyph URLs that change autofill matching, huge
  notes, malformed UTF-8, duplicate keys in JSON).
- Imported URLs go through `mosaic-origin` normalisation; anything that is not a valid origin is kept as a
  note field and never used for autofill matching.
- Imported `androidapp://` URLs keep the existing rules (no cert ⇒ "imported", ADR-0092 one-tap rules).
- No export file content is ever logged; the preview runs in memory; nothing is written to disk except the
  committed vault.

## 8. Where it shows

- First run: after the vault is created, "Bring your passwords" is the first suggested step (skippable).
- Settings → Import, always.
- Website: the existing switch guide and checklist pages link to the in-app flow; copy only claims sources
  that are actually supported.

## 9. Calls I made (please check)

1. **No direct browser-profile reading** (reliability and no new crypto); guided export instead.
2. **No Downloads-folder watcher**; a file dialog opened in Downloads.
3. **No third-party logos**, names only.
4. **Proton Pass PGP and Dashlane `.dash` encrypted exports are out of scope** at launch.
5. ~~Android gets import~~: owner, later, together with account creation on the phone.

## 10. Left to the builder

Exact screen layout within the design system; the wording of each guide (DE/EN, verified against the
products' current menus); the mapping tables per format; how the preview groups warnings.

## 11. Builder-check answers (2026-11-15)

1. **Android:** no import on the phone at launch (owner). Account creation on the phone: wanted, later.
   Separate launch item with its own design: **Android folder sync** for free-plan users.
2. **Bitwarden password-protected export:** out (conservative; no PBKDF2 without a spec change). My call.
3. **Samples:** parsers are written from the vendors' public export documentation and public format
   descriptions (no code copied from GPL importers), each preset marked `unverified` in a comment until a real,
   current export (synthetic content) has been checked. Checking one real export per source and every guide's
   menu path against the current product is a **launch gate** (the owner can supply exports; otherwise a test
   account). My call.
4. **Website to app:** text instructions only ("Open Mosaic, Settings, Import"); no custom URL scheme or deep
   link, which would be new attack surface. My call.
