# Landing page copy (English)

> **Working title.** "{NAME}" is the product name (working title "Mosaic"), "{DOMAIN}" the domain. Neither is final. The license is undecided, so the copy names none and never says "open source".
>
> **Framing.** Written for launch day of the finished product: every milestone through M10 has shipped (owner direction, 2026-11-07). Before publishing, check every line against `docs/PROGRESS.md` on that date and cut or label anything not shipped yet. Claims follow `positioning.md` §5 to §9; "only" claims are scoped to the managers on the comparison page.
>
> **Order.** Follows the design concept (`docs/superpowers/specs/2026-11-07-landing-page-design.md`, branch `feat/web`): hero → locked before it leaves → what a stolen server holds → two secrets → no backdoor → features → security → pricing and teams → download → FAQ. Two chapters are new and marked **NEW**: "Caught lying" (after the stolen server) and "Only {NAME}" (after the features). **Kinetic** lines are short on purpose; **Labels** are 12 px mono uppercase, keep them under 28 characters.

---

## 1. Hero (M1, living mosaic)

**Label:** END-TO-END ENCRYPTED PASSWORD MANAGER

### Headline candidates

| # | Headline | Subhead | Primary CTA |
|---|---|---|---|
| **1 (recommended)** | **Trust no server.<br>Not even ours.** | Every item is encrypted on your device before it moves. Sync through your own cloud folder or ours, and if a server ever rolls back, freezes or forks your vault, {NAME} catches it. | Get {NAME} free |
| 2 | **Your cloud or ours.<br>Readable only by you.** | Dropbox, iCloud, OneDrive, Nextcloud, your team folder or our servers. Same end-to-end encryption everywhere, and a client that notices when storage lies. | Get {NAME} free |
| 3 | **The server can lie.<br>Your vault will know.** | Other password managers take their server's word for it. {NAME} checks: rollback, freeze and fork detection, built into every device. | Start free |
| 4 | **Stored anywhere.<br>Readable nowhere else.** | Passwords, passkeys, 2FA codes and SSH keys, sealed one by one on your device and synced through the storage you choose. | Get {NAME} free |
| 5 | **Every secret. Every device.<br>No blind trust.** | Desktop, browser, iPhone and Android, one encrypted vault, synced through your cloud or ours. Nothing to take on faith: the spec is public. | Download {NAME} |

**Why 1:** it claims the one thing no other major manager we compared does (it assumes the server may be hostile and checks), it is true for both sync backends, it works as two kinetic lines, and "ours" is the confident twist. Kinetic moment: "ours" settles from weight 400 to 720 as the mosaic lands (replaces "both" in the design concept).

**Secondary CTA:** See what the server sees
**Trust strip:** Published crypto spec · Published threat model · Zero telemetry · Not yet independently audited

> Designer: the audit line stays in the hero. Once a published audit report exists, it becomes "Independently audited by {AUDITOR}. Read the report." and not a day earlier.

---

## 2. Locked before it leaves (M2, pinned)

**Label (left):** THIS DEVICE
**Label (right):** SERVER

**Kinetic headline:** Locked before it leaves.

**Body:**
Every login, card, note, passkey and file is encrypted on its own, on your device, with XChaCha20-Poly1305. Sync never moves anything else. Not to your cloud folder, not to our server.

**Beat captions (in scroll order):**
1. Your data. Readable here.
2. Sealed item by item.
3. Leaving as ciphertext.
4. This is all a stolen server would hold.

**Metadata labels (typed in at 0.70 to 0.90):** RECORD ID · SIZE · UPLOADED · DEVICE

**Illustration note:** Illustration. The bytes are random, generated in your browser. The real record format is in the crypto spec.

---

## 3. What a stolen server holds (M3, glass lens)

**Kinetic headline:** A stolen server holds noise.

**Body:**
Steal our database and you get ciphertext, IDs, sizes and timestamps. No titles, no URLs, no usernames, no vault names, no file names. Sign-in runs on OPAQUE, so the server never even holds a password hash worth guessing against.

**Toggle labels:** Server's view | Your view
**Live region:** The server stores this. It cannot turn it into that.

