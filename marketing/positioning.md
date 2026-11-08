# Positioning and messaging

> **Working title.** "Mosaic" is not the final product name and the domain is not decided (the repo is called "Mosaik"). Copy below says "Mosaic" in prose; replace it everywhere when the name is final.
>
> **Framing: launch day of the finished product.** Owner direction (2026-11-07): write for the day every milestone through M10 has shipped, sound like a category leader, lead with what sets us apart. Everything here is planned in `docs/BRIEF.md` and `docs/PROGRESS.md`; nothing outside those docs is promised. Before publication, every feature claim is checked against `docs/PROGRESS.md` on that date, and anything not shipped yet is cut or labeled.

## 1. What the finished product is

| Area | What ships (milestone) |
|---|---|
| Core | One Rust crypto core for every app: master password plus a 128-bit Secret Key, Argon2id, XChaCha20-Poly1305 per item, OPAQUE sign-in (M1, M2, M6) |
| Desktop | Windows, macOS, Linux. Command palette, quick unlock (Windows Hello, Touch ID), security-key unlock, auto-lock, clipboard auto-clear (M3) |
| Items | Logins, passkeys, one-time codes, notes, cards, identities, SSH keys, API keys, Wi-Fi, encrypted attachments, custom fields, tags, multiple vaults, history, trash (M2, M3) |
| Browser | Chrome, Edge, Firefox. Fill only after a click, exact-origin match, no cross-origin frame fills, look-alike warnings, one credential per page, HTTP refused (M4) |
| Passkeys | Authenticator in the extension that catches `navigator.credentials`; stored encrypted, synced; CXF import (M5); native passkey providers on iOS and Android (M10) |
| 2FA codes | TOTP, HOTP, Steam Guard, QR scan from screen or image, autofill after the password, optional vault that needs a hardware key (M2 to M4) |
| SSH | Built-in SSH agent, every use approved (M3) |
| Import | Enpass, Bitwarden, KeePass KDBX 4, 1Password 1PUX, CSV; passkeys from Bitwarden, KeePassXC and the FIDO CXF format; preview with dedupe (M3, M5) |
| Sync | Your own cloud folder (Dropbox, iCloud Drive, OneDrive, Nextcloud, a NAS, a Dropbox Business team folder) or the hosted Mosaic server, or your own server with one Docker Compose command. Offline-first, conflict copies instead of lost edits (M6) |
| Lying-server detection | Rollback and freeze detection on both backends; on hosted sync, devices compare what the server showed each of them and raise "server inconsistent" (M6) |
| Sharing | Shared vaults sealed per person, safety codes to verify who you share with, revoke re-keys the vault, emergency access with a waiting period, team vaults in a team folder or on the server (M6b) |
| Rotation | Change passwords on any site semi-automatically; connectors that rotate SSH keys on your hosts, PostgreSQL and MySQL passwords and webhooks on a schedule; breach check that sends only a hash prefix; "rotate now" queue; passkey upgrade hints; full account re-key (M7) |
| Security report | Weak, reused, breached, old, HTTP-only, missing 2FA (M3, M7) |
| CLI | `mosaic`: unlock, search, get, generate, TOTP, inject secrets into env and commands, JSON output (M8) |
| Releases | Signed, with checksums, SBOM and reproducible-build instructions; `SECURITY.md` with a disclosure policy; WebAuthn and TOTP 2FA for the sync account (M9) |
| Mobile | iOS and Android with native autofill and passkey providers on the same Rust core (M10) |
| UI | German and English, light and dark, keyboard-first, WCAG 2.2 AA |

**Still not decided anywhere in the docs:** the name, the license and any "open source" claim, prices, and an audit date. The copy keeps these as placeholders.

## 2. Ideal customer profile

### Primary: people who want control and are tired of blind trust
- Tech-savvy: terminal, SSH keys, self-hosted services. Reads Hacker News, r/privacy, r/selfhosted, heise.
- Comes from LastPass (breach), 1Password (price rises, subscription only), Bitwarden (wants more polish or a stricter trust model), KeePassXC or Enpass (loves owning the file, hates the sync and browser story).
- Cares about: who can read my data, where it lives, whether the vendor's server could fool me, no telemetry, no lock-in.
- Buying trigger: a breach story, a price increase, a new phone, a passkey rollout, a team that outgrew a shared spreadsheet.

