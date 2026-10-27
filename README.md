# Mosaic

> **Pre-audit software.** Mosaic has not had an external security audit yet.
> Do not use it for real secrets until an audit is complete and published.

Mosaic is a zero-knowledge password manager: desktop app, browser extension,
self-hostable sync server, CLI and (later) mobile apps. They share one Rust
crypto core. Your master password and a 128-bit Secret Key, which never leaves
your devices, together unlock your vault. The sync server only ever stores
ciphertext.

**Status:** foundations (M0). There is nothing usable yet. See
[`docs/PROGRESS.md`](docs/PROGRESS.md).

## Documents

- [Threat model](docs/THREAT_MODEL.md)
- [Cryptography specification](docs/CRYPTO_SPEC.md)
- [UX guidelines](docs/UX_GUIDELINES.md)
- [Desktop IPC contract](docs/DESKTOP_IPC.md)
- [Architecture decisions](docs/DECISIONS.md)
- [Build brief](docs/BRIEF.md)

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
