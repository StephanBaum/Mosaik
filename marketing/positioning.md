# Positioning and messaging

> **Working title.** "Mosaic" is not the final product name and the domain is not decided (the repo is called "Mosaik"). Copy below says "Mosaic" in prose; replace it everywhere when the name is final. Status facts are taken from `docs/PROGRESS.md` as of 2026-11-07.

## 1. What we can truthfully say today

Built (M1 to M5, done): shared Rust crypto core, desktop app (Tauri; Windows, macOS, Linux), browser extension (Chromium and Firefox, development build, not in any store yet), passkeys (ES256), one-time codes (TOTP, HOTP, Steam Guard, QR scan), SSH agent with per-use approval, password and passphrase generator, offline security report, import (Enpass, Bitwarden, KeePass KDBX 4, 1Password 1PUX, CSV; passkeys from Bitwarden, KeePassXC and FIDO CXF), attachments, custom fields, Emergency Kit onboarding, German and English UI.
Nearly done (M6): sync engine, own-cloud-folder backend, self-hostable server, HTTP transport. Not released.
Planned: team sharing and emergency access (M6b), password rotation and breach check (M7), CLI (M8), hardening, audit prep, signed releases, landing page (M9), mobile (M10).
Not stated anywhere in the docs: the license, whether the project is "open source", the name, price, and an audit date.

## 2. Ideal customer profile

### Primary: the privacy-minded individual who is leaving or doubting their manager
- Tech-savvy: uses a terminal, SSH keys, self-hosted services. Reads Hacker News, r/privacy, r/selfhosted, heise.
- Current tool is one of: LastPass (burned or uneasy since the 2022 breach), 1Password (price rises, subscription fatigue), Bitwarden (fine, but wants more polish or a different trust model), KeePass/KeePassXC (loves control, hates the sync and the browser story).
- Cares about: who can read my data, where it is stored, being able to read the spec, no telemetry, not being locked in.
- Buying trigger: a breach story, a price increase, a new device, a passkey rollout.
- Objection to expect: "pre-audit, new, one maintainer" (see launch plan risks).

### Secondary: small businesses and teams (5 to 50 people)
- Wants shared vaults, offboarding, and data that stays in their own cloud (Dropbox Business team folders, Nextcloud) or on a server they host.
- Honest position: team sharing is **coming with M6b, not shipped**. Today they can evaluate the product for individuals and join a waiting list.
- Do not sell this segment anything that does not exist. Treat it as an early-interest list.

## 3. Problem, alternatives, why they switch

| | |
|---|---|
| Problem | Your passwords are the keys to everything. You have to trust a vendor, a sync service and a pile of closed code, and few vendors let you check. |
| Alternatives | 1Password (polished, paid, mature); Bitwarden (open code, audited, cheap); Proton Pass (privacy brand, free tier); KeePassXC (free, local, DIY sync); the browser's built-in manager. |
| Why they switch | Trust shock (LastPass 2022), price rises (1Password and Bitwarden both raised prices in early 2026, sources in `comparison.md`), wish for choice of where the data lives, wish to read the actual design. |
| Why they do not switch | Fear of migration (answer: import from all five formats), fear of new software (answer: honesty, spec, threat model, audit plan), missing mobile (answer: roadmap, say so plainly). |

## 4. Value proposition

Mosaic is a password manager built so you do not have to take our word for it. Your vault is locked with your master password and a Secret Key that never leaves your devices, and it can sync through a cloud folder you already own. The cryptography, the threat model and every field the server can see are written down and published.

**One-liner options**
1. **Your vault, your cloud, your keys.** A zero-knowledge password manager that syncs through the folder you already trust.
2. **The password manager that shows its work.** Published crypto spec, published threat model, no telemetry.
3. **Two secrets. Zero trust required.** Master password plus a Secret Key that never leaves your devices, synced through your own cloud folder or ours.

## 5. Messaging pillars and proof points

| Pillar | Message | Proof (real, from the docs) |
|---|---|---|
| 1. Zero knowledge, for real | Even if the server is stolen, there is nothing to guess offline. | Master password plus 128-bit Secret Key (CRYPTO_SPEC §1, §5); Argon2id; per-item XChaCha20-Poly1305; OPAQUE login so the server never sees anything password-derived (§9); no backdoor and no recovery without the Emergency Kit (README). |
| 2. Read it, do not trust it | We publish how it works and where it is weak. | CRYPTO_SPEC with only standard primitives (§2); THREAT_MODEL with a residual-risk register (R1 to R68); the server's visible fields listed in §12; ADR log. |
| 3. Your data, your place | Sync through Dropbox, iCloud, OneDrive or Nextcloud, or use hosted sync. | Folder backend (ADR-0043, M6); self-hostable server (operator guide on the M6 server branch). Label as "arriving with M6". |
| 4. Careful by default | A browser extension that cannot be tricked into filling the wrong site. | No fill on page load; exact-origin match; no cross-origin iframe fills; one credential per content script; HTTP refused (BRIEF §4, EXTENSION_SECURITY.md, hostile test site). |
| 5. Modern basics | Passkeys, one-time codes and SSH keys in one place. | ES256 passkeys (ADR-0039), TOTP with QR scan, SSH agent with per-use approval. |
| 6. No tracking | No analytics, no telemetry, no third-party anything. | Project rule; CSP and fonts bundled locally, nothing loads from a CDN (ADR on fonts). |

## 6. Tone of voice

Calm, precise, a little dry. Written by engineers for engineers who are tired of hype.
- State the limit next to the claim ("pre-audit", "planned for M6b").
- Prefer a number or a document link to an adjective.
- No fear marketing about breaches; cite facts once, move on.
- No "military-grade", no superlatives, no emojis.
- German: du-form, plain sentences, keep established words (Passwort, Tresor, Synchronisierung), avoid needless English.

## 7. Claims we may and may not make

| May say | Only with this qualifier | May not say |
|---|---|---|
| Zero-knowledge design; server stores ciphertext and the listed metadata | "by design, as specified in CRYPTO_SPEC; not yet independently audited" | "audited", "bank-grade", "military-grade", "unhackable", "100% secure", "uncrackable" |
| Uses standard, published primitives (Argon2id, XChaCha20-Poly1305, HKDF, OPAQUE, HPKE) | "we compose, we do not invent" | "unbreakable encryption" |
| Crypto spec and threat model are public | link them | "open source" or any license name (undecided) |
| No analytics, no telemetry | project rule | "we never see anything" (the server sees listed metadata, e.g. account email) |
| Desktop, extension, passkeys, TOTP, SSH agent, import work today | "development build", extension not yet in a store | that sync, teams, mobile, CLI or rotation exist today |
| Sync through your own cloud folder | "arriving with M6" until released | that the cloud provider cannot see file sizes, counts and timing (it can, §12.1) |
| Lost password and Emergency Kit means data is unrecoverable | plain statement | "we can restore your account" |
| An external audit is planned | "we intend to commission one" (owner to confirm) | any date, auditor name or result |
| Better than competitors at X | only where the comparison doc supports it, with source | that Mosaic is more secure than 1Password, Bitwarden etc. |
