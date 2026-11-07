# Competitor comparison

> **Working title.** "Mosaic" is a working name.
>
> **Framing.** Mosaic's column shows the **finished product at launch** (all milestones through M10, per `docs/BRIEF.md` and `docs/PROGRESS.md`). On publication day, check that column against `docs/PROGRESS.md` and mark anything not shipped. Competitor facts were researched in 2026-11 ("as of 2026-11"); prices and features change, so re-check every cell on launch day. Cells marked **n/v** were not verified from a reliable source and must be checked or left blank before they go public. Be fair: where a competitor is ahead, say so.

## 1. Matrix

| | **Mosaic** (at launch) | **1Password** | **Bitwarden** | **Proton Pass** | **KeePassXC** | **Enpass** |
|---|---|---|---|---|---|---|
| **Where your vault syncs** | **Your choice:** own cloud folder (Dropbox, iCloud Drive, OneDrive, Nextcloud, NAS, Dropbox Business team folder), hosted Mosaic server, or self-hosted server | Vendor-hosted [1] | Vendor-hosted; self-hostable server [2] | Vendor-hosted [7] | No built-in sync; any file sync [4] | Own cloud (Dropbox, Google Drive, OneDrive, iCloud, Box, WebDAV, Nextcloud), folder or Wi-Fi sync; no hosted service [10] |
| **Detects a server that rolls back, freezes or forks your vault** | **Yes:** rollback and freeze on both backends, fork ("server inconsistent") on hosted; spec §10 | Not described in its docs or its response to ETH 2026 [1][11] | Not described (n/v) | Not described (n/v) | Not described; KDBX has no anti-rollback counter (n/v) | Not described (n/v) |
| **Login protocol** | OPAQUE (aPAKE) | SRP [1] | Client-side hash sent to server [12] | SRP [13] | No account | No account |
| **Second secret beyond the password** | 128-bit Secret Key | Secret Key [1] | No (n/v) | No (n/v) | Optional key file | Optional key file (n/v) |
| **Malicious-server research (ETH Zurich 2026)** | Not studied (new) | 2 attack scenarios, "known architectural limitations" [11] | 12 attacks, fixed per Bitwarden [11] | Not studied | Not studied | Not studied |
| **Verify who you share with** | Safety codes, key pinning, change warning | Says a malicious server could hand out dishonest public keys [1] | Fingerprint phrase (n/v scope) | n/v | n/a | n/a |
| **Default fill matching** | Exact origin (scheme, host, port); eTLD+1 per item, off by default | Domain; exact host per item [14] | Base domain [15] | n/v | n/v | n/v |
| **Autofill on page load** | Never, no setting | n/v | Optional, off by default [15] | n/v | n/v | n/v |
| **Telemetry** | None, no setting | Opt-in for individuals; on by default for business [16] | n/v | n/v | None known (n/v) | n/v |
| **Server metadata documented field by field** | Yes (spec §12, §12.1) | White paper [1] (field list n/v) | White paper [12] (field list n/v) | n/v | n/a | n/a |
| **Passkeys** | Yes; extension authenticator, iOS and Android providers, CXF import | n/v | Yes (n/v detail) [8] | Yes, all plans [7] | Yes, since 2.7.7 [4] | Yes (n/v detail) |
| **Built-in 2FA codes** | TOTP, HOTP, Steam Guard; optional hardware-key-only vault | n/v | Premium (n/v) | Plus and up [7] | Yes | Yes (n/v) |
| **SSH agent** | Yes, per-use approval | Yes (n/v detail) | Yes, desktop 2025.1.2+ [17] | Rolling out [18] | Yes (n/v detail) | n/v |
| **Password rotation** | Guided changes on any site; connectors for SSH keys, PostgreSQL, MySQL, webhooks; breach check (k-anonymity) | n/v | n/v | n/v | No | n/v |
| **Sharing and emergency access** | Yes (shared vaults, revoke re-keys, waiting-period access) | Yes, mature | Yes, mature | Vault sharing [7] | No | Via shared vaults in your cloud (n/v) |
| **Mobile apps** | iOS, Android, native autofill and passkeys | Yes | Yes | Yes [7] | No official app [4] | Yes |
| **CLI** | Yes | Yes (n/v) | Yes (n/v) | Yes [18] | Yes (`keepassxc-cli`, n/v) | n/v |
| **Independent audit** | **Not yet.** Spec, threat model and audit prep public | Publishes assessments [5] | Annual audits; ETH crypto audit 2026 [2] | Published audits [3] | 2023 audit [4] | n/v |
| **Source code** | {LICENSE_STATEMENT} (undecided) | n/v | Open source since 2017 [2] | Clients yes, server no [3] | GPL [4] | n/v |
| **Price, individual** | App and folder sync free; hosted {PRICE} | $47.88/yr [6] | $19.80/yr [9] | Free; Plus $2.99/mo yearly [7] | Free | n/v |
| **Price, family or team** | Teams {TEAM_PRICE} | Family $71.88/yr [6] | Families $47.88/yr [9] | Family $4.99/mo yearly [7] | n/a | n/v |
| **Track record** | New | Long | Long | Since 2023 | Long | Long |

LastPass, for context: in 2022 attackers copied backups of customer vault data that held unencrypted metadata such as website URLs [19]. ETH Zurich's 2026 study counted 7 malicious-server attacks against LastPass [11].

## 2. What only Mosaic does (in this set)

Verified against the matrix; details and caveats in `positioning.md` §5.