**Tag columns**
- **WHAT THE SERVER SEES:** email · device IDs · record IDs · sizes · upload times · ciphertext
- **WHAT IT NEVER SEES:** titles · URLs · usernames · passwords · notes · vault names · file names · your master password · your Secret Key

**Folder sync paragraph:**
Syncing through your own cloud folder? Your provider sees encrypted files, their names, sizes and edit times. Nothing inside them. *Link: §12.1 of the spec*

---

## 3b. Caught lying **NEW** (suggested signature moment)

> Designer: a short static or lightly animated chapter. Suggested visual: two device cards publish signed checkpoints; the server hands one of them an older or different view; the two digests disagree; a calm warning banner appears in the app's style. Data is fictional and labeled "sample".

**Label:** LYING-SERVER DETECTION

**Kinetic headline:** The server can lie. Your devices notice.

**Body:**
Most password managers believe whatever their server shows them. {NAME} doesn't. Every device signs a checkpoint of what it has seen. If a server or a cloud folder serves you an old copy, quietly stops delivering changes, or shows two of your devices different versions of your vault, {NAME} flags it and keeps flagging it until you look.

**Three beats (labels + one line each):**
- **ROLLBACK** Old data served as new? Refused.
- **FREEZE** Changes held back? You get a warning.
- **FORK** Your devices shown different vaults? "Server inconsistent."

**Warning banner (sample UI text):** Server inconsistent. Two of your devices were shown different versions of this vault. Nothing was lost. Review now.

**Small print:** Detection, not magic: a server can still withhold data, it just can't do it unnoticed. Fork detection compares your devices once they catch up with each other. How it works: spec §10.

---

## 4. Two secrets (M4, pinned)

**Kinetic headline:** One you remember. One your device keeps.

**Body:**
Your vault opens with two secrets: your master password and a Secret Key, 128 random bits created on your device and never sent anywhere. Even if someone had our whole database and guessed your password, they would still be 128 bits short.

**Key tile labels**
- **MASTER PASSWORD** Only in your head.
- **SECRET KEY** Only on your devices. 128 random bits.

**Join label:** Argon2id + HKDF → account key
**Caption after the ripple:** Readable again. Only here.
**Sample key note:** A sample, made in your browser. Never use a key from a website: the app makes yours on your device and never sends it.
**Button:** Make another

---

## 5. No backdoor (static, kinetic weight per word)

**Kinetic headline:** No backdoor. On purpose.

**Display line:** Forget your master password, and nobody can get your data back. Including us.

**Body:**
No master key, no support override, no "account recovery" that is really a backdoor. Your Emergency Kit brings your Secret Key to a new device, and emergency access lets someone you trust in, if you set it up in advance. That is what zero knowledge means, and we say it before you sign up, not after.

> Designer: the concept's "If you lose both" line is replaced. Losing the master password alone is already final; the Emergency Kit holds the Secret Key, not a password reset.

---

## 6. What it does (bento)

**Kinetic headline:** Every secret. Every device.

**Intro:** One vault for everything you guard, on everything you use.

