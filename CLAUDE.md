# Mosaic: project memory

Zero-knowledge password manager. Requirements: `docs/BRIEF.md`. Current state
and next steps: `docs/PROGRESS.md`, which you read first and update at the end
of every milestone.

## Non-negotiables

- **Never invent cryptography.** Use only the primitives and crates in
  `docs/CRYPTO_SPEC.md` §2. The spec is normative: change the spec (plus an
  ADR) first, then the code.
- Secrets live in zeroizing types. They are never logged, never in errors or
  panics, never `Clone`/`Debug`-printable. `#![forbid(unsafe_code)]` in every
  crate.
- Extension: no fill without a trusted user gesture, exact-origin matching,
  no cross-origin iframe fills, content scripts get one credential only
  (brief §4).
- The server sees only what `CRYPTO_SPEC.md` §12 lists.
- No third-party analytics or telemetry anywhere. Never claim "audited" or
  "bank-grade". The README stays labeled pre-audit.
- All UI strings go through `packages/i18n` (DE and EN). All styling goes
  through `packages/design-tokens`.
- The repo holds deliverables only. Scratch work goes in the system temp dir.
- Every new dependency gets a one-line justification in the commit message.
  Crypto dependencies need an ADR.
- Decisions not covered by the brief: take the most conservative security
  option and record an ADR in `docs/DECISIONS.md`.

## Commands (full gate, which must pass before each commit)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo audit --deny warnings
pnpm install --frozen-lockfile
pnpm lint          # eslint, zero warnings
pnpm typecheck
pnpm test          # vitest
pnpm audit
```

Use pnpm, never npm. The Rust toolchain is pinned in `rust-toolchain.toml`.

## Layout

- `crates/core`: crypto, vault model, item types, sync types (CRYPTO_SPEC)
- `crates/{totp,generator,passkey,import}`: feature crates on top of core
- `crates/server`: axum sync server. `crates/cli`: the `mosaic` binary
- `packages/{design-tokens,i18n}`: shared TS for the apps
- `apps/{desktop,extension,mobile}`: added in M3, M4 and M10
- `.claude/agents/`: crypto-reviewer, extension-attacker, sync-chaos,
  ux-reviewer, test-engineer. Run the relevant ones before each milestone
  commit.

## Conventions

- Conventional Commits, one clean commit per milestone (more only when
  splitting helps review).
- Rust: library code returns `Result` with `thiserror` errors. No
  `unwrap`/`expect`/`panic`/`println!` outside tests and the CLI's output
  layer.
- TypeScript: strict, ESM, `.ts` import extensions, no `any`, no
  `console.*`.