1. **Own cloud folder or hosted server, one end-to-end protocol.** Others are hosted-only (1Password, Bitwarden, Proton Pass) or own-storage-only (KeePassXC, Enpass).
2. **Lying-server detection with a visible warning.** No other vendor here documents rollback, freeze or fork detection of vault data.
3. **OPAQUE sign-in.** 1Password and Proton use SRP, Bitwarden sends a client-side hash; KeePassXC and Enpass have no account.

Not unique, and the copy must not claim it: a Secret Key (1Password has one), an SSH agent (1Password, Bitwarden, KeePassXC; Proton rolling out), passkeys, 2FA codes, zero telemetry (KeePassXC, likely).

## 3. Short honest takes

**1Password.** The most polished mainstream option, with a Secret Key like ours, mature apps and teams, and a long record. Paid only, with a March 2026 price rise [6]. Its own white paper says a malicious server could hand out dishonest public keys [1]. Mosaic matches the two-secret idea and adds OPAQUE, storage choice and lying-server detection; 1Password has years of polish and audits we do not yet have.

**Bitwarden.** Open source, regularly audited, inexpensive, and a sensible default for many people [2][9]. ETH Zurich found 12 malicious-server attacks, which Bitwarden says it has fixed [11]. Mosaic differs on the trust model: a Secret Key, OPAQUE, exact-origin matching by default, storage choice and lying-server detection.

**Proton Pass.** A privacy brand with a real free tier, passkeys on all plans, aliases and published audits [3][7]. Hosted only, server code closed per one review [3]. Mosaic's pitch: your storage, your choice, and a client that checks the server.

**KeePassXC.** Free, local-first, GPL, audited in 2023, with passkeys [4]. No built-in sync, no official mobile app, no sharing. Mosaic gives the same "your file, your place" control, plus sync that detects tampering, sharing, mobile apps and a hosted option.

**Enpass.** The closest model to ours: your vault in your own cloud, no vendor server [10]. Mosaic adds a hosted option, rollback and freeze detection on the folder itself, a Secret Key, OPAQUE sign-in for hosted accounts, and a published crypto spec.

## 4. Who should pick which

| If you... | Pick |
|---|---|
| want the longest track record and a published audit today | 1Password or Bitwarden |
| want the cheapest audited, open-source option | Bitwarden |
| already live in Proton's ecosystem or want aliases | Proton Pass |
| want a single local file and are happy to build sync yourself | KeePassXC |
| want your own cloud, a hosted option, a client that catches a lying server, and every spec in the open | Mosaic |

## 5. Sources (all accessed 2026-11)

1. 1Password Security Design white paper, release 0.5.2 (2026-01-22): https://agilebits.github.io/security-design/1Password_Security_Design.pdf
2. Bitwarden: ETH Zurich audit post: https://bitwarden.com/blog/security-through-transparency-eth-zurich-audits-bitwarden-cryptography/ ; third-party audits: https://bitwarden.com/blog/post/third-party-security-audit/ ; self-hosting: https://bitwarden.com/help/install-on-premise-linux/ (URL to verify)
3. Proton on its audit: https://proton.me/blog/pass-open-source-security-audit ; server code and Recurity Labs: https://www.security.org/password-manager/proton-pass/ (secondary, verify)
4. KeePassXC: https://github.com/keepassxreboot/keepassxc/ ; passkeys and audit via https://proprivacy.com/password-manager/review/keepassxc (secondary, verify against release notes and the audit report)
5. 1Password security assessments: https://support.1password.com/security-assessments/
6. 1Password price rise: https://www.macrumors.com/2026/02/24/1password-march-price-increase/
7. Proton Pass pricing (read directly): https://proton.me/pass/pricing
8. Bitwarden passkeys (older secondary source, verify): https://www.blackvoid.club/bitwarden-biometrics-passkeys-new-passwordless-technologies/
9. Bitwarden prices: https://www.businesswire.com/news/home/20260121289564/en/
10. Enpass sync: https://www.enpass.io/docs/manual-desktop/sync.html ; https://www.helpnetsecurity.com/2026/08/10/product-showcase-enpass-password-manager/
11. ETH Zurich, "Zero Knowledge (About) Encryption", IACR ePrint 2026/058: https://eprint.iacr.org/2026/058 ; per-vendor counts: https://infosecurity-magazine.com/news/vulnerabilities-password-managers ; 1Password's response: https://1password.com/blog/eth-zurich-zero-knowledge-malicious-server-review
12. Bitwarden security white paper: https://bitwarden.com/help/bitwarden-security-white-paper/
13. Proton SRP: https://proton.me/blog/encrypted-email-authentication
14. 1Password autofill behavior: https://support.1password.com/autofill-behavior/
15. Bitwarden: https://bitwarden.com/help/uri-match-detection/ ("The default URI match method for all new login items is Base domain") and https://bitwarden.com/help/auto-fill-browser/
16. 1Password telemetry: https://support.1password.com/telemetry and https://support.1password.com/telemetry-teams/
17. Bitwarden SSH agent: https://bitwarden.com/help/ssh-agent/
18. Proton Pass roadmap (SSH agent, CLI): https://proton.me/blog/pass-roadmap-spring-summer-2026
19. LastPass 2022 breach: https://www.cybersecuritydive.com/news/lastpass-cyberattack-timeline/

## 6. Before publishing

- Re-verify every price and feature cell against the vendor's own page on launch day.
- Fill or delete every **n/v** cell; re-read each vendor's security docs for rollback, freeze or fork detection before keeping claim 2 of §2.
- Mosaic's column must match `docs/PROGRESS.md` on that date.
- Send the page to each vendor's community for corrections; fix errors quickly and publicly.