| Tile | Headline | Copy |
|---|---|---|
| Desktop (6×2, live mini vault) | **Fast on every desktop.** | Windows, macOS and Linux. Press a shortcut, type, done: every action is one keystroke away. Unlock with Windows Hello, Touch ID or a security key. |
| Browser (3×2, click → fill) | **No click, no fill.** | Chrome, Edge and Firefox. Fills one login after you click, only on the exact site it belongs to, never in a hidden frame, never over plain HTTP. Look-alike domains get a warning, not your password. |
| Passkeys (3×1) | **Passkeys, everywhere.** | Create and use passkeys in your browser and on your phone, synced and end-to-end encrypted. |
| 2FA codes (3×1, TOTP ring) | **Codes that fill themselves.** | TOTP, HOTP and Steam Guard. Scan a QR code from your screen, and the code fills right after your password. Want more? Keep them in a vault only your hardware key opens. |
| SSH agent | **Your SSH keys, on a leash.** | A built-in SSH agent serves keys from your vault. Every use asks first. |
| Import | **Switch in minutes.** | Enpass, Bitwarden, KeePass, 1Password and CSV, with a preview and duplicate check. Passkeys come along via the FIDO exchange format. |
| Sync | **Your cloud or ours.** | Dropbox, iCloud Drive, OneDrive, Nextcloud, a NAS or a team folder. Or our hosted server. Or your own, with one Docker Compose command. Works offline, merges cleanly, never silently drops an edit. |
| Sharing | **Share without trusting the middle.** | Shared vaults sealed to each person's key. Check a safety code once, and a swapped key can't slip past you. Remove someone and the vault re-keys. |
| Emergency access | **A way in for the people you trust.** | Name someone. If you can't be reached, they get access after the waiting period you set, unless you say no. |
| Rotation | **Rotate before it hurts.** | Breach check that sends only five characters of a hash. A "rotate now" queue, guided password changes on any site, and connectors that rotate SSH keys, PostgreSQL and MySQL passwords and webhooks on a schedule. |
| Security report | **Know where you're weak.** | Weak, reused, breached, old, HTTP-only and missing-2FA logins, with one-click fixes. |
| Generator (live) | **Passwords sites accept.** | Random passwords and EFF passphrases with live entropy, shaped to each site's password rules. |
| CLI | **Secrets in your terminal.** | Search, get, generate and pull 2FA codes from the shell. Inject secrets straight into a command's environment. JSON out. |
| Mobile | **Your phone, first-class.** | iOS and Android with system autofill and passkeys, on the same Rust core as the desktop. |
| Files and fields | **Whatever you need to keep.** | Logins, cards, identities, notes, API keys, Wi-Fi, encrypted files and custom fields. |

**Chip labels:** DESKTOP · BROWSER · iOS · ANDROID · CLI

---

## 6b. Only {NAME} **NEW** (differentiation)

> Designer: no competitor logos or names here (they live on the comparison page). Three big statements, each with a mono "proof" link to the spec. The design concept's "no competitor table" rule still holds.

**Kinetic headline:** What others can't.

**Intro:** We compared {NAME} with the biggest names in password management. These are the things none of them do.

1. **Your folder or our server. Your choice, same encryption.**
   Sync through your own cloud folder, your team's Dropbox folder, our hosted server or a server you run, on one end-to-end protocol. Switch whenever you like. *Proof: spec §10.1*

2. **It catches a lying server.**
   Rollback, freeze and fork detection on every device, with a clear "server inconsistent" warning. Others trust their server's view of your vault. *Proof: spec §10*

3. **OPAQUE sign-in on top of two secrets.**
   The server never holds anything password-derived it could guess against offline, and a stolen database is 128 bits short without your Secret Key. *Proof: spec §5, §9*

**And unlike most password managers**
- Exact-origin fill by default. `login.bank.example` never fills on `evil.bank.example`.
- No autofill on page load. Not even as a setting.
- Zero telemetry. No analytics, no crash reports, no third-party calls.
- Every field our server can see, listed in the spec.
- A threat model that lists the risks we accept, not just the ones we beat.

*Link: Compare {NAME} with 1Password, Bitwarden, Proton Pass, KeePassXC and Enpass →*

---

## 7. Security, in the open

**Kinetic headline:** Read it. Don't trust it.

**Audit statement (always visible, h3 with the warning rule):**
**Not yet independently audited.** Everything an auditor needs is already public: the spec, the threat model, every design decision. *(Owner: add "An independent audit is commissioned for {AUDIT_DATE}" only once it is.)*

**Paper cards**
- **Crypto spec** · Every primitive and library named. Argon2id, XChaCha20-Poly1305, HKDF, OPAQUE, HPKE. We compose, we never invent. *§1 to §16*
- **Threat model** · Attackers, mitigations and a register of the risks we accept. *R1 onwards*
- **Decision log** · Every security decision, with the alternatives we turned down. *ADR log*
- **Disclosure policy** · How to report a vulnerability, and what we promise in return. *SECURITY.md*

**Built like it matters**
Memory-safe Rust core with no unsafe code. Secrets wiped from memory after use. Every parser fuzzed. A hostile test website attacks the browser extension on every build. Releases signed, with an SBOM and instructions to reproduce the build.

**Zero telemetry line:** No analytics. No crash reports. No tracking. The only outside call the app can make is the breach check, and it sends five characters of a hash.

