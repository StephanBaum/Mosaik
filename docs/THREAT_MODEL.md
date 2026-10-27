# Mosaic Threat Model

Status: v1 draft (M0). It is reviewed at every milestone and fully re-run in M9.
Crypto details are in `CRYPTO_SPEC.md`, and section references (§) point
there. Extension specifics will be expanded in `EXTENSION_SECURITY.md` (M4).

## 1. Assets

| ID | Asset | Worst outcome if lost |
|---|---|---|
| A1 | Item plaintext (passwords, TOTP secrets, passkey private keys, notes, files) | account takeover everywhere |
| A2 | Master password, Secret Key | full vault compromise |
| A3 | Unwrapped keys in memory (AK, VK, isk) | full vault compromise while unlocked |
| A4 | Integrity of items (URL ↔ credential binding) | credentials filled into phishing sites |
| A5 | Availability of the vault (sync data, local DB) | lock-out, data loss |
| A6 | Metadata (who shares with whom, item counts, usage times) | privacy loss |
| A7 | Build and release pipeline, signing keys | malicious update to every user |

## 2. Trust boundaries

```
 [User] ─ UI ─ [Desktop app (Tauri) ── core (Rust)] ── IPC ── [native-messaging host] ── stdio ── [Extension background SW]
                       │                                                                         │ runtime messages
                       │ TLS 1.3                                                                  [Content script] ── DOM ── [Web page (untrusted)]
                       ▼
                 [Sync server (axum)] ── [PostgreSQL]
 [CLI] ── core ── TLS ──┘           [Mobile app + OS autofill/passkey provider] ── core (UniFFI)
```

Untrusted: web pages, the network, the sync server (honest-but-curious at
best, malicious at worst), other extensions, import files, and anything
received over IPC before authentication.

Trusted: the OS kernel and the logged-in user's session (with the caveats in
P5), the browser's extension isolation, the Rust core, and the platform
keystores.

## 3. Attacker profiles

| ID | Attacker | Capabilities | Goal |
|---|---|---|---|
| P1 | **Stolen server DB** (or malicious server operator) | full DB and server code, can modify responses, cannot break crypto | read vaults, inject data, track users |
| P2 | **Malicious website** | arbitrary HTML/JS/CSS, iframes, lookalike domains, timing | get credentials filled or exfiltrated, clickjack the user |
| P3 | **Malicious extension in the same browser** | its own permissions, can message other extensions and read/modify page DOM if it has host permissions | read filled credentials, spoof messages to our extension or native host |
| P4 | **Network attacker** | MITM, DNS spoofing, captive portals | read or modify sync, downgrade TLS, steal fills on HTTP |
| P5 | **Local malware without admin/root** | runs as the user: reads user files, connects to user-level IPC, screenshots, clipboard, keylogging in some OSes | read vault, approve actions as user |
| P6 | **Lost or stolen device, locked** | disk and hardware, offline time | decrypt vault |
| P7 | **Lost or stolen device, unlocked** | physical access to a running session | read or export vault, change master password |
| P8 | **Compromised build dependency** | malicious code in a crate or npm package, or in a GitHub Action | exfiltrate secrets at runtime, backdoor releases |

Out of scope (stated in the README and `SECURITY.md`): malware running as
admin/root or kernel-level, a compromised OS or hardware, cold-boot or DMA
attacks on a running machine, and coercion of the user. Partially in scope:
P5. We raise its cost, but a user-level attacker who is actively watching an
unlocked session can usually read what the user sees.

## 4. STRIDE per component

Mitigations name the milestone in which they are implemented and tested.

### 4.1 Rust core (`crates/core`, `totp`, `generator`, `passkey`, `import`)

| STRIDE | Threat | Mitigation |
|---|---|---|
| S | Forged ciphertext accepted as a real item | AEAD with ID-bound AAD (§7.2), M1 |
| T | Item moved between vaults or items swapped by the server | `vault_id` + `item_id` in AAD, M1 |
| T | Rollback to an old item version | authenticated version vector in plaintext, monotonic acceptance (§10), M2/M6 |
| R | User disputes a change | per-item history and device ID in plaintext, M2 |
| I | Secrets leaked via logs, panics, `Debug`, crash reports | redacting secret types, lint rules, `panic=abort`, M1 |
| I | Length leakage | padding (§7.2), M1 |
| I | Timing side channels in comparisons | `subtle`, constant-time library code, M1 |
| I | Secrets left in memory after lock | `zeroize`, minimal lifetimes, M1. Swap is a residual risk (ADR-0005). |
| D | Malicious import file causes OOM or hang | bounded parsers, size limits, fuzzing, M3 |
| D | Hostile server sends huge Argon2 parameters | upper bounds (§5.3), M1 |
| E | Memory-unsafety exploit in a parser | `#![forbid(unsafe_code)]`, fuzzing, M1+ |