### Secondary: small businesses and teams (5 to 50 people)
- Want shared vaults, offboarding, emergency access, and data in a place they already run: a Dropbox Business team folder, Nextcloud, or their own server.
- The pitch: same end-to-end encryption, your storage, no new vendor cloud to vet.

## 3. Problem, alternatives, why they switch

| | |
|---|---|
| Problem | Every major password manager asks you to trust its server: to store your vault, to hand out the right keys, to show you the latest data. In 2026 ETH Zurich researchers showed 27 attacks across four big managers by a server that turns hostile [C1]. |
| Alternatives | 1Password (polished, paid, mature); Bitwarden (open code, audited, cheap); Proton Pass (privacy brand, free tier); KeePassXC and Enpass (your own file, DIY or folder sync); the browser's built-in manager. |
| Why they switch | Trust shock, price rises, the wish to choose where data lives, the wish to read the design, and now: a manager that assumes its own server may lie. |
| Why they hesitate | Migration (answer: import from five formats plus CXF passkeys); a new name (answer: published spec, threat model, pre-audit honesty, audit plan); "will it last" (answer: works with your own folder, so no server can take your data hostage). |

## 4. Value proposition

Mosaic is the password manager that does not ask you to trust its server, or anyone's. Every item is encrypted on your device before it moves. You choose where it lives: your own cloud folder, your team's folder, our hosted server or yours. And if that storage ever rolls back, freezes or shows your devices different data, Mosaic notices and tells you.

**One-liners**
1. **Trust no server. Not even ours.** The password manager that encrypts everything on your device and catches a server that lies. *(recommended)*
2. **Your cloud or ours. Readable only by you.** End-to-end encrypted sync through any folder you own, or our hosted server.
3. **Everything a password manager should be. Nothing you have to take on faith.** Passwords, passkeys, 2FA and SSH keys, on every device, with a published spec for all of it.

## 5. The "only Mosaic" list (verified)

Compared set: 1Password, Bitwarden, Proton Pass, KeePassXC and Enpass (see `comparison.md`). "Only" means **only within this set**, as of 2026-11; there are many smaller managers we did not check. Public copy says "unlike other major password managers" or "no other manager we compared", never "no one in the world".

| # | Claim | Status | Evidence |
|---|---|---|---|
| O1 | **Your folder or our server, same end-to-end protocol.** Sync through your own cloud folder (including a Dropbox Business team folder), or the hosted server, or a server you run, and switch between them. | **Only Mosaic** in the set | 1Password, Bitwarden, Proton Pass: vendor-hosted sync (Bitwarden can be self-hosted as a server, not as a cloud folder) [C2][C3][C4]. KeePassXC and Enpass: your own storage only, no hosted service [C5][C6]. Mosaic: ADR-0043 (one engine, two transports). |
| O2 | **Catches a lying server.** Detects a server or folder that rolls back or freezes your data; on hosted sync, devices also compare what the server showed each of them and raise "server inconsistent" on a fork. Findings stay until you acknowledge them, and anything the server dropped is uploaded again. | **Only Mosaic documents this** in the set; re-check vendor docs before launch | None of the compared vendors' security documentation describes rollback, freeze or fork detection of vault data (checked 2026-11); 1Password's answer to the ETH paper does not mention it [C2]. KDBX and Enpass files carry no anti-rollback counter (n/v for Enpass internals). ETH Zurich 2026 shows the malicious-server model is where today's managers are weakest [C1]. Mosaic: CRYPTO_SPEC §10, THREAT_MODEL R56, R68. |
| O3 | **OPAQUE sign-in on top of two secrets.** The server never receives anything password-derived it could guess against offline; without the Secret Key a stolen database is not enough. | **Only Mosaic uses OPAQUE** in the set. Two secrets alone is **not** unique: 1Password has a Secret Key too | 1Password: SRP plus Secret Key [C7]. Proton: SRP [C8]. Bitwarden: client-side hash sent to the server [C9]. KeePassXC, Enpass: no account. OPAQUE is the IETF/IRTF successor to SRP that resists pre-computation [C10]. Mosaic: CRYPTO_SPEC §5, §9. |
| O4 | **The combination:** own folder or hosted, lying-server detection, OPAQUE plus two secrets, exact-origin fill by default, zero telemetry, SSH agent, passkeys, 2FA codes, rotation connectors, CLI and mobile apps, all in one product. | **Only Mosaic** in the set, because O1 to O3 are | Follows from O1 to O3 and §6. |

