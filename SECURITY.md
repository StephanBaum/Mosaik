# Security policy

**Mosaic is pre-audit software.** No outside security firm has examined it.
Do not use it for real secrets until an external audit is finished and
published. The internal review we did is listed in
[`docs/SECURITY_REVIEW_LOG.md`](docs/SECURITY_REVIEW_LOG.md). It is not an
independent audit.

## Supported versions

Only the newest pre-release gets security fixes. Older ones do not.

## How to report a problem

Use GitHub private vulnerability reporting:
<https://github.com/StephanBaum/Mosaik/security/advisories/new>

Only you and the maintainers can see the report. Please do not open a public
issue or post the details anywhere before we have talked.

An email address will be added here when the project has a domain. There is
no PGP key. If you need to send something sensitive, say so in the report and
we will agree on a way.

## What to put in a report

- What you found, in a few sentences.
- Which part it touches: desktop app, extension, native host, CLI, sync
  server or a library crate. Give the version or commit.
- Steps to reproduce it, or a proof of concept. A test that fails is ideal.
- What an attacker gains, and what the attacker needs first.
- Whether you want to be named in the [hall of fame](docs/HALL_OF_FAME.md).
  We list nobody without a clear yes.

Do not send real vault data. Test with your own throwaway accounts.

## What to expect from us

| Step | Target |
|---|---|
| We confirm we received your report | 7 days |
| We tell you whether we accept it, and how severe it is | 14 days |
| We aim to ship a fix | 90 days |
| We publish an advisory | at the fix, or at 90 days, whichever comes first |

We can agree on a later date if a fix needs more time. We will tell you what
we plan before we publish.

## How we rate severity

We use CVSS 4.0, plus these Mosaic rules. Where they differ, the Mosaic rule
wins.

- **Critical:** anything that shows vault contents or keys to someone who
  should not see them. Also a break of the master password and Secret Key
  protection.
- **High:** the extension or desktop app fills a login to the wrong origin, or
  fills without a click by the user.
- **Medium and below:** by CVSS 4.0, judged against the
  [threat model](docs/THREAT_MODEL.md).

## Safe harbor

If you test in good faith and follow this policy, we will not take legal
action against you, and we will say so if anyone else tries. Good faith means:

- You test only your own accounts, or the challenge accounts on the test
  server.
- You do not read, change or delete other people's data.
- You do not slow the service down for others.
- You report what you find, and you give us time to fix it.

The full terms are in [`docs/BUG_BOUNTY.md`](docs/BUG_BOUNTY.md).

## Out of scope

- Malware that runs as an administrator, root or the kernel, a hacked
  operating system or hardware, cold-boot and memory-reading attacks on a
  running machine, and forcing the user to unlock.
- Malware that runs as the same user, where the
  [threat model](docs/THREAT_MODEL.md) says we accept it.
- Floods of traffic, social engineering, physical access and services we do
  not run.

## Release signing keys

Releases are signed with a minisign key (`release/minisign.pub`) and the
Android app with an Android signing key. Check a download as
[`docs/VERIFY.md`](docs/VERIFY.md) describes. The Android signer's SHA-256
fingerprint is published there and in `release/android-signer.sha256` with the
first release that ships an APK. Take keys and fingerprints from the
repository, never from a release page or a message.

## More reading

- [Threat model](docs/THREAT_MODEL.md) and [cryptography spec](docs/CRYPTO_SPEC.md)
- [Audit preparation](docs/AUDIT_PREP.md): scope, priorities, how to build and test
- [Security challenge](docs/BUG_BOUNTY.md) and its [targets](docs/CHALLENGE.md)
- [Hall of fame](docs/HALL_OF_FAME.md)
