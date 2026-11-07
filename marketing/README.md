# Marketing documents

> **Working title.** "Mosaic" is not the final name. Written 2026-11-07 from `docs/PROGRESS.md`, `docs/BRIEF.md`, `docs/CRYPTO_SPEC.md` and `docs/THREAT_MODEL.md`. Competitor facts were researched in 2026-11 and need re-checking before publication.

## Index

| File | Content |
|---|---|
| [positioning.md](positioning.md) | ideal customer profile, problem and alternatives, value proposition, three one-liners, messaging pillars with proof, tone of voice, claims table |
| [landing-copy.en.md](landing-copy.en.md) | landing page copy, English |
| [landing-copy.de.md](landing-copy.de.md) | landing page copy, German (informal "du") |
| [comparison.md](comparison.md) | matrix against 1Password, Bitwarden, Proton Pass and KeePassXC, honest takes, sources |
| [launch-plan.md](launch-plan.md) | phases, channels, assets, checklist, measures, risks |

## Hard rules applied

No "audited", "bank-grade", "military-grade", "unhackable" or "100% secure". No telemetry or analytics, including in the launch plan. No license named, no "open source" claim (the brief does not say so). Planned features carry their milestone.

## Open owner decisions

1. **Product name** (Mosaic is a working title; the repo is "Mosaik"): copy uses `{NAME}`. Includes a trademark check.
2. **Domain** (`{DOMAIN}`).
3. **Hosted sync price and billing period** (`{PRICE}`, `{PERIOD}`), and whether there is a trial or a family tier.
4. **License and the "open source" claim** (`{LICENSE_STATEMENT}`). The docs do not decide it. The task brief calls the product open source, but `docs/BRIEF.md` does not, so the copy avoids the phrase.
5. **External audit**: the docs only prepare for one (M9 `AUDIT_PREP.md`). Copy says "we intend to commission one before 1.0". Confirm, and decide on timing, auditor and whether to publish the report.
6. **Origin story / "made in Germany"**: press angle only if true; confirm.
7. **Company or person behind the product**, imprint (`{IMPRINT}`) and contact channel (`{CONTACT_CHANNEL}`). No emails are in these files.
8. **Waiting-list tool** for teams and beta: self-hosted form or plain mailbox, no third-party tracking.
9. **Beta policy**: who may use real secrets, and when hosted sync opens (needs a deployed, operated server).
10. **Team timing** (M6b): no date is promised in the copy.
11. **Launch timing**: public beta after M9, Product Hunt kept for 1.0; confirm.
12. **Store listings** (Chrome, Edge, Firefox): store IDs are existing owner items in PROGRESS.
13. **Whether to mention Secret Key and Emergency Kit** prominently in the hero (they are a friction as well as a strength).
14. **Competitor claims** marked "n/v" in `comparison.md` need verification or removal.
15. **Mobile gap** messaging: decide whether to delay public beta until M10 or launch desktop-first as written.