### 4.2 Desktop app (Tauri)

| STRIDE | Threat | Mitigation |
|---|---|---|
| S | Another local process impersonates the native host on IPC | peer verification (process path and code signature) plus a pairing key, M4 |
| T | Malicious webview content (XSS in rendered item fields) | strict CSP, no remote content, React escaping, no `dangerouslySetInnerHTML`, a narrow Tauri capability allow-list, M3 |
| R | Silent export or change | audit log entries for export, password change, share, M3 |
| I | Clipboard sniffing | auto-clear after 30 s, exclusion from clipboard history (Windows `ExcludeClipboardContentFromMonitorProcessing`, macOS `org.nspasteboard.ConcealedType`), M3 |
| I | Screen capture of revealed secrets | reveal on demand only, auto-hide, Windows `SetWindowDisplayAffinity`, macOS `sharingType = none` where available, M3 |
| I | Unlocked vault on unattended machine (P7) | auto-lock on idle, sleep, screen lock. Re-auth for reveal-all, export, password change and the Secret Key view, M3 |
| D | Corrupted local DB | SQLite WAL, integrity check, server re-sync, M2/M6 |
| E | Webview reaches the Tauri API surface | the capability grants only Mosaic's own commands (no `core:*`, no plugin with a JavaScript API, events over a `Channel`), typed inputs validated in Rust, strict CSP, frozen `Object.prototype`, no devtools in release builds (ADR-0019, M3). The isolation pattern is deferred to M9: it needs AES-GCM outside CRYPTO_SPEC §2 and could only re-check what Rust validates (ADR-0019 item 4) |
| E | Malicious update | signed updates (Tauri updater with an offline-held key), M9 |

### 4.3 Browser extension (highest risk)

| STRIDE | Threat | Mitigation |
|---|---|---|
| S | Phishing site that looks like the real one (P2) | exact-origin matching, punycode/homoglyph warning, M4 |
| S | Page or other extension sends spoofed `runtime` messages (P2, P3) | accept only from our own contexts (`sender.id`, `sender.origin`, `sender.tab` checks). No `externally_connectable` for web origins, M4 |
| T | Page restyles or overlays injected UI (clickjacking) | ADR in M4: closed shadow root with occlusion checks, or popup-only, M4 |
| T | Hidden fields harvest extra data | fill only visible, focused-form fields, and only the fields the user chose, M4 |
| I | Auto-fill on load exfiltrates credentials | never fill without a trusted user gesture, M4 |
| I | Cross-origin iframe receives a fill | refused. Same-origin iframes need confirmation, M4 |
| I | Content script compromised by page (shared DOM) | content script gets one credential at a time, only at fill time, M4 |
| I | Fill over HTTP read by a network attacker (P4) | refused unless overridden per item, M4 |
| D | Page floods the extension with events | rate limits in the background worker, M4 |
| E | Over-broad permissions abused after compromise | minimal manifest permissions, runtime host permissions, M4 |

### 4.4 Native messaging channel

