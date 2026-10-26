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
| E | Webview reaches the Tauri API surface | isolation pattern, minimal commands, typed inputs validated in Rust, M3 |
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
| Removed member keeps old data | VK rotation on removal (§8.3). **Residual:** data already downloaded stays exposed |
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

## 6. Review log

| Date | Milestone | Reviewer | Result |
|---|---|---|---|
| 2026-10-25 | M0 | principal engineer (initial draft) | created |
| 2026-10-25 | M0 | crypto-reviewer (design review of CRYPTO_SPEC) | 3 high, 7 medium, 3 low. All resolved in the spec, or recorded as R7–R10 |
| 2026-10-26 | M1 | crypto-reviewer, test-engineer, gate review (implementation of `crates/core`) | 2 high, 7 medium, 11 low. Fixed with regression tests, except the upstream HMAC/HKDF stack residue (R11) and the items listed in PROGRESS "Open risks / findings" |
| 2026-10-26 | M1 | crypto-reviewer, test-engineer, gate review (follow-up round) | 3 medium, 6 low. All fixed with regression tests; residual timing note recorded as PROGRESS M1-R6 |
