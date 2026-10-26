# Progress

Resume from here in every session. Requirements: `docs/BRIEF.md`. Decisions:
`docs/DECISIONS.md`.

## Status

| Milestone | Status |
|---|---|
| M0 Foundations | ✅ done (2026-10-25) |
| M1 Crypto core | 🔄 implemented, review findings addressed (step 10), not yet committed |
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

## M1 Crypto core — implementation state (2026-10-25)

Steps 1–10 of the plan are implemented, and the step 10 review findings are
fixed or accepted (see below). M1 is done once it is committed and CI is green.

- `crates/core/src/`: `error` (one `thiserror` enum; every authentication or
  padding failure is `Error::Decryption`), `rng`, `secret` (zeroizing
  containers and typed keys AK, VK, KEK/W, OPW, FK, SK), `ids` (UUIDv7, nil
  rejected, `KeyGeneration`), `encoding` (`ctx`, `Aad` builder with header
  binding, strict ISO/IEC 7816-4 pad/unpad), `kdf` (NFC, the §5.3 profile
  table, `stretch` with a fallibly reserved and zeroized Argon2 buffer,
  injectable calibration plus a real-clock adapter, the §5.4 HKDF chain),
  `secret_key` (`M1-` Crockford format, checksum, aliases, strict pad bits,
  last-group check), `aead` (crate-private XChaCha20-Poly1305, random
  nonces only), `envelope` (`WrappedKey`, `ItemCiphertext`,
  `VaultMetaCiphertext`, `TombstoneCiphertext`), `attachment` (STREAM-BE32
  via `aead-stream`, 64 KiB chunks, 50 MiB limit), `keys` (AK/VK generation
  and wrapping, `create_account`, `unlock_with_password`,
  `change_master_password`).
- Tests: KATs for RFC 9106 Argon2id, draft-irtf-cfrg-xchacha A.3.1 and
  RFC 5869 cases 1–3. Proptests for round trips, single-bit tampering and
  wrong context for every object, pad/unpad, and Secret Key round trip and
  typos (exhaustive single-substitution detection ≥ 99 %). Golden files in
  `crates/core/tests/golden/` (regenerate with `MOSAIC_BLESS=1`, review the
  diff). A password-change test shows that item ciphertexts are byte-for-byte
  unchanged. Only `tests/baseline_profile.rs` runs a real profile; everything
  else uses 64 KiB test parameters. The 50 MiB attachment round trip is
  `#[ignore]` (slow in debug builds); run it with
  `cargo test --release -p mosaic-core -- --ignored`.
- Coverage (`cargo llvm-cov -p mosaic-core`): **98.5 % lines** (97.0 %
  regions) after the step 10 fixes. The number includes the in-file
  unit-test modules.
- Fuzzing: `crates/core/fuzz` (its own workspace, excluded from the main one)
  has the targets `secret_key_parse`, `wrapped_key` (decode and open),
  `item_ciphertext` (item, vault metadata, tombstone), `attachment_open` and
  `unpad`. Each one also round-trips sealed data. Native Windows MSVC does
  not work: the VS 2019 ASan runtime does not match nightly's LLVM
  (`STATUS_DLL_INIT_FAILED`), and `-s none` fails to link the sancov
  symbols. The 60 s runs used Docker (`rustlang/rust:nightly`, Linux,
  ASan). Every target ran without a crash: `secret_key_parse` 6.7 M
  executions, `wrapped_key` 0.79 M, `item_ciphertext` 0.28 M,
  `attachment_open` 0.35 M, `unpad` 2.6 M. Keep the corpus off Windows bind
  mounts, which cut throughput by about 1000×. The runs found one
  performance issue: every attachment open reserved and zeroized a full
  64 KiB chunk buffer, even for tiny inputs. The buffer is now sized to the
  data. After the step 10 rework (fixed golden keys, tamper and structured
  modes, committed seeds) every target ran 60 s in the same Docker setup
  without a crash: `secret_key_parse` 2.2 M executions, `wrapped_key`
  0.47 M, `item_ciphertext` 0.58 M, `attachment_open` 65 k (with
  `-max_len=270000` and multi-chunk structured inputs) and `unpad` 3.5 M.
  Run with `cargo +nightly fuzz run <t> corpus/<t> seeds/<t>`; the repo's
  `rust-toolchain.toml` otherwise selects stable.