---

## 8. Pricing and teams

**Kinetic headline:** Free where it counts.

| | **Free** | **Hosted** | **Teams** |
|---|---|---|---|
| Price | 0 | {PRICE} per {PERIOD} | {TEAM_PRICE} per user per {PERIOD} |
| Apps for desktop, browser, iOS, Android and CLI | ✓ | ✓ | ✓ |
| Passkeys, 2FA codes, SSH agent, import, rotation | ✓ | ✓ | ✓ |
| Sync through your own cloud folder | ✓ | ✓ | ✓ |
| Sync through our hosted server, with 2FA for your account | | ✓ | ✓ |
| Emergency access | {TBD} | ✓ | ✓ |
| Shared team vaults, in your team folder or on our server | | | ✓ |
| Run the server yourself | ✓ | | {TBD} |

**Line under the table:** The app is free, including sync through storage you already own. You pay only if you want us to run the server.

**Teams band:** **Keep your team's secrets in your team's cloud.** Shared vaults in a Dropbox Business team folder, on Nextcloud or on our server. Offboarding re-keys the vault. Emergency access included. **CTA:** Start a team

> Owner: the plan matrix is a proposal; `{TBD}` cells and the prices are open decisions (README).

---

## 9. Download and verify

**Kinetic headline:** Verify what you install.

**Body:** Every release is signed and ships with checksums, an SBOM and instructions to rebuild it from source.

**Platform cards:** Windows · macOS · Linux · Chrome · Edge · Firefox · iOS · Android · Command line

**Verify steps**
1. **Download** the installer and its signature.
2. **Check** the checksum and signature with one command.
3. **Rebuild** it yourself if you want to be sure. *Link: reproducible-build guide*

---

## 10. FAQ

**1. Is {NAME} safe?**
It is designed so that a stolen or hostile server gets nothing it can use, and so that your devices notice when the server lies. It has not been independently audited yet, and we will not say it has until a report is published. Everything you need to judge it yourself is public.

**2. What happens if your server is hacked?**
The attacker gets ciphertext and the metadata listed in spec §12. Without your master password and your Secret Key there is nothing to decrypt, and OPAQUE means there is no password hash to crack offline.

**3. What if the server itself turns against me?**
That is exactly what {NAME} is built for. It cannot forge your data or slip you an old version unnoticed. If it withholds changes or shows your devices different versions, you get a warning.

**4. Can I sync without your server?**
Yes, for free. Point {NAME} at a folder in Dropbox, iCloud Drive, OneDrive, Nextcloud, a NAS or a Dropbox Business team folder. Your provider only ever sees encrypted files.

**5. What if I forget my master password?**
Then nobody can recover your data. Not us, not your server's operator. Your Emergency Kit holds your Secret Key for new devices, and emergency access lets a person you trust in after a waiting period, if you set it up in advance. There is no third way, on purpose.

**6. Can I import from my current manager?**
Yes: Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) and CSV, so LastPass and others work via CSV. Passkeys come along from Bitwarden, KeePassXC and the FIDO exchange format.

**7. Does it work on my phone?**
Yes. iOS and Android, with system autofill and passkeys.

**8. Do you collect usage data?**
No. No analytics, no telemetry, no crash reports. Not opt-in, not opt-out: there is nothing to switch on.

**9. Can my team use it?**
Yes. Shared vaults, offboarding and emergency access, with the data in your own team folder or on our server.

**10. Where is the code?**
The crypto spec, threat model and decision log are public. {LICENSE_STATEMENT}

**11. What does it cost?**
The apps and folder sync are free. Hosted sync is {PRICE} per {PERIOD}; teams are {TEAM_PRICE} per user.

---

## 11. Footer

{NAME} · {AUDIT_FOOTER, default: "Not yet independently audited."}
Links: Crypto spec · Threat model · Decision log · Security policy · Compare · Changelog · Privacy: we collect nothing · Imprint {IMPRINT} · Contact {CONTACT_CHANNEL}
No cookies, so no banner. · English | Deutsch

---

## 12. 404

**Headline:** This page is ciphertext.
**Link:** Back to the readable part
