# Build Brief: Mosaic — Zero-Knowledge Password Manager

> The project owner's original brief, kept as the source of requirements.
> The working name "Tessera" was replaced by the product name "Mosaic" (ADR-0001).

---

## 0. Your role and operating mode

You are a principal engineer leading a small elite team (that you simulate with subagents) building a production-grade, security-critical password manager. Security and correctness come before speed; usability comes immediately after security.

Work autonomously through the milestones below. Do not stop to ask questions unless a decision is irreversible AND not covered by this brief; otherwise make the most conservative security choice, record it in `docs/DECISIONS.md` (ADR format: context, decision, alternatives, consequences), and continue.

Rules that always apply:

- **Never invent cryptography.** Only use well-reviewed, maintained libraries (RustCrypto, `opaque-ke`, `argon2`, `chacha20poly1305`, `zeroize`, `webauthn-rs`/`passkey-rs`-class crates). No custom primitives, no custom protocols where a standard exists.
- **The repo contains only real deliverables.** Scratch files, experiments, throwaway scripts and temporary tooling live outside the repo (e.g. `/tmp`) and are deleted after use. No helper/toolkit folders that become obsolete.
- **Every milestone ends green**: build, lint, tests, security checks, docs updated, one clean commit (Conventional Commits). Never leave the repo broken between milestones.
- **Keep a running `docs/PROGRESS.md`**: what's done, what's next, open risks. If the session ends, the next session resumes from it.
- If you are blocked on something external (store accounts, signing certificates, paid APIs), stub it behind an interface, document it in `docs/PROGRESS.md` under "Needs owner", and move on.

---

## 1. Product vision

A password manager that beats Enpass on sync and connectivity and matches 1Password/Bitwarden on security, with a calm, fast, keyboard-first UX. Zero-knowledge: the server and anyone who steals it learn nothing useful.

**Platforms (in build order):** Rust core → desktop app (Windows, macOS, Linux) → browser extension (Chrome, Edge, Firefox; Safari later) → sync server → CLI → mobile (iOS, Android).

**Languages of the UI:** German and English from day one (i18n infrastructure, no hard-coded strings).

---

## 2. Fixed architecture (do not deviate without an ADR)

| Layer | Technology |
|---|---|
| Crypto + vault core | Rust workspace, `#![forbid(unsafe_code)]` where possible, compiled native and to WASM |
| Desktop | Tauri v2, React + TypeScript (strict), Vite |
| Browser extension | Manifest V3, TypeScript, React for the popup only |
| Extension ↔ desktop | Native messaging; the desktop app holds the unlocked vault |
| Sync server | Rust (axum), PostgreSQL, WebSockets for push, Docker Compose for self-hosting |
| CLI | Rust, shares the core crate |
| Mobile (final milestone) | Native autofill/passkey providers (iOS Credential Provider, Android Credential Manager / Autofill Service) around the shared Rust core via UniFFI |

Suggested repo layout:

```
crates/core        # crypto, vault model, item types, sync protocol types
crates/totp        # RFC 4226/6238, Steam Guard
crates/generator   # passwords, passphrases, site rules
crates/passkey     # WebAuthn authenticator logic
crates/import      # Enpass JSON, Bitwarden JSON, KeePass KDBX, 1Password 1PUX, CSV
crates/server      # sync server
crates/cli
apps/desktop       # Tauri app
apps/extension
apps/mobile        # last
docs/
```

---

## 3. Security design (normative)

Write `docs/THREAT_MODEL.md` (STRIDE per component, attacker profiles: stolen server DB, malicious website, malicious extension on same browser, local malware without root, lost/stolen unlocked/locked device, compromised build dependency) and `docs/CRYPTO_SPEC.md` BEFORE writing crypto code. Implementation must match the spec.

**Key hierarchy**
- Master password + 128-bit **Secret Key** (generated on device, shown once in an Emergency Kit PDF, never sent to the server) → Argon2id (≥ 256 MiB memory where the device allows, calibrated to ~1 s; parameters stored per account and upgradable) → HKDF → key-encryption key.
- Random 256-bit **vault key** per vault, wrapped by the KEK. Changing the master password re-wraps keys, never re-encrypts all items.
- Every item encrypted individually with XChaCha20-Poly1305, random nonce, item ID + vault ID + schema version as associated data.
- Sharing: per-recipient X25519 public-key wrapping of vault keys.
- Secrets in memory: `zeroize`, minimal lifetime, never logged, never in panic messages, never in crash reports.

**Authentication**
- OPAQUE (`opaque-ke`) for account login: the server never sees anything password-derived it could brute-force offline.
- Optional hardware-key unlock via WebAuthn PRF extension; quick unlock via OS secure hardware (Secure Enclave / TPM / Windows Hello / Touch ID).
- Account 2FA for the sync account (TOTP + WebAuthn).

**Transport & server**
- TLS 1.3 only; server stores ciphertext blobs, item versions and minimal metadata. Document every metadata field the server can see and why.
- Rate limiting, audit log (no secrets), no third-party analytics anywhere.