## 6. "Unlike most password managers" (true, but not unique)

| Claim | Who else does it, fairly stated | Evidence |
|---|---|---|
| **Exact-origin matching by default.** `login.bank.example` never fills on `evil.bank.example`; eTLD+1 only per item, off by default | 1Password matches the domain by default, exact host is a per-item option [C11]; Bitwarden defaults to base domain [C12]. Proton Pass, KeePassXC n/v | BRIEF §4, EXTENSION_SECURITY.md |
| **No fill without your click, and no setting to change that**; no cross-origin iframe fills; one credential per page; HTTP refused | Bitwarden offers opt-in fill on page load (off by default) [C12]; others n/v | BRIEF §4, hostile test suite |
| **Zero telemetry, written into the spec.** No analytics, no crash reports, no third-party requests | 1Password: opt-in for individuals, on by default for business accounts [C13]. KeePassXC: none known (n/v) | CLAUDE.md rule, CRYPTO_SPEC §12 |
| **Every field the server sees, listed** in a normative spec, plus what your cloud provider sees | 1Password and Bitwarden publish security white papers [C7][C9]; a field-by-field list n/v | CRYPTO_SPEC §12, §12.1 |
| **A threat model with a residual-risk register** (R1 onwards), published | n/v for others | THREAT_MODEL.md |
| **Safety codes for sharing**: verify a contact's key by QR or aloud; a key change warns you | Bitwarden shows fingerprint phrases (n/v scope). 1Password states a malicious server could hand out dishonest public keys [C2]. ETH 2026 found 5 attacks via unauthenticated public keys across vendors [C1] | CRYPTO_SPEC §8.2 |
| **A 2FA vault that needs a hardware key** | n/v for others | BRIEF §5 |
| **Rotation connectors** for SSH keys on your hosts, PostgreSQL and MySQL passwords and webhooks, on a schedule | n/v for consumer managers; enterprise PAM tools do this | BRIEF §5 (M7) |
| **SSH agent + passkeys + 2FA codes in one app** | **Not unique:** 1Password and Bitwarden have all three, KeePassXC has an SSH agent, Proton is rolling one out [C14][C15]. Use as a parity point, never as "only" | BRIEF §5 |

## 7. Messaging pillars

| Pillar | Headline | Message | Proof |
|---|---|---|---|
| 1. Your storage, your call | **Your cloud or ours.** | Dropbox, iCloud, OneDrive, Nextcloud, a team folder, our server or yours. Same encryption everywhere. | O1; ADR-0043, ADR-0044; `deploy/compose.yaml` |
| 2. Catches a lying server | **The server can lie. Your devices notice.** | Rollback, freeze and fork detection with a clear warning. | O2; CRYPTO_SPEC §10 |
| 3. Nothing to crack on our side | **A stolen server holds noise.** | Two secrets, OPAQUE, every item sealed on its own. | O3; CRYPTO_SPEC §5, §7, §9 |
| 4. A browser extension that refuses to be tricked | **No click, no fill.** | Exact origin, no cross-origin frames, look-alike warnings, hostile test site in CI. | §6; EXTENSION_SECURITY.md |
| 5. Everything, everywhere | **Every secret. Every device.** | Passwords, passkeys, 2FA, SSH keys, files, CLI, desktop, browser, iOS, Android. | §1 |
| 6. Shows its work | **Read it. Don't trust it.** | Spec, threat model, decision log, signed reproducible releases, zero telemetry. | CRYPTO_SPEC, THREAT_MODEL, DECISIONS, M9 |

## 8. Tone of voice

Confident, fast, precise. A category leader that can back every line.
- **Short headlines.** Two to six words, a full stop, often a pair: "Your cloud or ours." "No click, no fill."
- **Specific beats vague.** "128-bit Secret Key" beats "advanced security"; "rolls back, freezes or forks" beats "tampering".
- **Strong, never false.** Every claim maps to a row in §5, §6 or §9. If it does not, it does not ship.
- **No competitor bashing on the landing page.** Name the difference ("unlike other major managers"), name competitors only on the comparison page, with sources.
- **No fear marketing,** no emojis, no exclamation marks, no "revolutionary".
- **German:** du, short sentences, active verbs. Product terms stay as in the app (Secret Key, Emergency Kit, Passkey, Login); everything else in plain German (Tresor, Synchronisierung, Einmalcode, Ordner).