- CI: new `coverage` job (fails below 90 % lines on mosaic-core) and
  `fuzz-smoke` job (pinned nightly, every target from `cargo fuzz list` for
  60 s, seeded).
- Spec clarifications plus ADR-0007 (accepted with amendments in the M1
  review, 2026-10-26): size limits (item 1 MiB padded, vault metadata 64 KiB padded,
  tombstone 64 KiB), the tombstone header, the AAD header of attachments and
  their canonical chunking, strict unpadding, the error mapping
  (`Decryption` in core, `ItemCorrupted` in M2), how calibration reads the
  thresholds, sequential Argon2 lanes, Secret Key prefix rules, and the
  zeroize features.
- Dependency changes: M0 had no Rust dependencies, so the whole
  `[workspace.dependencies]` table is new in M1 (about 50 packages in
  `Cargo.lock`, mostly RustCrypto internals). Direct dependencies of
  `mosaic-core`, each pinned with `=` and with default features off:
  `argon2 =0.6.0` (Argon2id, §5.3), `hkdf =0.13.0` and `sha2 =0.11.1`
  (HKDF-SHA-256, checksums, §5.2–§5.4), `chacha20poly1305 =0.11.0`
  (XChaCha20-Poly1305, §6–§7), `aead-stream =0.6.0` (STREAM, which §7.3
  names; `aead` 0.6 no longer has `aead::stream`), `zeroize =1.9.1` (§4),
  `subtle =2.6.1` (constant-time compare and select), `getrandom =0.4.3`
  (OS CSPRNG), `unicode-normalization =0.1.26` (NFC, §5.1) and
  `thiserror =2.0.21` (derives the error enum; build-time proc macro).
  `chacha20poly1305` and `sha2` enable `zeroize`. Dev-only: `proptest
  =1.12.0`, `hex-literal =1.1.0`. Fuzz-only (own lockfile, never linked into
  Mosaic): `libfuzzer-sys =0.4.10`. The M1 plan also listed `secrecy` and
  `getrandom`/`rand_core`: `secrecy` was dropped because `secret` already
  provides the containers (zeroize on drop, no `Clone`, redacted `Debug`,
  constant-time equality, boxed fixed-size keys), and a direct `rand_core`
  was dropped because `getrandom` is used directly. The full justification
  is ADR-0007 item 14 and goes into the M1 commit message. Crypto crates
  build at `opt-level = 3` in dev and test profiles.
- Known gaps: attachments are sealed and opened in memory only (no streaming
  API yet). Argon2 runs without the `parallel` feature, so the 4 lanes run
  sequentially (ADR-0007 item 7). NFC normalization leaves short-lived copies
  inside `unicode-normalization` we cannot zeroize. Our HKDF code zeroizes
  the PRK it receives; `hmac`'s key-pad block and `hkdf`'s last output block
  stay on the stack upstream (THREAT_MODEL R11). Checking
  `key_generation >= highest accepted` per vault is client state and belongs
  to M2. Identity keys, HPKE, OPAQUE and quick unlock are later milestones.

### M1 review (step 10), 2026-10-26

crypto-reviewer, test-engineer and a gate review reported 2 high, 7 medium
and 11 low findings. All were fixed with regression tests, except the ones
in the table below:

- Crypto: a password change or Secret Key rotation now takes the current
  `PasswordParams` and refuses a weaker profile (`Error::ProfileDowngrade`,
  §5.3). The HKDF PRK is extracted and zeroized explicitly. Secret Key
  symbol mapping is branch-free and lookup-free (`subtle` selects and range
  arithmetic), checked against a table-based reference for every value and
  for every character up to U+07FF. `Aad` and `Label` are crate-private and
  labels are checked for NUL at compile time (no `debug_assert`).