**Supply chain**
- Pinned dependencies, lockfiles committed, `cargo-deny` + `cargo-audit` + `npm audit` in CI, minimal dependency count (justify each new dependency in the PR/commit message).
- Signed releases, reproducible-build instructions, SBOM generation.

---

## 4. Browser extension security (highest-risk component)

Implement all of these and prove each with an automated test against a local hostile test site:

1. **No autofill on page load, ever.** Filling requires a trusted user gesture (click in extension UI, keyboard shortcut, or explicit inline-button click verified as trusted).
2. **Exact origin matching** by default (scheme + host + port); optional eTLD+1 matching per item, off by default. Punycode/homoglyph detection with a warning.
3. **No filling into cross-origin iframes**; same-origin iframes only after explicit confirmation.
4. **Clickjacking resistance**: no injected UI that a page can overlay or restyle; inline suggestions in a closed shadow root with visibility/occlusion checks, or no inline UI at all (toolbar popup only) — choose and document via ADR. Defeat DOM-based clickjacking (opacity tricks, overlays, pointer-events tricks).
5. Content scripts receive **only the single credential being filled**, never the vault or item lists.
6. Minimal permissions; host permissions requested at runtime where possible.
7. Native-messaging channel authenticated and bound to the installed desktop app; reject unknown origins/extension IDs.
8. Detect and refuse HTTP (non-TLS) fills unless the user explicitly overrides per item.

Write `docs/EXTENSION_SECURITY.md` mapping each attack to its mitigation and test.

---

## 5. Feature set (all in scope)

**Vault & items:** logins, passkeys, TOTP, secure notes, credit cards, identities, SSH keys, API keys, Wi-Fi, documents/attachments (encrypted, size-limited), custom fields, tags, favorites, multiple vaults, password history per item, trash with restore.

