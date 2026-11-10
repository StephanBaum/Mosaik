# Security review log

**This is not an independent audit.** It is a ledger of the internal security
review of Mosaic, done by AI agents (Claude models) working under human
direction. No outside security firm has examined Mosaic, and Mosaic is
pre-audit software: do not use it for real secrets until an external audit is
complete and published (see the README). Internal review finds bugs; it does
not replace an audit.

The log exists so that a reader can see what was reviewed, by what kind of
reviewer, what it found, and what happened to each finding, without having to
trust a summary. It is kept honest by one rule: **every number comes from a
source** (the review sections of `docs/PROGRESS.md`, the review log in
`docs/THREAT_MODEL.md` §6, the per-track decision files that fixed each review,
and `git log`). Where no count was written down, the log says "not recorded".
Nothing is estimated.

## How to read it

- **Findings** are counted as the reviewers reported them: High / Medium / Low
  / Info. Overlapping findings (two reviewers finding the same issue) are not
  de-duplicated. "Ungraded" means the source records a count but no severity.
- **Model tier** is the `model:` of the reviewer's agent definition in
  `.claude/agents/` on that date (git `366a4c0`; `58178da` moved `ux-reviewer` and
  `test-engineer` from opus to sonnet on 2026-10-29). The model of one individual
  spawn is not recorded, and the orchestrator can override it. Where the agent
  itself is not recorded, so is the tier.
- **Outcome**: *fixed* means fixed with a regression test (the policy of every
  fix round; the number of tests is recorded only where stated). *Accepted* names
  the residual-risk ID in `docs/THREAT_MODEL.md` §5 (R-ids) or `docs/PROGRESS.md`
  "Open risks / findings" (Mx-Ry ids). *Deferred* names the target.
- Info items of a decision file are counted as the items listed there.
- The totals count findings reported, not vulnerabilities proven.

## How we review

1. **Spec first.** `docs/CRYPTO_SPEC.md` is normative. A format or rule change
   edits the spec (and an ADR in `docs/DECISIONS.md`) before the code.
2. **Adversarial reviewer agents per change.** After a track lands, read-only or
   test-writing agents attack it: `crypto-reviewer` (keys, KDFs, formats, secret
   memory), `sync-chaos` (clock skew, partitions, replay, rollback, malicious
   server, concurrent devices), `extension-attacker` (hostile pages, spoofed
   messages, the channel), `ux-reviewer` (WCAG 2.2 AA, `docs/UX_GUIDELINES.md`) and
   `test-engineer` (untested paths, property tests, known answers, fuzz targets).
3. **Fix rounds with a regression test per finding.** The orchestrator writes a
   decision file that turns each finding into an item (fix, accept with a
   threat-model entry, or defer with a target). Each fix gets a test that fails on
   the old code where a test can.
4. **Verify passes.** A second review of the fixes (a re-review) checks that
   the fixes hold and did not open something new. Several tracks needed one.
5. **Gate and milestone pass.** A full gate (format, lint, tests, supply-chain
   checks) runs before each commit, and a milestone-level review looks at how the
   parts fit together.

### What this does not give you, compared with an independent audit

- **Independence.** The reviewers are models briefed by the same project that
  wrote the code. They share blind spots with the authors and with each other.
- **A fixed, reproducible scope and report.** Reviewer reports are not
  published here; this log records counts and outcomes, and some counts were
  never written down.
- **Human expert review.** No cryptographer or penetration tester outside the
  project has read the protocol or the code.
- **Real-world testing.** Hardware, operating-system and browser behaviour that
  cannot be tested in CI is on the manual-check lists in `docs/PROGRESS.md`
  ("Needs owner"), not in this log.
- **A guarantee.** A clean log means these reviews found nothing more, not that
  nothing is left.

## Totals

Graded findings with a recorded severity (not recorded: see the per-milestone
tables).

