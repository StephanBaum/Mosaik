# Competitor comparison

> **Working title.** "Mosaic" is a working name. Competitor facts were researched on 2026-11 (all "as of 2026-11"); prices change, so re-check before publishing. Cells marked **n/v** could not be verified from a reliable source in this pass and must be checked before they go on a public page (or stay blank). Mosaic's column states what is **built today**, with milestones for what is planned.

## 1. Matrix

| | **Mosaic** (pre-audit) | **1Password** | **Bitwarden** | **Proton Pass** | **KeePassXC** |
|---|---|---|---|---|---|
| Independent audit | **No.** Intended before 1.0 (owner to confirm) | Publishes assessment reports [1] (details n/v) | Yes: annual third-party audits per Bitwarden; ETH Zurich cryptography audit published Feb 2026 [2] | Yes: published audits, Cure53 and Recurity Labs named in sources [3] | Yes, limited: 2023 audit of 2.7.4 by an independent reviewer, free of charge [4] |
| Source code public | Spec and threat model public; code license undecided | n/v | Yes, open source since 2017 [2] | Client apps yes, server code no, per a review [3] | Yes, GPL-2/GPL-3 [4] |
| Price, individual (as of 2026-11) | App and folder sync free; hosted sync {PRICE} | $47.88/yr (from March 2026 renewals) [5] | Premium $19.80/yr (Jan 2026) [6] | Free; Plus $2.99/mo billed yearly [7] | Free |
| Price, family | n/a yet | $71.88/yr [5] | $47.88/yr, up to 6 users [6] | Family $4.99/mo billed yearly, up to 6 [7] | n/a |
| Free tier | Yes (app + folder sync) | No, 14-day trial per most sources [5] | Yes [6] | Yes: unlimited logins and devices [7] | Yes |
| Two-secret derivation (Secret Key) | Yes (master password + 128-bit Secret Key) | Yes, per 1Password's design; not re-verified here (n/v) | No (n/v in this pass) | n/v | Optional key file |
| Passkeys | Yes (ES256, dev build) | n/v | Stored in the vault since 2023 per a secondary source [8]; current status n/v | Yes, all plans [7] | Yes, since 2.7.7 [4] |
| Built-in 2FA codes | Yes (TOTP, HOTP, Steam) | n/v | Premium feature per pricing pages (n/v) | Plus and up [7] | Yes |
| Sync | **Coming (M6):** own cloud folder or hosted | Hosted | Hosted; self-host options exist (details n/v) | Hosted | None built in; use any file sync [4] |
| Own-cloud-folder sync with team folders | Planned (M6, team sharing M6b) | No | No | No | Manual file sync only |
| Teams and sharing | **Not yet** (M6b) | Yes, mature | Yes, mature | Yes, vault sharing [7] | No |
| Mobile apps | **Not yet** (M10) | Yes (n/v detail) | Yes (n/v detail) | Yes (passkeys on all devices per Proton [7]) | No official app; third-party KDBX clients [4] |
| Browser extension | Chromium + Firefox, dev build, not in a store | Yes | Yes | Yes | Yes (browser integration) |
| SSH agent | Yes | n/v | n/v | n/v | Yes (not verified here) |
| Telemetry / analytics | None, by rule | n/v | n/v | n/v | n/v |
| Breach history | None known; also very new | n/v | n/v | n/v | n/v |

LastPass, for context: in 2022 attackers copied backups of customer vault data. The vaults were encrypted, but the backups also held unencrypted metadata such as website URLs, account emails and billing data [9]. This is why many people are looking to leave. Mosaic's answer is design, not a record: it has no history, and it is pre-audit.

## 2. Short honest takes

**1Password.** The most polished mainstream option, with mature apps, team features and a long track record. It is a paid subscription with no free plan, and raised personal prices in March 2026 [5]. Choose it if you want a finished product and are happy to pay. Mosaic shares its two-secret idea but is not yet close in maturity.

**Bitwarden.** Open source since 2017, regularly audited, cheap, and the default recommendation for good reason [2]. It raised prices in January 2026 but stays inexpensive [6]. It has mature teams and mobile apps. Choose it if you want an audited, open, inexpensive manager today. Mosaic differs by its two-secret model, the folder-sync option and passkey and SSH handling, but it has no audit and no mobile app yet.

**Proton Pass.** A privacy-focused ecosystem with a real free tier, passkeys on all plans and published audits [3][7]. Choose it if you already use Proton or want a friendly free option with aliases. Server code is closed per one review [3]. Mosaic's pitch is more control over where data lives.

**KeePassXC.** Free, local-first, open source (GPL), with passkey support and a published audit [4]. No built-in sync and no official mobile app; you assemble that yourself. Choose it if you like files you control and do not mind the DIY sync. Mosaic aims at the same people but wants to add sync, a polished UI and team folders; it will take time to match KeePassXC's years of use.

## 3. Who should pick which (today)

| If you... | Pick |
|---|---|
| need mobile apps, teams or an audited product this month | Bitwarden or 1Password |
| want the cheapest audited, open option | Bitwarden |
| want a free tier plus aliases, or already use Proton | Proton Pass |
| want a local file you fully control and accept DIY sync | KeePassXC |
| want to follow and test a new design with a Secret Key, folder sync in your own cloud, and published specs, and can tolerate a pre-audit beta on desktop | Mosaic, alongside your current manager |

## 4. Sources (all accessed 2026-11)

1. 1Password security assessments: https://support.1password.com/security-assessments/
2. Bitwarden: ETH Zurich audit post: https://bitwarden.com/blog/security-through-transparency-eth-zurich-audits-bitwarden-cryptography/ ; open source and audit claims on the same page and https://bitwarden.com/blog/post/third-party-security-audit/
3. Proton on its audit: https://proton.me/blog/pass-open-source-security-audit ; server code claim and Recurity Labs: https://www.security.org/password-manager/proton-pass/ (secondary, verify)
4. KeePassXC project page: https://github.com/keepassxreboot/keepassxc/ ; passkey and audit summary via https://proprivacy.com/password-manager/review/keepassxc (secondary, verify against the KeePassXC release notes and audit report)
5. 1Password price rise: https://www.macrumors.com/2026/02/24/1password-march-price-increase/ and https://alternativeto.net/news/2026/2/1password-to-raise-subscription-prices-for-individual-and-family-plans-by-up-to-33- ; free trial claims vary by site
6. Bitwarden prices: https://www.businesswire.com/news/home/20260121289564/en/ and https://alternativeto.net/news/2026/1/bitwarden-enhances-premium-and-families-plans-with-more-security-features-and-raises-prices
7. Proton Pass pricing page (read directly): https://proton.me/pass/pricing
8. Bitwarden passkeys (older secondary source, verify): https://www.blackvoid.club/bitwarden-biometrics-passkeys-new-passwordless-technologies/
9. LastPass 2022 breach: https://www.cybersecuritydive.com/news/lastpass-cyberattack-timeline/ and https://www.helpnetsecurity.com/2022/12/23/lastpass-breach-customer-vault/

## 5. Before publishing

- Re-verify every price against the vendor's own page on launch day.
- Fill or delete every **n/v** cell (1Password source code and passkeys, Bitwarden 2FA and self-hosting, SSH agents, telemetry, mobile apps).
- Send the page to each vendor's community for corrections; fix errors quickly and publicly.
- Mosaic's own claims must match `docs/PROGRESS.md` at that date.