## 9. Claims we may and may not make

| May say | Only with this qualifier | May never say |
|---|---|---|
| End-to-end encrypted, zero knowledge; the server stores ciphertext and the listed metadata | link CRYPTO_SPEC §12 | "audited" (until a published report exists), "bank-grade", "military-grade", "unhackable", "uncrackable", "100 % secure", "unbreakable" |
| "Only Mosaic" for O1 to O4 | "of the major managers we compared", with the comparison page linked | "only" for anything in §6, "no one in the world" |
| Detects a server that rolls back, freezes or forks your vault | detection, not prevention; fork detection needs two devices that reach the same state (R56); the server can still withhold data, it just cannot do it unnoticed for long | "the server cannot attack you", "tamper-proof" |
| Trust no server, not even ours | as a design stance backed by §5; emergency-access waiting periods and read-only roles are still enforced by the server (CRYPTO_SPEC §8) | "you never need to trust anyone" |
| Zero telemetry | the breach check sends a 5-character hash prefix to Have I Been Pwned when you use it; folder sync talks to nothing but your folder | "we never see anything" (the hosted server sees the §12 fields, e.g. your email) |
| Your cloud provider sees only encrypted files | it sees file names, sizes, counts and times, and can delete or roll back files (which Mosaic detects) (§12.1) | "your provider sees nothing" |
| Standard, published primitives | "we compose, we never invent" | "our own encryption", "unbreakable encryption" |
| Not yet independently audited; everything an auditor needs is public | until the report is out | any audit date, auditor or result before it exists |
| Lost master password and Emergency Kit means nobody can recover the data | plain statement, always | "we can restore your account" |
| Open source | only after the owner decides the license | anything about the license before that |

## Sources for §3, §5 and §6 (accessed 2026-11)

- C1. ETH Zurich, Scarlata, Torrisi, Backendal, Paterson, "Zero Knowledge (About) Encryption", IACR ePrint 2026/058: https://eprint.iacr.org/2026/058 ; coverage with per-vendor counts: https://infosecurity-magazine.com/news/vulnerabilities-password-managers
- C2. 1Password Security Design white paper, release 0.5.2 (2026-01-22): https://agilebits.github.io/security-design/1Password_Security_Design.pdf ; 1Password's answer to the ETH paper (says a malicious server could hand out dishonest public keys; says nothing about rollback or fork detection): https://1password.com/blog/eth-zurich-zero-knowledge-malicious-server-review
- C3. Bitwarden self-hosting: https://bitwarden.com/help/install-on-premise-linux/ (URL to verify)
- C4. Proton Pass pricing and features: https://proton.me/pass/pricing
- C5. KeePassXC: https://github.com/keepassxreboot/keepassxc/
- C6. Enpass sync options: https://www.enpass.io/docs/manual-desktop/sync.html and https://www.helpnetsecurity.com/2026/08/10/product-showcase-enpass-password-manager/
- C7. 1Password SRP and Secret Key: white paper [C2]; 1Password's response in [C1] coverage
- C8. Proton SRP: https://proton.me/blog/encrypted-email-authentication
- C9. Bitwarden security white paper: https://bitwarden.com/help/bitwarden-security-white-paper/
- C10. OPAQUE (IRTF CFRG), with its comparison to SRP: https://datatracker.ietf.org/doc/draft-irtf-cfrg-opaque/
- C11. 1Password autofill behavior: https://support.1password.com/autofill-behavior/
- C12. Bitwarden autofill and URI matching: https://bitwarden.com/help/auto-fill-browser/ and https://bitwarden.com/help/uri-match-detection/
- C13. 1Password telemetry: https://support.1password.com/telemetry and https://support.1password.com/telemetry-teams/
- C14. Bitwarden SSH agent: https://bitwarden.com/help/ssh-agent/
- C15. Proton Pass roadmap (SSH agent rollout): https://proton.me/blog/pass-roadmap-spring-summer-2026
