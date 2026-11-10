# The Mosaic Security Challenge

Mosaic is pre-audit software. We have no paid audit. So we invite you to try
to break it, and we promise to treat you fairly when you do. This page is the
rulebook. The targets are in [`CHALLENGE.md`](CHALLENGE.md). To report a
problem that is not part of the challenge, use [`SECURITY.md`](../SECURITY.md).

## How it runs

- The challenge is open all the time. It runs in seasons.
- A season ends when the test server is reset, or when we announce it. Each
  reset gets new flags and new data.
- A finding counts for the season in which you report it. A fix we ship lasts
  across seasons.
- The test server is separate from anything real. It holds no real data
  ([`deploy/contest/README.md`](../deploy/contest/README.md)).

## Who can take part

Anyone, unless the law where you live forbids it. If you are under 18, ask a
parent or guardian first. People who work on Mosaic can report findings, but
they are not eligible for prizes.

## What is in scope

- The seven targets C1 to C7 in [`CHALLENGE.md`](CHALLENGE.md).
- Any other flaw in the code in this repository that breaks a promise in the
  [threat model](THREAT_MODEL.md) or the [cryptography spec](CRYPTO_SPEC.md).
  Report it through [`SECURITY.md`](../SECURITY.md).

## What is out of scope

- Denial of service and traffic floods, including load tests.
- Social engineering of people who work on Mosaic or other users.
- Physical access to anyone's machine.
- Services we do not run: GitHub, browser stores, your operating system, your
  cloud storage.
- The website.
- The risks we accept in the [threat model, section 5](THREAT_MODEL.md),
  unless your report shows harm beyond what that row describes.
- Anything that needs malware running as the same user, where a row of the
  threat model accepts it (attacker P5).
- Findings that only work against a build you changed.

## Rules

- Test only the challenge accounts, your own accounts and your own machines.
- Do not read, change or delete another person's data.
- Do not publish a finding before the advisory, or before 90 days have
  passed, whichever comes first.
- Do not publish a flag before the season ends.
- One finding per report. Duplicates: the first complete report counts.
- Tell us how you did it. A flag alone, with no steps, does not count.
- Do not use what you find to harm anyone.

## Safe harbor

This section is adapted from the disclose.io core terms, which are released
under CC0.

We want you to report problems. If you follow this policy, then:

- We consider your research authorized, and we will not start or support
  legal action against you for it.
- If a third party starts legal action over research that followed this
  policy, we will say, publicly if needed, that it was authorized.
- We will not ask a service provider to take action against you over it.
- We cannot speak for anyone who is not us. If your research touches a
  system we do not run, that owner decides.

If you are not sure that something is allowed, ask first through
[`SECURITY.md`](../SECURITY.md). This safe harbor has not had a legal review.

## From report to thank-you

1. **Report.** Use GitHub private vulnerability reporting, as in
   [`SECURITY.md`](../SECURITY.md). Say which target, and include your steps.
2. **Triage.** We reply within 7 days and rate it within 14.
3. **Fix.** We aim to ship within 90 days, with a test that fails before the
   fix and passes after.
4. **Advisory.** We publish a GitHub security advisory.
5. **Hall of fame.** If you say yes, we add you to
   [`HALL_OF_FAME.md`](HALL_OF_FAME.md).

## Rewards

By default, the reward is recognition: your name in the hall of fame, if you
want it, and a thank-you in the advisory.

Money or other prizes are not set. If the owner sets them, they go in this
table before the season starts. Until then every row stays a placeholder,
and nothing here is a promise to pay.

| Finding | Reward |
|---|---|
| Critical (see [`SECURITY.md`](../SECURITY.md)) | {PRIZE_CRITICAL} |
| High | {PRIZE_HIGH} |
| Medium | {PRIZE_MEDIUM} |
| Low | {PRIZE_LOW} |
| Each of C1 to C7, first valid solution | {PRIZE_FLAG} |
