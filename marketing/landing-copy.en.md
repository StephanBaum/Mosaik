# Landing page copy (English)

> **Working title.** "{NAME}" stands for the product name (working title "Mosaic") and "{DOMAIN}" for the domain. Neither is final. The license is undecided, so the copy never names one and avoids "open source". Items marked **(coming)** are planned and must keep that label until shipped. Facts as of 2026-11-07. Prices are placeholders.

---

## 1. Hero

**Headline options**
1. Your vault. Your cloud. Your keys.
2. The password manager that shows its work.
3. Passwords locked with two secrets. Only you hold both.

**Subhead**
{NAME} is a zero-knowledge password manager with a published crypto spec and threat model. Sync through the cloud folder you already use, or let us host it. No telemetry, no tracking.

**Primary CTA:** Download {NAME} (desktop, development build)
**Secondary CTA:** Read how it works

**Trust strip (under the buttons)**
Pre-audit software. Published crypto spec. Published threat model. No analytics.

> Note for the designer: keep the pre-audit line visible in the hero, not hidden in the footer.

---

## 2. How it works, in plain words

**Two secrets unlock your vault.**
Your master password is one. A Secret Key, 128 random bits created on your device, is the other. The Secret Key never leaves your devices, and you save it once in your Emergency Kit.

**Everything is locked before it leaves your device.**
Each item is encrypted on its own. Sync only ever moves ciphertext, whether it goes through your cloud folder or our server.

**A stolen server gives attackers nothing to guess.**
Sign-in uses OPAQUE, so the server never receives anything derived from your password that it could be brute-forced against. Without your Secret Key, a copy of the database is not enough.

**No backdoor, on purpose.**
If you lose your master password and your Emergency Kit, nobody can recover your data, including us. That is the price of zero knowledge, and we say it up front.

*Link: Read the cryptography specification.*

---

## 3. Features

### On your desktop (available in the development build)
Windows, macOS and Linux. Logins, secure notes, cards, identities, SSH keys, API keys, Wi-Fi and file attachments, with custom fields and tags. Command palette, keyboard-first. Quick unlock with Windows Hello or Touch ID. Auto-lock and clipboard auto-clear.

### Browser extension (development build, not in any store yet)
Chromium browsers and Firefox. Fills one login after you click, never on page load. Exact-origin matching, no filling into cross-origin frames, no fill over plain HTTP. Talks to the desktop app over a local, authenticated channel.

### Passkeys (available in the development build)
Create and use passkeys in the browser, with a confirmation in the desktop app for each one. Import from Bitwarden, KeePassXC and FIDO's exchange format.

### One-time codes
TOTP, HOTP and Steam Guard. Scan a QR code from your screen or an image. Codes can live in a separate vault that needs a hardware key.

### SSH agent
Serve your SSH keys from the vault. Every use needs your approval.

### Import
Bring your data from Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) and CSV, with a preview before anything is saved.

### Generator and security report
Random passwords and passphrases from a cryptographic random source, with live entropy. An offline report for weak, reused and old passwords. Nothing leaves your device.

### Sync through your own cloud folder **(coming with M6)**
Point {NAME} at a folder in Dropbox, iCloud, OneDrive or Nextcloud. Your files stay encrypted, and the provider only sees what is listed in our spec.

### Hosted sync **(coming with M6)**
Prefer not to manage anything? Use sync on our server, or run the server yourself with a single Compose command.

### Also planned
Password rotation and breach checks (M7), a command-line tool (M8), mobile apps for iOS and Android (M10).

---

## 4. Security and honesty

**{NAME} is pre-audit.** No independent security audit has happened yet. We intend to commission one before version 1.0. Until then, please do not trust it with secrets you cannot afford to lose, and keep your current manager while you try it.

**What we publish**
- The cryptography specification, with every primitive and library named. We use standard building blocks (Argon2id, XChaCha20-Poly1305, HKDF, OPAQUE, HPKE) and invent none.
- The threat model, including a register of risks we have accepted. Examples: malware on an unlocked computer can read what you see, and a stolen device that exposes the Secret Key leaves only your master password in the way.
- Every decision, as a written record.

**What the server sees (hosted sync)**
Your account email, per-device IDs and names, vault and record IDs, sizes and upload times, and encrypted data. It does **not** see titles, URLs, usernames, tags, vault names or file names. IP addresses are kept in memory for rate limiting and never written to disk. The full table is in section 12 of the spec.

**What your cloud provider sees (folder sync)**
File names, sizes, counts and edit times, not content. Anyone with access to the folder can also delete or withhold files. See section 12.1 of the spec.

**No telemetry.** No analytics, no third-party scripts, no crash reports sent anywhere.

---

## 5. Free and paid

| | **Free** | **Hosted sync** |
|---|---|---|
| Price | 0 | {PRICE} per {PERIOD} |
| Desktop app and browser extension | yes | yes |
| Passkeys, one-time codes, SSH agent, import | yes | yes |
| Sync through your own cloud folder (Dropbox, iCloud, OneDrive, Nextcloud) **(coming)** | yes | yes |
| Sync through our server **(coming)** | no | yes |

The app is free, including sync through a folder you own. The subscription pays for running the sync server. Details and price will be final at launch.

---

## 6. For teams (coming)

Small team? Shared vaults, offboarding and emergency access are planned for milestone M6b. They are **not available yet.** The design lets a team keep its data in its own Dropbox Business team folder or on a self-hosted server. If that fits you, join the list and tell us what you need.

**CTA:** Tell me when teams are ready

---

## 7. Downloads

**Verify what you install. (coming with M9)**
Release builds will ship signed, with checksums, an SBOM and instructions to reproduce the build. Until M9, only development builds exist, and they are unsigned.

| Platform | Status |
|---|---|
| Windows, macOS, Linux desktop | development build |
| Chrome, Edge, Firefox extension | development build, store listings pending |
| iOS, Android | planned (M10) |
| Command line | planned (M8) |

---

## 8. FAQ

**1. Is {NAME} safe to use?**
It is built carefully and designed to resist a stolen server, but it has not been independently audited. Treat it as beta software and keep a second copy of anything important.

**2. Will there be an audit?**
We intend to commission an independent audit before 1.0 and to publish the result. *(Owner to confirm.)* We will not use the word "audited" until that report exists.

**3. What happens if I forget my master password?**
You need your Emergency Kit, which holds the Secret Key, plus a master password. If you lose both, the data cannot be recovered by anyone.

**4. Can I import from my current manager?**
Yes, from Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) and CSV. LastPass users can export to CSV and import that. Passkeys import from Bitwarden, KeePassXC and the FIDO exchange format.

**5. How does sync work?**
Not released yet. When it is, you choose: a folder in your own cloud storage, or our hosted server. Both move only encrypted data.

**6. What does hosted sync cost?**
{PRICE}. The app and folder sync stay free.

**7. Do you collect usage data?**
No. No analytics, no telemetry. We count downloads and public GitHub activity, nothing about you.

**8. Where is the code?**
The specification and threat model are public. License and source availability: {LICENSE_STATEMENT} (decision pending).

**9. Is there a mobile app or a CLI?**
Not yet. Mobile is planned for M10 and the CLI for M8.

**10. Can my team use it?**
Not yet. Team sharing is planned for M6b. Individuals are the focus of the first release.

---

## 9. Footer

{NAME} (working title). Pre-audit software.
Links: Cryptography spec, Threat model, Architecture decisions, Security policy (M9), Changelog, Privacy: we collect nothing, Imprint {IMPRINT}, Contact {CONTACT_CHANNEL}.
Languages: English, Deutsch.
