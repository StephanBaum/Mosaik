# Launch plan

> **Working title.** "Mosaic" and the domain are not final. Dates are relative to milestones, not calendar dates, because the roadmap is milestone-driven (`docs/PROGRESS.md`, as of 2026-11-07: M1 to M5 done, M6 nearly done). No analytics or tracking anywhere in this plan.

## 1. Principles

1. Honest before loud. Pre-audit is stated in every post's first paragraph.
2. Individuals first; teams get a waiting list, not a pitch.
3. Technical audience first: spec, threat model and the hostile-site test suite are the product's proof.
4. Do not launch publicly before the thing people will try first (install, import, fill, sync) works end to end.
5. Respect each community's self-promotion rules; read them the week before and ask moderators where the rules are unclear.

## 2. Phases

### Phase 0: prepare (now until M6 is released)
- Settle name, domain, license statement, price, audit intent (see README, open decisions). Nothing public ships under a placeholder.
- Publish the spec, threat model and decision log from a stable public place (docs site or repo), with the pre-audit banner.
- Build the landing page from `landing-copy.*.md` (M9 includes the page; a minimal version can ship earlier).
- Write the "Moving from X" import guides (Enpass, Bitwarden, KeePass, 1Password, LastPass via CSV).
- Set up a security contact channel and a short disclosure policy (the full SECURITY.md arrives in M9; a stub is enough for beta).

### Phase 1: private beta (after M6 ships, before M9)
- Audience: 20 to 50 people. Sources: GitHub followers, a few friendly community members, security-minded colleagues, invited readers of the spec.
- Offer: desktop and extension development builds, folder sync. Hosted sync only if the server is deployed and operated.
- Ask: break it, read the spec, test import from your manager, report in a public or private issue tracker.
- Rules: people keep their old manager; no real secrets without an informed opt-in. Say so on the invitation.
- Exit criteria: no known data-loss bug in sync or import for two weeks; installer and extension install work on all three desktop OSes; manual checks listed in PROGRESS (Hello, Touch ID, Firefox, Chrome registry install) done.

### Phase 2: public beta (after M9: hardening, signed releases, landing page)
- Needs from M9: signed releases and checksums, SBOM, SECURITY.md with a disclosure policy, AUDIT_PREP.md, the downloads page with verification instructions.
- Extension in the Chrome Web Store and Firefox AMO (store IDs are owner items in PROGRESS).
- Main public launch happens here (section 3). Label: **public beta, pre-audit**.
- M7 (rotation) and M8 (CLI) can land before or after; do not wait for them. Mention as roadmap only.
- Teams: waiting list only, until M6b.

### Phase 3: 1.0 after the audit
- Precondition: independent audit commissioned, findings fixed, report published. M10 mobile apps are available or the 1.0 page states plainly that they are not.
- Second launch moment: "audited and published" (only then may that word appear), Show HN update, Product Hunt launch (best kept for 1.0 so the upvote moment has the stronger story), press outreach with the audit report as the news.
- Teams launch follows M6b as its own small announcement.

Timing relative to milestones: private beta needs M6; public beta needs M9; 1.0 needs the audit and, ideally, M10. M10 mobile is the largest gap for switchers, so be upfront in the comparison page.

## 3. Channels and order for the public beta

| Order | Channel | What | Notes and rules |
|---|---|---|---|
| T-14 days | Mastodon / Fediverse | Build-in-public posts: the spec, the threat-model residual risks, a short demo clip | Use hashtags #privacy #infosec #selfhosted; no ads; reply to everyone |
| T-7 days | Press and newsletters: heise, Golem, netzpolitik-type outlets, privacy newsletters and podcasts | Short pitch with the pre-audit angle, a link to the spec, an offer of an interview | Send individually, no mass mail; German angle: made in Germany if true (owner to confirm), du-friendly DE UI, own cloud folder |
| T-0, weekday morning | Hacker News: Show HN | Title like "Show HN: {NAME}, a password manager with a published threat model (pre-audit)" | Follow the Show HN rules: a link to something people can try, no sign-up wall, the author answers comments for hours; no voting rings, no asking friends to upvote |
| T-0 | r/selfhosted | Focus on the self-hostable sync server and folder sync (once released) | Read the sub rules on self-promotion and AI or new-project posts first; use the right flair |
| T+1 | r/privacy | The no-telemetry, published-threat-model angle | Strict self-promotion limits; ask a moderator before posting; be ready for hard questions |
| T+2 | Password-manager communities (r/Bitwarden, r/KeePass, r/1Password, r/Passkeys) | Do **not** post an ad. Join existing "alternatives" threads honestly, or post a "migration notes and import testing" question if the rules allow | Many of these ban competing-product promotion; if a rule says no, do not. A migration guide on your own site is the safe route |
| T+3 | Lobsters, relevant tech forums, Rust communities | Technical posts: the crypto spec, the hostile-site suite, the sync simulation tests | Lobsters is invite-only and has its own rules |
| Later | Product Hunt | Save for 1.0 | Prepare assets, a maker comment, a real first-day answer team |

