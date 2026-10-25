# Progress

Resume from here in every session. Requirements: `docs/BRIEF.md`. Decisions:
`docs/DECISIONS.md`.

## Status

| Milestone | Status |
|---|---|
| M0 Foundations | ✅ done (2026-10-25) |
| M1 Crypto core | ⏭ next |
| M2 Vault model + local store | — |
| M3 Desktop app | — |
| M4 Browser extension | — |
| M5 Passkeys | — |
| M6 Sync server | — |
| M7 Rotation | — |
| M8 CLI | — |
| M9 Hardening | — |
| M10 Mobile | — |

## M0 Foundations — done

- Cargo workspace (`crates/*`, Rust 1.99.1 pinned in `rust-toolchain.toml`,
  workspace lints: `unsafe_code = forbid`, clippy pedantic plus
  no-unwrap/panic/print in library code, release `panic = "abort"`).
  The crates are scaffolds that state the milestone they land in.
- pnpm workspace with `packages/design-tokens` (WCAG contrast tests for both
  themes) and `packages/i18n` (DE/EN catalogs, key and ICU-argument parity
  tests).
- Supply chain: `deny.toml` (advisories, licenses, bans, sources), lockfiles,
  dependency lifecycle scripts disabled, Dependabot, GitHub Actions pinned by
  SHA with read-only permissions.
- CI (`.github/workflows/ci.yml`): Rust fmt/clippy/test and TS
  lint/typecheck/test on Linux, Windows and macOS. `cargo deny`, `cargo audit`
  and `pnpm audit`.
- Docs: `THREAT_MODEL.md`, `CRYPTO_SPEC.md`, `UX_GUIDELINES.md`,
  `DECISIONS.md` (template plus ADR-0001…0006), `CLAUDE.md`, README.
- Subagents in `.claude/agents/`: crypto-reviewer, extension-attacker,
  sync-chaos, ux-reviewer, test-engineer.
- Design review by crypto-reviewer: 3 high, 7 medium, 3 low findings, all
  addressed in CRYPTO_SPEC (key generations against rotation rollback, an
  honest description of rotation plus a full re-key flow, profile-based
  Argon2 parameters with enumeration-resistant pre-login, header-bound AAD,
  per-platform quick unlock, 130-bit safety codes, OPAQUE re-registration
  authorization). Residual items are THREAT_MODEL R7–R10.

Gate run locally on Windows: all green. CI runs on GitHub
(`StephanBaum/Mosaik`).

## M1 Crypto core — plan

Scope (CRYPTO_SPEC §3–§7, §14):
1. `mosaic-core::secret`: zeroizing secret types with redacted `Debug` and no
   `Clone`.
2. `mosaic-core::encoding`: `ctx()` / `aad()` builders, envelope header,
   padding.
3. `mosaic-core::kdf`: NFC preparation, Argon2id profiles (table, calibration,
   calibration), HKDF derivations (`S`, `R`, `KEK`, `OPW`).
4. `mosaic-core::secret_key`: generation, `M1-` Crockford format with
   checksum, parsing with aliases.
5. `mosaic-core::aead`: XChaCha20-Poly1305 envelopes for wrapped keys, items
   and vault metadata. STREAM for attachments.
6. `mosaic-core::keys`: AK/VK generation, wrap and unwrap, password change.
7. Tests: KATs (RFC 9106, XChaCha draft, RFC 5869), proptest round-trip and
   tamper tests, golden files.
8. `crates/core/fuzz`: decoders for envelope, secret key and wrapped key.
9. CI: add `cargo llvm-cov` (≥ 90 % on core) and a bounded fuzz smoke job.
10. Review: crypto-reviewer and test-engineer, then fix or accept findings.

New dependencies to justify in the commit: `argon2`, `hkdf`, `sha2`,
`chacha20poly1305`, `zeroize`, `secrecy`, `subtle`, `getrandom`/`rand_core`,
`unicode-normalization`, `thiserror`, and (dev) `proptest`, `hex-literal`.

Risks: Argon2 calibration flakiness in CI (calibration takes an injected
clock, and tests use fixed parameters). Windows/macOS differences in
`getrandom`: none expected.

## Open risks / findings

| ID | Severity | Item | Status |
|---|---|---|---|
| — | — | No open findings | — |

See the THREAT_MODEL §5 residual-risk register.

## Needs owner

- **License**: choose a project license (for example AGPL-3.0 for the server
  and Apache-2.0/MIT for clients, or a single license). Crates are
  `publish = false` until decided.
- **Name availability**: "Mosaic" on crates.io / npm / domains / app stores,
  and a trademark check.
- **Signing**: Apple Developer ID, Windows code-signing certificate, extension
  store accounts (Chrome Web Store, Edge Add-ons, AMO). Needed from M3/M4 and
  for releases in M9.