| STRIDE | Threat | Mitigation |
|---|---|---|
| S | Another extension launches our host | `allowed_origins` lists only our extension IDs. The host checks the caller origin argument, M4 |
| S | Local malware talks to the desktop app directly (P5) | authenticated pairing (user confirms a code on first connection). Messages are authenticated with the pairing key, M4 |
| T | Frames altered in transit | authenticated framing, strict length limits, M4 |
| I | Vault data leaked over the channel | the request/response API returns only the requested credential after user approval. There is no list or dump API, M4 |
| D | Oversized frames | 1 MiB frame cap (Chrome's host→extension limit), fuzzed decoder, M4 |

### 4.5 Sync server

| STRIDE | Threat | Mitigation |
|---|---|---|
| S | Credential stuffing or online guessing | OPAQUE plus rate limiting per IP and per account, optional account 2FA, M6 |
| S | Offline guessing after DB theft (P1) | OPAQUE input includes the 128-bit SK (§9.2), M1/M6 |
| T | Server injects or alters records | AEAD, HPKE Auth shares, authenticated tombstones, M6 |
| T | Server serves old data (rollback/freeze) | monotonic version vectors, checkpoint freeze detection (§10), M6 |
| R | Disputed account actions | append-only audit log without secrets, M6 |
| I | Metadata analysis (P1) | minimal metadata (CRYPTO_SPEC §12), padding, short IP retention, no analytics, M6 |
| I | Account enumeration | fake salts and fake OPAQUE responses, uniform errors and timing, M6 |
| D | Storage or connection exhaustion | quotas, WebSocket limits, body size limits, M6 |
| E | SQL injection, deserialization bugs | `sqlx` with bound parameters, strict typed requests, fuzzed decoders, M6 |

### 4.6 Sharing and emergency access

| Threat | Mitigation / residual risk |
|---|---|
| Server substitutes a recipient's public key (P1) | safety codes, key pinning and change warnings (§8.2). **Residual:** unverified contacts can be MITM'd by a malicious server |
| Removed member keeps old data | VK rotation on removal (§8.3), attachments re-sealed under fresh file keys (ADR-0017, R18). **Residual:** data already downloaded stays exposed |
| Server releases emergency access early | **Accepted:** the waiting period is enforced by the server (§8.4) |

### 4.7 CLI

| Threat | Mitigation |
|---|---|
| Secrets in shell history or process args | secrets never passed as args. The `inject` subcommand passes them via env or stdin |
| Secrets left in env of child processes | `run` injects only the requested variables into the child process |
| Session cached on disk | session key in the OS keystore only, with a short TTL |

### 4.8 Mobile (M10)

| Threat | Mitigation |
|---|---|
| AutoFill extension memory limits force weaker KDF | never run Argon2 in the extension. Use quick unlock via the shared keychain (§5.3, §6.2) |
| Other apps read the clipboard | iOS `UIPasteboard` local-only with expiry. Android `ClipDescription.EXTRA_IS_SENSITIVE` |
| App switcher screenshots | secure flag or blur on background |

### 4.9 Supply chain (P8)

| Threat | Mitigation |
|---|---|
| Malicious or typosquatted dependency | lockfiles committed, `cargo-deny` (sources, bans, licenses), `cargo-audit`, `pnpm audit`, minimal dependencies with written justification, M0 |
| Compromised GitHub Action | actions pinned by commit SHA, least-privilege `permissions:`, M0 |
| npm install scripts | `pnpm` with lifecycle scripts disabled except an explicit allow-list (`onlyBuiltDependencies`), M0 |
| Tampered release | signed artifacts, SBOM, reproducible-build instructions, M9 |
| Compromised developer machine | signing keys kept off developer machines (hardware token or CI OIDC with environment protection), M9 |

## 5. Residual risks register

| ID | Risk | Status |
|---|---|---|
| R1 | Local malware (P5) watching an unlocked session can read what the user sees | accepted. We document it and raise the cost |
| R2 | Device stolen while locked (P6): an attacker who extracts SK from the keystore can run an offline Argon2id attack on MP | accepted. Strength meter, Argon2 parameters, optional hardware-key-only unlock |
| R3 | MITM of unverified sharing contacts by a malicious server | accepted, with a safety-code UX |
| R4 | Server-enforced emergency waiting period | accepted |
| R5 | Secrets may be paged to swap | open, ADR-0005 (revisit in M9) |
| R6 | Browser/OS zero-days | out of scope |
| R7 | Shared vaults: any member holding VK can create items (read-only is enforced by the server only). HPKE Auth is open to key-compromise impersonation (CRYPTO_SPEC §8.2) | accepted for v1. Per-author signatures are a v2 candidate |
| R8 | Hardware-key unlock: key + PIN + a copy of the wrapping unlocks without MP/SK (CRYPTO_SPEC §6.3) | accepted. It is opt-in, needs UV, and the UI explains it |
| R9 | Changing MP or SK does not revoke someone who already holds old secrets plus an old wrapping. Only a full re-key does (CRYPTO_SPEC §6.5–6.6) | mitigated by the re-key flow and explicit UI copy |
| R10 | Pre-login profile distribution drift gives weak statistical hints about which accounts exist (CRYPTO_SPEC §9.2) | accepted |
| R11 | Upstream `hmac`/`hkdf` leave short-lived stack copies during derivation: the `key ^ ipad/opad` block (for `R` and the KEK/OPW step that is `R`-equivalent) and the last HKDF output block. They cannot be zeroized without patching the crates. Exploiting them needs a memory read of the process during or right after unlock, which is P5 (R1) territory | accepted, ADR-0005 / ADR-0007 item 9. Revisit with R5 in M9 or when upstream wipes them |
| R12 | A removed wrapped key (expired quick unlock, old password wrapping) is overwritten in the local database and its WAL (`secure_delete`, WAL truncation), but the file system or SSD can keep old blocks, and backups or snapshots of the database keep it. Recovering AK from it still needs that method's `W` (or MP + SK) | accepted, ADR-0010. Full re-key (CRYPTO_SPEC §6.6) is the remedy after a compromise |
| R13 | Two writers sharing a writer ID produce equal edit stamps for different edits; when they edit different fields each side's version vector covers the other's stamp and the pure merge drops both values. Until the third M2 review the ID lived in the store file, so a restored backup or copied profile kept it, and even the documented recovery lost data. Remaining ways to share an ID: a forged or corrupted record (a vault member, R7), a UUIDv7 collision, and two sessions on one store through a hard link (the store lock follows the resolved path, not the inode) | mitigated: a fresh writer ID for every unlock (ADR-0015), so copies and restores never share one; the store lock on the resolved path (ADR-0010 item 10 and amendment); equal vectors with different contents and IDs rotated after `DeviceIdReused` are merged without losing a value and reported as `stamp_collision` (ADR-0015). Hard links: accepted, a copy through a hard link is an unusual setup and each session still writes under its own ID |
| R14 | `serde_json` may keep short unwiped copies of strings with escape sequences in its scratch buffer while item plaintext is parsed. Reading them needs a memory read of the unlocked process (R1) | accepted, ADR-0009 item 2. Revisit with R5 in M9 |
| R15 | Gaps of the M2 merge model (PROGRESS M2-R7 to M2-R10): a delete racing an edit can restore a device's stale value over a newer one it never saw, depending on delivery order (the newer value is lost); a delete racing a move of the same item, or two concurrent moves, leave live copies without a notice; a delete or move that loses against an edit has already pruned the attachment files on its device; whether the delete-race repair runs depends on which versions a device receives (if the device that won the race dismisses the notice before a third device pulls, and the server keeps only the latest version, the third device never sees the deletion and keeps the deleter's state without a notice). None needs an attacker; the server cannot cause them beyond reordering, which it can do anyway (§10 freeze attack) | accepted for M2. Fixes need the server's record history or move pointers in the tombstone, so they are designed with the M6 protocol (spec and ADR first) |
| R16 | An edit the server accepted under key generation `n` is lost if another device rotates to `n + 1` before pulling it and the editing device never comes back (lost or wiped). The rotating device re-seals only what it holds, and every peer that accepts `n + 1` before applying the record refuses it as a rollback (§7.1); whether a peer keeps the edit depends on delivery order. The same applies to a move into a vault that a peer rotates concurrently: the peer refuses the copy and fast-forwards to the source's tombstone, so the whole item vanishes there without a notice until the mover accepts `n + 1` and re-seals the copy (fourth M2 review, characterization test). No attacker needed; a malicious server can cause the same by withholding, which it can do anyway (§10 freeze attack) | accepted for M2 (PROGRESS M2-R12). M6 makes a rotation conditional on the vault head (the server accepts the wrapped key of `n + 1` only from a device that has applied every record up to the vault's latest `server_seq`) and refuses uploads below the current generation; a move uploads its source tombstone only after the server accepted the copy, and a peer that cannot read the copy flags the source instead of fast-forwarding to the tombstone; spec and ADR first |
| R17 | A record or vault-metadata row that cannot be read (damaged, or sealed under a key the device no longer has) gives no rollback anchor: the next received version at the current key generation replaces it without a version check, so a malicious server can serve any older version it holds of that item, or an older vault name or description, at that generation | mitigated: the replacement is reported (`ApplyOutcome::ReplacedUnreadable`, for metadata since the fourth M2 review) so the UI flags the item or vault; one damaged key row no longer makes every older record anchor-less (ADR-0012 amendments). M6 can require freeze-checkpoint evidence or a full re-fetch for such replacements |
| R18 | Revoking a vault member (§8.3) without re-sealing attachments leaves their file keys with the removed member, who together with the server could replace an attachment's ciphertext undetected (the attachment AAD has no key generation) | open until sharing exists (M6/M7): CRYPTO_SPEC §8.3 and ADR-0017 require the re-seal under fresh IDs and file keys; not reachable in M2, where no other account holds a vault key |
| R19 | Secrets in the desktop webview: what the user types (master password, Secret Key, new item secrets) and what Rust hands out on request (a revealed field, a generated password, a one-time code, the Secret Key once at account creation) live in the webview's JavaScript heap, which cannot be zeroized; the IPC request body Tauri parses is not zeroized either. Reading them needs code execution in the webview (blocked by the CSP and the closed command set) or a memory read of the app (R1) | accepted (ADR-0019 item 5, `docs/DESKTOP_IPC.md` §6): lists, details, events and errors carry no secret; copying, import reading and SSH signing happen in Rust; the UI must hide revealed values after 60 s, on blur and on lock (UX_GUIDELINES §8) |