**Password generator:** CSPRNG only, rejection sampling (no modulo bias); random characters with configurable classes; passphrases from the EFF large wordlist; per-site rules (consume the `passwordrules` syntax and Apple's open-source Password Manager Resources quirks data, MIT — respect its license); live entropy display; generator available in desktop, extension (on password fields) and CLI.

**2FA / TOTP:** RFC 6238 with official test vectors; SHA1/256/512, 6/8 digits, custom periods; HOTP; Steam Guard; QR scanning from screen and image; `otpauth://` import/export; autofill of the code after the password. Per-item option to store TOTP secrets in a **separate vault that requires hardware-key unlock** (explain the trade-off in the UI in one sentence).

**Passkeys:** the extension acts as a WebAuthn authenticator (intercept `navigator.credentials.create/get` safely, fall back to the platform authenticator when the user chooses); passkeys stored encrypted and synced; desktop and mobile OS passkey-provider integration where the OS allows; import/export via the FIDO Alliance Credential Exchange format (CXF/CXP) where available.

**Password rotation (three tiers):**
1. *Semi-automatic (all sites):* generate → open the site's change-password page via `/.well-known/change-password` (fallback: stored URL) → fill old/new fields → commit the new password to the vault only after the user confirms success; old password kept in history; rollback button.
2. *Automatic connectors (plugin system):* a sandboxed connector interface with a typed manifest (name, scopes, required inputs) for services with real APIs. Ship reference connectors: SSH key rotation on your own hosts, PostgreSQL/MySQL user passwords, a generic HTTP/webhook connector. Design so AWS IAM, Google Workspace and Microsoft Entra connectors can be added later. Scheduled rotation with retry, audit log, and failure notifications.
3. *Triggered:* breach check via Have I Been Pwned k-anonymity range API (only hash prefixes leave the device), weak/reused/old password detection, "rotate now" queue, and "upgrade to passkey" suggestions where the site supports passkeys.

**Sync:** per-item encrypted records with version vectors; offline-first local store (SQLite via the core); WebSocket push with exponential-backoff reconnect; deterministic conflict resolution (per-field last-writer-wins with conflict copies surfaced to the user — never silently lose data); full end-to-end tests with 3 simulated devices editing offline and reconciling. Self-hostable server via one `docker compose up`.

**Security dashboard:** weak, reused, breached, old, missing-2FA (use 2fa.directory data where license permits), HTTP-only sites; one-click actions.

**Sharing & recovery:** share items/vaults with other users (public-key wrapped), emergency access with waiting period, account recovery only via Emergency Kit (no backdoor — document this clearly).

**Import/export:** Enpass JSON (priority), Bitwarden JSON, KeePass KDBX 4, 1Password 1PUX, generic CSV with column mapping; encrypted export; import preview with dedupe.

**SSH agent:** desktop app can serve SSH keys via an agent with per-use approval.

**CLI:** unlock, search, get, generate, TOTP, inject secrets into env/commands, JSON output.

---

## 6. UX requirements ("great usability" is a deliverable, not a hope)

- Onboarding in under 2 minutes: create account → Secret Key + Emergency Kit PDF (print/save, with a verification step) → import from Enpass → install extension.
- Keyboard-first: global shortcut opens a command palette; every action reachable without a mouse.
- Unlock in one gesture after first setup (biometrics / hardware key); auto-lock on idle, sleep, screen lock (configurable).
- Clipboard auto-clear (default 30 s), excluded from clipboard history where the OS supports it.
- Calm visual design: light/dark/system themes, consistent spacing scale, design tokens, no clutter; progressive disclosure for advanced settings.
- Accessibility: WCAG 2.2 AA, full screen-reader labels, focus states, reduced-motion support.
- Every error message says what happened and what to do next; no stack traces in UI.
- Write `docs/UX_GUIDELINES.md` and follow it. A UX-reviewer subagent checks every UI milestone against it.

---

## 7. Workflow

**Per milestone:**
1. *Plan* (plan mode): write the milestone plan into `docs/PROGRESS.md` — scope, files, tests, risks.
2. *Specify*: update the relevant spec doc first if behavior or crypto changes.
3. *Implement* with tests written alongside (or first for crypto/protocol code).
4. *Verify*: run the full gate (section 8).
5. *Adversarial review* by subagents (see below); fix every finding or record it as accepted risk with justification.
6. *Commit* with a clear message; update `docs/PROGRESS.md`.

**Subagents (create them as project subagents):**
- `crypto-reviewer` — checks code against `CRYPTO_SPEC.md`, nonce handling, key lifetimes, zeroization, side channels, error-handling leaks.
- `extension-attacker` — tries to break the extension: writes new hostile test pages, attempts clickjacking, iframe, lookalike-domain and message-spoofing attacks.
- `sync-chaos` — writes adversarial sync scenarios (clock skew, partitions, replay, reordering, duplicate delivery).
- `ux-reviewer` — reviews UI against `UX_GUIDELINES.md` and accessibility rules, with screenshots via Playwright.
- `test-engineer` — finds untested paths, adds property tests and fuzz targets.

**Parallelism:** once the core crate API is frozen (end of M2), develop desktop, extension and server in parallel git worktrees; merge only through the full gate.

**Project memory:** maintain a concise `CLAUDE.md` with build/test commands, architecture rules and the non-negotiables from this brief.

---

## 8. Quality gate (must pass at every milestone)

- Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, `cargo audit`.
- TypeScript: strict mode, ESLint with zero warnings, type-check, unit tests (Vitest).
- Crypto: known-answer tests (RFC 6238/4226 vectors, Argon2 vectors, XChaCha20-Poly1305 vectors), property tests (`proptest`) for encrypt/decrypt round-trips and tamper detection.
- Fuzzing: `cargo-fuzz` targets for every parser (imports, item deserialization, sync messages, `otpauth://` URIs, native-messaging frames); run each for a bounded time per milestone.
- End-to-end: Playwright tests for desktop UI and extension, including the hostile test site suite.
- Sync: multi-device simulation tests.
- CI: GitHub Actions workflow running all of the above on Windows, macOS, Linux.
- Coverage target: ≥ 90 % for `crates/core`, `totp`, `generator`, `passkey`.

---

## 9. Milestones

- **M0 Foundations:** repo layout, CI, `CLAUDE.md`, subagents, `THREAT_MODEL.md`, `CRYPTO_SPEC.md`, `UX_GUIDELINES.md`, ADR template.
- **M1 Crypto core:** key hierarchy, Secret Key, Argon2id calibration, item encryption, zeroization, test vectors, fuzz targets.
- **M2 Vault model + local store:** all item types, multiple vaults, history, trash, attachments; generator and TOTP crates; freeze core API.
- **M3 Desktop app:** onboarding with Emergency Kit, unlock (password, biometrics, hardware key), vault UI, command palette, generator, TOTP, security dashboard (offline parts), Enpass + other importers, SSH agent.
- **M4 Browser extension:** native messaging, fill with all section 4 protections, generator on password fields, TOTP fill, save/update prompts, hostile test suite.
- **M5 Passkeys:** authenticator in extension, storage, desktop OS provider integration, CXF import/export.
- **M6 Sync server:** OPAQUE auth, account 2FA, encrypted item sync, WebSocket push, conflict handling, Docker Compose, multi-device tests, sharing, emergency access.
- **M7 Rotation:** semi-automatic flow, connector plugin system with reference connectors, scheduler, HIBP integration, rotate-now queue.
- **M8 CLI.**
- **M9 Hardening:** full threat-model review by all subagents, extended fuzzing, dependency minimization, release signing, SBOM, `SECURITY.md` with vulnerability disclosure policy, `docs/AUDIT_PREP.md` (scope and materials for an external auditor).
- **M10 Mobile:** iOS and Android apps with native autofill and passkey providers on the shared core.

---

## 10. Definition of done (whole project)

- All milestones pass the gate; `docs/PROGRESS.md` shows no open critical or high findings.
- A new user can install desktop + extension, import from Enpass, and log in to a site with autofill, TOTP and a passkey in under 5 minutes, without reading docs.
- Three devices sync edits made offline without data loss.
- The project is ready for an external security audit — and is labeled pre-audit in the README until that audit is done. Do not claim it is audited or "bank-grade" anywhere.