| Milestone | High | Medium | Low | Info | Ungraded (count recorded, no severity) |
|---|---|---|---|---|---|
| M0 | 3 | 7 | 3 | not recorded | none |
| M1 | 2 | 10 | 17 | not recorded | none |
| M2 | 8 | 43 | 71 | 2 | none |
| M3 | 3 | 24 | 36 | not recorded | 8 (insights track) |
| M4 | not recorded | not recorded | not recorded | not recorded | 7 (extension-attacker breaks) |
| M5 | not recorded | not recorded | not recorded | not recorded | not recorded |
| M6 (T1 to T5 and the milestone review) | 3 | 27 | 42 | 36 | 36 |
| M6b (T1 review, milestone review, two verify passes and the T5 UX review) | 7 | 8 | 11 | 6 | none |
| M7 Key rotation (T1 review, milestone review, one verify pass and two UX passes) | 6 | 20 | 28 | 2 | none |
| **Overall (M0 to M7)** | **32** | **139** | **208** | **46** | **51 recorded** |

The overall Info and Ungraded figures are lower bounds: most milestones did not
record them. M4 and M5 recorded no graded counts, so the High, Medium and Low
totals leave those two milestones out.

## M0 Foundations

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-10-25 | crypto-reviewer, design review of `CRYPTO_SPEC` | opus | 3 / 7 / 3 / not recorded | All resolved in the spec or recorded as R7 to R10 |

Total: 3 / 7 / 3.

## M1 Crypto core

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-10-26 | crypto-reviewer, test-engineer, gate review (`crates/core`, step 10) | opus, opus, not recorded | 2 / 7 / 11 / not recorded | Fixed with regression tests, except the upstream HMAC and HKDF stack residue (R11, M1-R1) and M1-R2 to M1-R6 (accepted) |
| 2026-10-26 | crypto-reviewer, test-engineer, gate review (follow-up round) | opus, opus, not recorded | 0 / 3 / 6 / not recorded | All fixed with regression tests, each checked by mutating the code it guards; the timing note is M1-R6 |

Total: 2 / 10 / 17.

## M2 Vault model and local store

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-10-26 | Track review: model | not recorded | 2 / 3 / 4 / not recorded | Fixed with regression tests or settled by ADR-0008 to ADR-0011 |
| 2026-10-26 | Track review: store | not recorded | 0 / 5 / 6 / not recorded | As above |
| 2026-10-26 | Track review: generator | not recorded | 0 / 2 / 5 / 2 | As above |
| 2026-10-26 | Track review: totp | not recorded | 0 / 1 / 5 / not recorded | As above; residuals R12 to R14 |
| 2026-10-26 | Integration (session layer, 3-device simulation) | not recorded | 0 / 1 / 0 / not recorded | Fixed with a regression test (ADR-0012 item 5) |
| 2026-10-26 | crypto-reviewer, sync-chaos, test-engineer, gate review (integration, round 1) | opus, opus, opus, not recorded | 2 / 11 / 17 / not recorded (the 2 High were one issue) | Fixed with regression tests, except the merge-model gaps (R15, M2-R7 to M2-R9) and the items in "Open risks / findings" |
| 2026-10-27 | Same reviewers (round 2) | as round 1 | 2 / 4 / 14 / not recorded | Fixed, except the in-flight rotation loss (R16, M2-R12) and the delivery-dependent repair (R15), accepted for M6 |
| 2026-10-27 | Same reviewers (round 3) | as round 1 | 1 / 9 / 14 / not recorded | Fixed; attachment re-seal on revocation specified for M6 and M7 (R18, M2-R15); anchor-less replacement reported (R17) |
| 2026-10-27 | Same reviewers (round 4; the loop ended with 14 actionable findings) | as round 1 | 1 / 7 / 6 / not recorded | Fixed; move-into-a-rotating-vault loss recorded under R16 |

Total: 8 / 43 / 71, plus 2 Info. Accepted residuals: R12 to R18 (see the
register). The items it deferred to M6 (M2-R1, R2, R10, R11, R12, R16) are
closed or retargeted in `docs/PROGRESS.md` at the M6 close.

