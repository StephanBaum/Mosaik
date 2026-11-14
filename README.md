# Mosaic

> **Pre-audit software.** Mosaic has not had an external security audit yet.
> Do not use it for real secrets until an audit is complete and published.

Mosaic is a zero-knowledge password manager: desktop app, browser extension,
self-hostable sync server, CLI and (later) mobile apps. They share one Rust
crypto core. Your master password and a 128-bit Secret Key, which never leaves
your devices, together unlock your vault. The sync server only ever stores
ciphertext.

**Status:** M3 desktop app, M4 browser extension, M5 passkeys and M6 sync built. Onboarding with the Emergency Kit,
the vault, the generator, one-time codes, import, the offline security report, the SSH agent
and quick unlock run in the desktop app. The browser extension (Chromium and Firefox, a
toolbar popup that fills one login after a click, no inline UI) pairs with the desktop over
a local, authenticated channel; it is a development build, not yet in any store. Mosaic also
stores passkeys (ES256) and answers passkey requests in the browser after you confirm each one
in the desktop app; passkeys can be imported (Bitwarden, KeePassXC, FIDO CXF) and exported as a
CXF file. Sync runs through a folder in your own cloud storage or the self-hostable
server; it is off by default and unreleased. See
[`docs/PROGRESS.md`](docs/PROGRESS.md).

## Documents

- [Try a preview build](docs/TRY_IT.md) (unsigned Windows, Chrome and Edge, Android; for trying out only)
- [Threat model](docs/THREAT_MODEL.md)
- [Cryptography specification](docs/CRYPTO_SPEC.md)
- [UX guidelines](docs/UX_GUIDELINES.md)
- [Desktop IPC contract](docs/DESKTOP_IPC.md)
- [Browser extension security](docs/EXTENSION_SECURITY.md)
- [Security review log](docs/SECURITY_REVIEW_LOG.md) (internal, AI-assisted review; not an independent audit)
- [Security policy](SECURITY.md) and [audit preparation](docs/AUDIT_PREP.md)
- [Security challenge](docs/BUG_BOUNTY.md), its [targets](docs/CHALLENGE.md) and the [hall of fame](docs/HALL_OF_FAME.md)
- [Architecture decisions](docs/DECISIONS.md)
- [Build brief](docs/BRIEF.md)

## Command line

`mosaic` is a command-line client that opens a vault store directly (no running app needed).
It never takes a secret from the command line or the environment and prints one only with
`--reveal`. Pre-audit like the rest. Commands, output formats and exit codes are in
[`docs/CLI.md`](docs/CLI.md).

```sh
cargo build -p mosaic-cli --release     # target/release/mosaic
mosaic unlock --ttl 10m                 # password prompt; a short session
mosaic list
mosaic get mosaic://Personal/GitHub --field password --copy
mosaic run --env DB_PASSWORD=mosaic://Work/db/password -- ./migrate
mosaic setup own-store --server https://sync.example.org --email you@example.org
mosaic sync
```

A man page (`mosaic help --man`, committed as `crates/cli/man/mosaic.1`) and completion scripts for
bash, zsh, fish and PowerShell (`mosaic completions <shell>`, committed in `crates/cli/completions/`)
are generated from the command table. Signing, installers and package-manager channels come with
the release milestone.

## Account recovery

There is no backdoor. If you lose both your master password and your
Emergency Kit (which contains your Secret Key), nobody can recover your data,
including the Mosaic developers and your server operator. Emergency access to
a trusted contact can be set up in advance.

## Development

Requirements: Rust (installed automatically from `rust-toolchain.toml` by
rustup), Node 22+, and pnpm 9.

```sh
pnpm install
cargo test --workspace && pnpm check
```

The full quality gate is listed in [`CLAUDE.md`](CLAUDE.md).

Desktop app (`apps/desktop`, Tauri v2; on Linux it needs the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)):

```sh
pnpm --filter @mosaic/desktop dev:mock   # UI in the browser with the mock IPC
pnpm --filter @mosaic/desktop tauri dev  # the real app
pnpm test:e2e                            # Playwright + axe (after `playwright install chromium`)
```