## 6. Review log

| Date | Milestone | Reviewer | Result |
|---|---|---|---|
| 2026-10-25 | M0 | principal engineer (initial draft) | created |
| 2026-10-25 | M0 | crypto-reviewer (design review of CRYPTO_SPEC) | 3 high, 7 medium, 3 low. All resolved in the spec, or recorded as R7–R10 |
| 2026-10-26 | M1 | crypto-reviewer, test-engineer, gate review (implementation of `crates/core`) | 2 high, 7 medium, 11 low. Fixed with regression tests, except the upstream HMAC/HKDF stack residue (R11) and the items listed in PROGRESS "Open risks / findings" |
| 2026-10-26 | M1 | crypto-reviewer, test-engineer, gate review (follow-up round) | 3 medium, 6 low. All fixed with regression tests; residual timing note recorded as PROGRESS M1-R6 |
| 2026-10-26 | M2 | track reviews (model, store, generator, totp) | model: 2 high, 3 medium, 4 low; store: 5 medium, 6 low; generator: 2 medium, 5 low, 2 info; totp: 1 medium, 5 low. All fixed with regression tests or settled by ADR-0008 to ADR-0011; residuals R12 to R14 |
| 2026-10-26 | M2 | integration (session layer, 3-device simulation) | 1 medium: device-ID reuse check rejected moved items on devices that had edited them. Fixed with a regression test (ADR-0012 item 5) |
| 2026-10-26 | M2 | crypto-reviewer, sync-chaos, test-engineer, gate review (integration) | 2 high (one issue: concurrent rotations replaced a held vault key), 11 medium, 17 low. Fixed with regression tests (ADR-0013, ADR-0009 amendments, ADR-0012 item 7), except the merge-model gaps recorded as R15 and the items listed in PROGRESS "Open risks / findings" |
| 2026-10-27 | M2 | crypto-reviewer, sync-chaos, test-engineer, gate review (integration, second round) | 2 high (two sessions on one store lost an edit; the long sync simulation failed on an oracle error), 4 medium, 14 low. Fixed with regression tests (store lock, ADR-0010 item 10; per-entry delete-race repair and `VaultMeta::merge_and_repair`, ADR-0009 amendments; vault-key repair and error mapping, ADR-0012 amendments), except the in-flight rotation loss (R16) and the delivery-dependent repair (R15), accepted for M6 |
| 2026-10-27 | M2 | crypto-reviewer, sync-chaos, test-engineer, gate review (integration, third round) | 1 high (a reused device ID lost both writers' edits of different fields, also after the documented recovery), 9 medium, 14 low. Fixed with regression tests (fresh writer IDs and collision-safe merges, ADR-0015; oversized merges split, ADR-0016; the password generator's length floor, ADR-0011 amendment; store lock on the resolved path; zeroized OTP codes and labels, ADR-0008 amendments), the attachment re-seal on revocation specified for M6/M7 (ADR-0017, R18), the anchor-less replacement reported (R17), and the delivery-dependent delete-race repair re-confirmed as R15 with a characterization test |
| 2026-10-27 | M2 | crypto-reviewer, sync-chaos, test-engineer, gate review (integration, fourth round; the loop ended with 14 findings, fixed afterwards) | 1 high (an oversized split lost a committed edit depending on delivery order), 7 medium, 6 low. Fixed with regression tests (splits as a pure function of the merge with deterministic conflict-copy IDs, ADR-0016 amendment, which the new oversized case of the sync simulation confirmed; the site-rules entropy floor, ADR-0011 amendment; history trimmed by the key's stamp and collision write-backs kept visible, ADR-0009 amendments; metadata replacements reported, R17), the move-into-a-rotating-vault loss recorded under R16, and the limit of the ID-reuse detection pinned by a characterization test (ADR-0015 amendment) |
