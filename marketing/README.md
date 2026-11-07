# Marketing documents

> **Working title.** "Mosaic" is not the final name. Sources: `docs/PROGRESS.md`, `docs/BRIEF.md`, `docs/CRYPTO_SPEC.md`, `docs/THREAT_MODEL.md` and the landing page design concept on `feat/web`. Competitor facts were researched in 2026-11 and must be re-checked before publication.
>
> **Framing (owner direction, 2026-11-07):** written for launch day of the finished product, with every milestone through M10 shipped; confident, premium tone; differentiation first. Nothing outside the docs is promised. On publication day, every feature line is checked against `docs/PROGRESS.md` and anything not shipped is cut or labeled.

## Index

| File | Content |
|---|---|
| [positioning.md](positioning.md) | finished feature set, ICP, value proposition, one-liners, the verified "only Mosaic" list with evidence, "unlike most" list, pillars, tone, claims table |
| [landing-copy.en.md](landing-copy.en.md) | landing page copy in the design concept's scroll order, 5 hero candidates (1 recommended), kinetic lines and labels, two new chapters |
| [landing-copy.de.md](landing-copy.de.md) | the same in German (du) |
| [comparison.md](comparison.md) | matrix against 1Password, Bitwarden, Proton Pass, KeePassXC and Enpass, what only Mosaic does, honest takes, sources |
| [launch-plan.md](launch-plan.md) | phases, channels, assets, checklist, risks. **Predates the finished-app framing**; update it once the decisions below are made |

## Hard rules applied

No "audited" (until a published report exists), "bank-grade", "military-grade", "unhackable", "uncrackable" or "100 % secure". No telemetry or analytics. No license named and no "open source" claim. "Only Mosaic" only for the three verified items and their combination, always scoped to the managers we compared; everything else is "unlike most password managers". Two secrets alone is never sold as unique (1Password has a Secret Key).

## Open owner decisions

1. **Hero headline.** Recommended: "Trust no server. Not even ours." / "Vertrau keinem Server. Nicht mal unserem." This replaces the design concept's "Two secrets. Only you hold both." (`docs/superpowers/specs/2026-11-07-landing-page-design.md` §1 and §10.1 on `feat/web` need the same change, plus the kinetic word: "ours" / "unserem" instead of "both" / "beide").
2. **Two new landing chapters:** "Caught lying" (3b, a possible sixth signature moment) and "Only {NAME}" (6b, differentiation without competitor names). Approve, and decide whether 3b gets a motion design or stays static.
3. **Comparison page:** publish `comparison.md` as its own page linked from 6b (the design concept rules out a competitor table on the landing page itself). Every n/v cell must be resolved first.
4. **Audit.** Copy says "Not yet independently audited" in the hero, section 7, FAQ and footer. Decide whether to commission an audit before launch, the timing, the auditor, and whether the report is published. The wording switches only when a report exists.
5. **Product name** (`{NAME}`) and trademark check; **domain** (`{DOMAIN}`).
6. **Pricing and plans** (`{PRICE}`, `{TEAM_PRICE}`, `{PERIOD}`, the `{TBD}` cells): is emergency access free, are team vaults in your own team folder free or paid, is self-hosting allowed for teams, is there a family plan or a trial.
7. **License and the "open source" claim** (`{LICENSE_STATEMENT}`).
8. **CXF export.** PROGRESS lists "keep or drop the CXF export" as an owner item; copy only claims CXF passkey **import**.
9. **Desktop OS passkey providers** are deferred in PROGRESS (Windows, macOS); copy only claims passkey providers on iOS and Android. Confirm, or add desktop once it ships.
10. **Launch timing** and the beta policy: launch after M10 as this copy assumes, or a desktop-first beta with milestone labels put back.
11. **Company or person behind the product**, imprint (`{IMPRINT}`), contact channel (`{CONTACT_CHANNEL}`), origin story ("made in Germany" only if true).
12. **Team sign-up without third parties** ("Start a team"): `mailto:`, our own endpoint, or the hosted server's own registration.
13. **Store listings** (Chrome, Edge, Firefox, App Store, Play Store): IDs are open owner items in PROGRESS.
14. **Re-verify before publishing:** the "only Mosaic" claim O2 (lying-server detection) depends on vendors not documenting such a feature; re-read their security docs on launch day.