## M3 Desktop app

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-10-29 | Insights track review | not recorded | 8 findings, ungraded | 6 fixed, CI wiring and ADRs carried into the integration, 1 accepted |
| 2026-10-29 | crypto-reviewer over all M3 code | opus | 0 / 3 / 2 / not recorded | Fixed in two rounds, each checked by an independent final review (count of that review not recorded); includes the quadratic 1PUX attachment lookup, fixed with a lookup-count test |
| 2026-10-29 | ux-reviewer over the integrated UI | not recorded (definition changed to sonnet the same day) | 2 / 7 / 5 / not recorded | As above |
| 2026-11-03 | UI fidelity: side-by-side review and a final review | opus (recorded in PROGRESS) | not recorded | The gaps were fixed in 10 work packages plus the final review |
| 2026-11-03 | crypto-reviewer, quick unlock: core and vault | opus | 0 / 3 / 7 / not recorded | Fixed except the manual hardware checks (R23, M3-R6) |
| 2026-11-03 | crypto-reviewer, quick unlock: desktop | opus | 0 / 3 / 9 / not recorded | As above |
| 2026-11-03 | ux-reviewer, quick unlock | sonnet | 1 / 7 / 12 / not recorded | Fixed except L5, L11 and L12 (target not recorded) |
| 2026-11-03 | Red team of the IPC commands, the re-authentication grant and `settings.json` | not recorded | not recorded | Fixed: settings MAC (ADR-0030), unlock and grant races, spoofing characters; residuals R28 (joint rollback, M3-R23) and M3-R24 |
| 2026-11-04 | extension-attacker, desktop IPC and webview surface | opus | 0 / 1 / 1 / not recorded | Fixed with regression tests in `redteam_surface_tests.rs` and `e2e/hostile.spec.ts`; R33 added |
| 2026-11-04 | Crypto review of the QR scan work (one-time-code setup) | not recorded | not recorded | Fixed (`rqrr` vendored with a work bound, pending-slot and clipboard handling); R30 to R32 |
| 2026-11-05 | Attachment review fixes, then UX review fixes | not recorded; sonnet for the ux-reviewer | not recorded | Fixed (`c12d8bf`; `fix/m3-ux-polish`); R34 to R37 |

Total graded: 3 / 24 / 36, plus 8 ungraded.

