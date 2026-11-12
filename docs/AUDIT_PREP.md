# Audit preparation

Mosaic is pre-audit software. No outside firm has examined it. This page is
for the reviewer who will: a firm, a researcher or a contest player. It says
what to look at, in what order, and how to build and run everything.

It is written so that you do not have to trust us. Every claim points at a
document or a command.

## Read first

1. [`THREAT_MODEL.md`](THREAT_MODEL.md): assets, trust boundaries (section 2),
   attackers (section 3), the rows per component (section 4) and the
   accepted risks (section 5).
2. [`CRYPTO_SPEC.md`](CRYPTO_SPEC.md): normative. The code follows it. Where
   they differ, that is a finding. Section 2 lists every allowed primitive and
   crate. Section 12 lists what the server can see.
3. [`DECISIONS.md`](DECISIONS.md): why each choice was made.

## Scope

Trust boundaries are in the threat model, section 2. Untrusted: web pages,
the network, the sync server, other extensions, import files, and anything
received over IPC before sign-in. Trusted: the operating system and the user
session (with the limits of attacker P5), the browser's extension isolation,
the Rust core and the platform keystores.

| Component | Where | Look at |
|---|---|---|
| Crypto core | `crates/core` | keys, envelopes, KDFs, secret handling, the export format |
| Vault and storage | `crates/vault` | sessions, the local store, item merge and split, folder MACs |
| Sync client | `crates/sync` | the engine, the HTTP client, freeze and rollback detection, sharing with HPKE |
| Sync server | `crates/server` | OPAQUE login, the WebAuthn relying party, records, sharing, limits |
| Passkeys | `crates/passkey` | the software authenticator (ES256) |
| Feature crates | `crates/{totp,generator,import,insights,ssh-agent,qr}` | parsers (import, QR), one-time codes, generator |
| Origin rules | `crates/origin` | exact-origin matching, look-alike and IDN checks |
| Native messaging | `crates/native-messaging` | the host `mosaic-native-host` and the channel to the extension |
| CLI | `crates/cli` | the session cache, secrets on the command line, `run` |
| Desktop app | `apps/desktop` | the Tauri IPC surface ([`DESKTOP_IPC.md`](DESKTOP_IPC.md)) |
| Browser extension | `apps/extension` | the fill gate and its tests ([`EXTENSION_SECURITY.md`](EXTENSION_SECURITY.md)) |
| Build and release | `.github/workflows`, `scripts/release` | signing, checksums, reproducibility |

Not in scope: the mobile apps (not built yet), the website, and the risks of
section 5 of the threat model that we accept.

## Where we would look first

1. **The OPAQUE integration.** The server in `crates/server`, the client in
   `crates/sync`. The identities that bind a login to the server's origin. The
   fake answers for unknown accounts.
2. **Item merge and split.** How two edits of one item merge, and how a merge
   that grows too large is split. A bug here loses data or leaks it.
3. **Folder MACs and freeze detection.** How a device notices that a server or
   a cloud folder shows it old data or different data than to another device.
4. **Sharing with HPKE.** Invites, key seals, the roster, removal and
   re-keying.
5. **The extension fill gate and the native-messaging channel.** No fill
   without a user click, exact origin, no cross-origin frame fills, one
   credential per content script.
6. **The WebAuthn relying party.** The second factor of the sync account:
   challenges, origin and RP ID checks, counters, the browser handoff page.
7. **The CLI session cache.** What stays on disk after unlock, and who can
   read it.
8. **The export format.** `.mosaicx` files and the passphrase protection.
9. **The release pipeline.** Who can change what ends up in a download, and how
   a user checks it.

## Build

The toolchain is pinned in `rust-toolchain.toml`. Use `pnpm`, not `npm`.

```sh
cargo build --workspace
pnpm install --frozen-lockfile
```

## Test

The full gate, which must pass before each commit:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo audit --deny warnings
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test
pnpm audit
```

The extension has browser tests with Playwright (`pnpm --filter` the
extension package, script `test:e2e`). The hostile pages are in
`apps/extension`.

Slow tests: the server tests derive keys with Argon2id, which needs a few
hundred megabytes of memory per run. Run one test binary at a time on a small
machine.

## Fuzz

Every parser and decoder has a fuzz target. Each crate that has one keeps it
in its own `fuzz/` directory, outside the main workspace: `crates/core`,
`generator`, `import`, `insights`, `native-messaging`, `passkey`, `qr`,
`server`, `ssh-agent`, `sync`, `totp` and `vault`. It needs `cargo-fuzz` and a
nightly toolchain:

```sh
cd crates/core/fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run <target>
```

Seeds are committed next to the targets.

## Reproduce a release

The release steps, the checksums, the signature check and the build notes are
in `docs/VERIFY.md`, added with the release workflow. In short: build twice in
different directories, compare the hashes, and check the signed
`SHA256SUMS`. The desktop installers are not reproducible, and the notes say
why. Software bills of materials (CycloneDX) come with each release.

## Residual risks

[`THREAT_MODEL.md`](THREAT_MODEL.md) section 5 is the register of risks we know
and accept, or have not fixed. It runs from R1 to R123. Each row says what the
risk is and why it stands. Findings that fall inside a row are out of scope for
the challenge unless they go beyond it ([`BUG_BOUNTY.md`](BUG_BOUNTY.md)).

Open items of each milestone are in [`PROGRESS.md`](PROGRESS.md).

## Internal review, not independent

[`SECURITY_REVIEW_LOG.md`](SECURITY_REVIEW_LOG.md) lists every internal
review: who did it, what it found and what we did. The reviewers were AI
agents working under human direction. The log is a ledger of our own work. It
does not replace an outside review, and it must not be read as one.

## The challenge

The test server, its accounts and the rules are in
[`BUG_BOUNTY.md`](BUG_BOUNTY.md) and [`CHALLENGE.md`](CHALLENGE.md). The
seeder that builds the accounts is `crates/server/examples/contest_seed.rs`,
so you can run the same set-up yourself.

## Contact

Report problems through [`SECURITY.md`](../SECURITY.md): GitHub private
vulnerability reporting. An email address will be added when the project has a
domain.