- Tests: authentic-but-badly-padded item and vault-metadata bodies fail with
  `Decryption` (verified by mutating the error mapping). STREAM output is
  compared chunk by chunk with an independent per-chunk XChaCha20-Poly1305
  construction (`attachment::reference`), golden attachments open through
  it, and `golden/attachment_multi.hex` pins two multi-chunk files. Every
  tamper and wrong-context check asserts the exact error. Cross-type
  decoding (item, metadata, tombstone) now really parses and fails with
  `Decryption`. The decoder property test draws valid headers and boundary
  lengths, and a coverage test shows every decoder reaches `open`. Strict
  decoding covers meta and tombstone headers, sub-minimum and misaligned
  bodies and the exact size limit. NFC has a proptest (equivalence,
  idempotence, no reallocation) plus worst-case expansions. The baseline
  profile has a vector computed independently with argon2-cffi and a
  stdlib HKDF (`tests/vectors/derive_keys_baseline.py`, which also
  reproduces `golden/derived_keys.hex`).
- Fuzzing: fixed (golden) keys, tamper modes on authentic golden objects,
  a structured multi-chunk attachment mode, a `matches_last_group`
  reference check, committed seeds in `crates/core/fuzz/seeds/`, and CI
  running every target from `cargo fuzz list` with the seeds and
  `-max_len=270000` for attachments.
- CI and supply chain: pinned `cargo-llvm-cov@0.9.1`, `cargo-fuzz@0.13.2`,
  `cargo-audit@0.22.2` and `nightly-2026-10-25`; `cargo deny` and
  `cargo audit` also run on the fuzz lockfile (NCSA allowed for
  `libfuzzer-sys` only, ADR-0007 item 10).

Follow-up review round (crypto, tests, gate), 2026-10-26: 3 medium and 6
low findings, all fixed with regression tests, each checked by mutating
the code it guards:

- Unpadding of item and vault-metadata plaintext is constant-time in the
  contents (`subtle` selects over the whole buffer, one final branch) and
  is compared against the old branching version at every marker position
  and on random inputs (CRYPTO_SPEC §7.2, ADR-0007 item 4).
- Every public seal path (item, wrapped AK, wrapped VK, vault metadata,
  tombstone, attachment nonce prefix) is checked for a fresh, non-zero
  nonce. Before, only items were, so a constant nonce in the others would
  have passed every test.
- Size limits are tested at exactly the limit and one above it for
  tombstone seal and decode and vault-metadata seal. A maximum-size
  attachment ciphertext is checked cheaply in every build, and CI now
  runs `cargo test --release -p mosaic-core -- --ignored` (the 50 MiB
  round trip).
- The Argon2 memory reservation is a separate function whose failure path
  (`Error::Allocation`) is tested. Buffers that must not reallocate
  (Secret Key display and last group, attachment and AEAD plaintext) are
  checked by capacity, through `#[cfg(test)]` `capacity()` accessors.
- Secret Key prefix leniency (`ml`, `MI`, `M-1`, ...) is pinned, along
  with the rejected prefixes `M2`, `X1`, `N1` and `MO`.
- ADR-0007 is accepted (with the two amendments listed in its status).
- The fuzz crate builds only with `--cfg fuzzing`. A plain `cargo check`
  now stops with a `compile_error!` that explains this, instead of only
  E0599. `cargo fuzz build` and CI are unaffected.

## Open risks / findings

| ID | Severity | Item | Status |
|---|---|---|---|
| M1-R1 | low | Upstream `hmac`/`hkdf` stack residue (key-pad block, last HKDF output block) during derivation | accepted, THREAT_MODEL R11, ADR-0005 |
| M1-R2 | low | Secret Key parsing still branches on whether a character is `-`/whitespace and on the final validity flag. Neither reveals the key: separators carry no key bits and invalid input is rejected | accepted |
| M1-R3 | low | The independent baseline vector is checked in, but CI does not re-run the Python script (it needs argon2-cffi and 128 MiB). Re-run it when the derivation changes | accepted |
| M1-R4 | low | Fuzzing cannot run natively on Windows MSVC (ASan runtime mismatch); local runs use Docker, CI runs on Linux | accepted |
| M1-R5 | info | The main-workspace `cargo deny check` warns `license-exception-not-encountered` for the `libfuzzer-sys` NCSA exception, which only applies to the fuzz lockfile | accepted |
| M1-R6 | low | Constant-time unpadding is best-effort at the source level (`subtle` selects, no early exit). The compiler gives no hard guarantee. Code that later processes the unpadded plaintext (M2 item parsing) takes time proportional to its length. Only a local attacker who can time client-side decryption could exploit this | accepted |

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