## M4 Browser extension

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-11-05 | crypto-reviewer, T1 channel | opus | not recorded | Fixed (`c363a27`); R40, R42 |
| 2026-11-05 | crypto-reviewer, T2 desktop endpoint | opus | not recorded | Fixed (`5b32f68`); R43, R45 |
| 2026-11-05 | extension-attacker, T3 extension core (PR #26) | opus | 6 breaks, ungraded | All fixed with regression tests; the hostile suite grew from 14 to 39 specs |
| 2026-11-05 to 2026-11-06 | ux-reviewer, T4 popup and save (PR #27) | sonnet | not recorded | Fixed (`5163dee`) |
| 2026-11-05 to 2026-11-06 | extension-attacker, T4 capture and popup (PR #27) | opus | 1 break (capture clickjacking through an invisible frame), ungraded | Fixed |
| 2026-11-06 | T5 close (CI and documents) | none | none | No review ran on this delta; it changes no code |

Totals: no graded counts recorded; 7 extension-attacker breaks (6 + 1).
Accepted residuals: R38 to R46 (M4-R1 to M4-R12).

## M5 Passkeys

| Date | Review | Model tier | H / M / L / Info | Outcome |
|---|---|---|---|---|
| 2026-11-06 | crypto-reviewer, T1 authenticator | opus | not recorded | Fixed (`687559f`: registrable-domain rp ID rule, crate-private signing, `PendingAssertion`) |
| 2026-11-06 | crypto-reviewer, T2 desktop handlers, UV and counter | opus | not recorded | Fixed (`784a31f`); R49 to R51 |
| 2026-11-06 | extension-attacker, T3 browser interception | opus | not recorded | Fixed (`acdb600`, tests in `2057a67`); R47 |
| 2026-11-06 | crypto-reviewer, T4 import parsing and CXF export | opus | not recorded | Fixed (`d2cba1a`); R52, R53 |
| 2026-11-06 | ux-reviewer, passkey dialog, item views, export panel, popup switch | sonnet | not recorded | Fixed at the close (`eb2e90c`) |
| 2026-11-06 | extension-attacker, per-origin passkey budget | opus | not recorded | Fixed (`84e1e1d`); M5-R8 |

Totals: not recorded. Accepted residuals: R47 to R53 (M5-R1 to M5-R10).

## M6 Sync

Sources: the M6 T1 to T5 sections of `docs/PROGRESS.md`, the decision files of
each fix round (the counts below are the items those files list, by ID) and the
commit messages named in the table. The reviewer reports themselves are not in
the repository.

| Date | Track and review | Model tier | H / M / L / Info | Ungraded | Outcome |
|---|---|---|---|---|---|
| 2026-11-06 | T1 protocol foundation: crypto-reviewer | opus | 1 / 5 / 7 / 1 note | none | Fixed, `8d9d877`; known-answer vectors regenerated; R55 note |
| 2026-11-06 | T2 engine and folder transport: crypto-reviewer | opus | 1 / 5 / 6 / 6 | none | Fixed, `2d5a114`; R57a accepted until re-key (M7); the blob-verification lock hold deferred to T5 |
| 2026-11-06 | T2: sync-chaos | opus | not recorded | 10 (S1 to S10) | Fixed, the ten chaos tests run without `#[ignore]` |
| 2026-11-06 | T2 re-review: crypto-reviewer | opus | 0 / 2 / 3 / 5 | none | Fixed or noted, `a79ef47`; old-`K_fold` segment handling at re-key deferred to M7 |
| 2026-11-07 | T3 server: server review (agent not recorded) | not recorded | 1 / 3 / 8 / 7 | none | Fixed with a regression test each, `1dcddb8`; R58, R61 amended, R63 to R66 accepted |
| 2026-11-07 | T4 HTTP transport: crypto-reviewer | opus | 0 / 2 / 4 / 5 | none | Fixed, `8a5aa81`; R56, R57, R67 amended |
| 2026-11-07 | T4: sync-chaos | opus | not recorded | 10 (S1 to S10; S1 graded "severe") | Fixed, the ten chaos tests run without `#[ignore]`; R68 accepted |
| 2026-11-07 | T4 re-review: crypto-reviewer | opus | 0 / 4 / 7 / 5 | none | Fixed, `2a1e7c9`; revocation rotations constrained for M6b |
| 2026-11-07 | T5 desktop: crypto/IPC review (agent not recorded) | not recorded | 0 / 3 / 3 / 3 listed (C-I1, C-I3, C-I4) | none | Fixed, `f682a34`; R69 accepted |
| 2026-11-07 | T5: UX review (agent not recorded) | not recorded | not recorded | 16 (U1 to U16) | All accepted and fixed, `f682a34` |
| 2026-11-07 | T5 verify pass over the fixes (agent and counts not recorded) | not recorded | not recorded | not recorded | Fixes applied, `655a423` |
| 2026-11-07 | M6 milestone review: crypto-reviewer over the whole milestone | opus | 0 / 3 / 4 / 4 | none | M1 to M3, L1 to L3 and I1 to I3 fixed with a regression test each, in the M6 close fix commit (see below); L4 fixed with the trade-off recorded in R62; I4 accepted as R70 (target M6b) |

Total for T1 to T5: High 3, Medium 24, Low 38, Info 32 (counted by ID in the
decision files), plus 36 ungraded (10 + 10 sync-chaos, 16 UX), plus the T5
verify pass whose count is not recorded.

Regression tests: one per fixed item is the rule of each fix round; the number
is recorded only for the two sync-chaos sets (ten tests each, `crates/sync/tests/it/chaos/`
and `crates/server/tests/it/chaos_hosted.rs`).

Known limits left after the fixes: no settings screen for `change_master_password`, no UI for the
different-server confirmation, the folder join account option shows the first
eight characters of the account ID (`docs/PROGRESS.md`, "Known items").

**M6 milestone review (opus), 0 High, 3 Medium, 4 Low, 4 Info.** Medium: M1 a
rebuild that never ends switched the digest check off for every vault (now per
peer and vault, bounded by the 48-hour grace), M2 a password change with an
unknown outcome (checked at once, `syncPasswordOutcomeUnknown`), M3 reconnecting
to a folder kept a superseded password live (reconciled at connect and thread
start). Low: L1 the folder account version was global (reset on a change of
backend), L2 label registry (typed constants for every label, byte values
unchanged), L3 documentation drift (§12, SERVER.md, §11), L4 a forged checkpoint
counter made the halt loop (the published counter does not advance on
`checkpoint_stale`; trade-off in R62). Info: I1 M6b forward constraint extended
(§10.2), I2 pending-login epoch race, I3 `SecretText` equality comment, I4 view
digest leaves out vault metadata (accepted, R70, target M6b). Every fixed item
has one regression test. **Pending:** the final sync-chaos long run over both
transports.

## M6b Sharing and emergency access

Sources: the M6b fix decision files (`m6b-t1-fix.md`, `m6b-fix.md`, kept by the orchestrator), the commit
messages named in the table and `docs/PROGRESS.md` ("M6b"). The reviewer reports themselves are not in the
repository. The T2 to T4 track reviews were not counted separately in the sources: the milestone review
covered them together.

| Date | Track and review | Model tier | H / M / L / Info | Ungraded | Outcome |
|---|---|---|---|---|---|
| 2026-11-08 | T1 core: crypto-reviewer (`b388cc6..eddbb84`) | opus | 1 / 1 / 3 / 5 | none | Fixed, `976e910`: only the owner issues rotation shares (`authorizes_rotation`), rosters bound to their generation, the owner entry fixed at creation, ADR-0049 facts, KAT with the standard library only |
| 2026-11-08 | M6b milestone review: crypto-reviewer over `b388cc6..6501920` | opus | 4 / 4 / 4 / 1 | none | All fixed in `8e3b495` with a regression test each; deviations documented (ADR-0055, R71, R72, R77, R79, R82) |
| 2026-11-08 | Verify pass 1 over the milestone review fixes (`8e3b495`) | opus | 1 / 1 / 2 / 0 (H3 still open, High) | none | Fixed in `789265b` (owned-vault set, grant-delete rotation) |
| 2026-11-08 | Verify pass 2 over `789265b` | opus | 1 / 2 / 2 / 0 (H3 still open) | none | Fixed fail-closed in `e8653b9` (positive owner authorization; `MemberKeyMismatch` and `VaultHeld` findings) |
| 2026-11-08 | T5 desktop: UX review (ux-reviewer) | sonnet | 2 blockers / 8 major / 11 minor | none | All fixed in the follow-up commit `fix(desktop): M6b sharing UX review follow-ups and new findings`, with the two new finding kinds mapped through the contract, mock, fixtures and EN/DE copy |

Total: High 7, Medium 8, Low 11, Info 6 (T1 review, milestone review and the two verify passes, counted by
item in the decision files; H3 was reported open in both verify passes and closed fail-closed in `e8653b9`). The
UX review is counted apart: 2 blockers, 8 major, 11 minor, no security findings.

The headline limits after the fixes (THREAT_MODEL R71 to R82): a member who still holds an old key can forge
roster entries at the old generation (the defence is owner-only rotation plus fingerprint pinning); a
revocation waits for the owner's device; the server keeps the emergency wait, so whoever controls the server
can skip it (the grant dialog says so); vault keys only are covered by emergency access.

## M7 Key rotation

Status: **done.** Sources: the M7 fix decision files (`m7-t1-fix.md`, `m7-fix.md`, `m7-fix2.md`, `m7-ux-fix.md`, kept
by the orchestrator), the commit messages named below and `docs/PROGRESS.md` ("M7 Key rotation"). The reviewer reports
are not in the repository. UX findings are counted as Blocker = High, Major = Medium, Minor = Low.

| Date | Track and review | Model tier | H / M / L / Info | Ungraded | Outcome |
|---|---|---|---|---|---|
| 2026-11-08 | T1 core: crypto-reviewer on the spec text and `mosaic-core` (`19eff2a`) | opus | 0 / 3 / 6 / 2 | none | All fixed in `0e27585`: no equal-epoch adoption (a conflict is resolved by re-keying to max + 1), retired Secret Key commitments refuse any key this device held, a hard halt only on a verified marker (an unverifiable higher-epoch file is a dismissible finding), the server's wrapping of the current generation kept on adoption after a constant-time key check, checked epoch arithmetic and hosted adoption only at current + 1, the backend epoch in a folder is the highest verified one, phase-3 resume needs the last kit group, `rekey/finish` ends pending logins and deletes PRF wrappings, the hosted account version never goes down, `change_credentials` verifies the current wrapping first; the byte KAT of the v2 account file and the marker went to T4; the in-app notice replaces the email (R89) |
| 2026-11-09 | M7 milestone review: crypto-reviewer over `19eff2a..e88dc6b` (core, vault, sync, server, desktop) | opus | 3 / 5 / 5 / not recorded | none | Fixed in `395015e` and `730e8f1`: a folder device adopts any higher epoch that verifies under the typed Secret Key, the join and a fresh hosted registration store the backend's epoch, a cancelled invite reads as cancelled, hosted adoption needs no clean sync, a resume re-queues attachment re-seals, member vaults the device can't open get no rewrap, owned vaults are held until the next vault key arrives, a lower served epoch is a rollback finding, triggers tie the identity epoch to the account epoch, an unreadable attachment is skipped and listed; the reason-6 revocation row is deferred (ADR-0063) |
| 2026-11-09 | Verify pass over the fix round: crypto-reviewer | opus | 1 / 3 / 2 / 0 | none | Fixed in `1fedd69` and `824e0a4` (ADR-0064, R94 to R96): the desktop hosted join uses the served epoch, an epoch jump is capped at 16 so a forged marker can't exhaust the epoch space, marker MACs are checked before the scan budget, only vaults the backend knows are held, a committed job can only be abandoned after the local commit, the hold lookup fails closed, `raise_epoch_to` is test-only, dismissed markers can be restored |
| 2026-11-10 | ux-reviewer on the rotation wizards, resume, adoption and notices (EN light) | opus | 1 / 7 / 10 / not recorded | none | Fixed in `0a24778`, `0f42855` and `c1a546b`: the hidden run progress, five focus losses, a confirm before discarding a prepared change, the hosted re-key card asks for the new Secret Key, clearer notice and conflict copy, live announcements, German wording; a signed-out device still can't learn "re-keyed" before sign-in (M7-R3) |
| 2026-11-10 | ux-reviewer re-check (EN and DE, light and dark, 720×480) | opus | 1 / 2 / 5 / not recorded | none | Fixed in `ef35dc9`: the sticky dialog bar covered the focused field at 720×480, Esc on the stop dialog re-opened it, a warning text had gone to the wrong key, finished phases marked, the sidebar matches the sync state |

Total: High 6, Medium 20, Low 28, Info 2 (counted by item in the decision files).

The headline limits (THREAT_MODEL R83 to R89): until every device adopts a re-key a stale folder device accepts
old-epoch objects that a holder of the old account key writes; members who keep the old pin can be fed forged
rotations until they verify the owner's new safety code; a re-key does not take back what was already copied; vaults
owned by others stay readable to the old key holder until their owner rotates; the pending Secret Key sits in the OS
keystore before the commit.
After the milestone review (R94 to R96): a folder writer can halt an adoption by planting or deleting files (recovery:
re-key from this device or remove the planted files); a device more than 16 epochs behind must be set up again; a device
that a re-key signed out learns that it needs the new Secret Key only from the sign-in hint (M7-R3).

## Appending a milestone

Each milestone close adds one section here and updates the totals table:

1. One row per review pass: date, track, reviewer agent, model tier, H / M / L /
   Info counts, outcome with R-ids or targets.
2. Take counts only from the PROGRESS review section, the THREAT_MODEL §6 log,
   the decision file or `git log`. If a count was not written down, write "not
   recorded".
3. Keep the header: internal, AI-assisted, not an independent audit, pre-audit.