Mastodon and email stay the long-term channels; they are the ones we control.

## 4. Assets needed

- Landing page (EN, DE) with the pre-audit banner, downloads, FAQ, waiting list form (email only, stored by us, no third-party form tool).
- 60 to 90 second demo: install, import from another manager, Secret Key and Emergency Kit, fill in the browser, passkey sign-in.
- 5 screenshots (light and dark, DE and EN) from the real app, not mockups.
- One diagram: where encryption happens and what the server sees (from CRYPTO_SPEC §12).
- Spec, threat model and decision log published with a readable index.
- Comparison page (from `comparison.md`, re-verified on the day).
- Import guides per source manager.
- Press kit: one-pager, logo, screenshots, founder bio, fact sheet, a "what we do not claim" box.
- Show HN text, Reddit posts and pitch emails drafted ahead, each leading with pre-audit.
- Open-graph image, favicon, imprint, privacy statement ("we collect nothing").
- Signed release artifacts and verification instructions (M9).

## 5. Checklist

Before private beta
- [ ] name, domain, license statement, price decided
- [ ] M6 released; sync, import and fill run on Windows, macOS, Linux
- [ ] issue tracker and security contact ready
- [ ] pre-audit banner in the app's first run and on every page

Before public beta
- [ ] M9 done: signed releases, SBOM, SECURITY.md, AUDIT_PREP.md
- [ ] store listings approved (Chrome, Edge, Firefox)
- [ ] landing page live in both languages; comparison facts re-verified
- [ ] FAQ answers rehearsed (section 7)
- [ ] a person available to answer comments for the first 12 hours
- [ ] community rules read; moderator asks sent
- [ ] backup of the server and a status page for hosted sync, if offered

Before 1.0
- [ ] audit report published, critical and high findings fixed
- [ ] M10 status stated truthfully
- [ ] press outreach with the report

## 6. Success measures (privacy-respecting)

No tracking pixels, no analytics scripts, no fingerprinting. Use only:
- Download counts from the release host (aggregate counts only).
- GitHub stars, forks, issues and discussion activity.
- Extension store install counts (the store's own aggregate numbers).
- Waiting-list and hosted-sync sign-ups (data we hold because the person gave it to us).
- Count of imported-from-X reports in issues and feedback (self-reported).
- Quality of feedback: number of substantive spec and threat-model issues raised, and number fixed.
- Server-side aggregate counts such as the number of accounts (no per-user behavior logs).
- Qualitative: replies, mentions, press pieces.

Targets are set by the owner; a sensible first target is a small number of engaged testers and substantive security feedback rather than raw volume.

## 7. Risks and how to answer them

| Risk | Answer |
|---|---|
| "Launching a password manager with no audit is irresponsible." | Say it first, in the title. Position as a public beta for people who keep their old manager. Commit to the audit and publish the scope (AUDIT_PREP.md). Do not use any word that implies assurance. |
| Security community finds a real flaw during launch | Have a disclosure channel ready. Respond within hours, thank the reporter, fix, publish a short post-mortem. A visible fast fix helps more than silence. |
| "Why trust a new project over Bitwarden or KeePassXC?" | Do not ask for trust. Point to the spec, the threat model, the residual-risk register and the tests; admit the competitors' advantages (audits, maturity, mobile) in the comparison. |
| Accusations of astroturfing or spam | No sockpuppets, no friend upvotes, disclose affiliation in every post, follow each community's rules. |
| Scrutiny of the zero-knowledge claim | Link the exact metadata table (§12, §12.1). Do not claim "the server sees nothing". |
| Hosted sync outage or data loss at launch | Keep hosted sync small and capped; folder sync needs no server; publish the backup and restore approach; keep the free path first. |
| Missing mobile apps disappoint switchers | State it on the first screen of the downloads section; tell them to keep the current manager for mobile until M10. |
| Team customers ask for features that do not exist | Waiting list; never promise a date for M6b. |
| Name or trademark conflict | Resolve the name and run a trademark check before the first public post. |
| Overpromising the folder-sync privacy | State that the cloud provider sees file names, sizes, counts and edit times, and can delete or roll back files (R54). |

## 8. Prepared answers to likely first comments

- "Is it open source?" Answer according to the owner's license decision; until then, say the spec and threat model are public and the license decision is pending.
- "Has it been audited?" No. We intend to commission one before 1.0 (owner to confirm) and will publish it.
- "What about LastPass-style breaches?" Link §12. Backups stolen from a server contain ciphertext and the listed metadata; without the Secret Key an offline attack on the master password is not enough.
- "Why not just use KeePassXC with Syncthing?" Fair. If that works for you, keep it. Mosaic is for people who want the same control with passkeys, an extension that refuses unsafe fills and a built-in sync story.
