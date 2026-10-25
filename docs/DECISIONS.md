# Architecture Decision Records

Newest last. ADRs are never edited after they are accepted, except to change
their status. To change a decision, add a new ADR that supersedes the old one.

## Template

```markdown
## ADR-NNNN: <title>

- Status: proposed | accepted | superseded by ADR-XXXX
- Date: YYYY-MM-DD
- Milestone: Mx

### Context
What forces are at play, and what problem are we solving?

### Decision
What we do. Be specific enough to implement and review against.

### Alternatives considered
Each option, and why it was rejected.

### Consequences
What gets easier, what gets harder, new risks, and follow-up work.
```

---

## ADR-0001: Product name "Mosaic"

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief used the working name "Tessera". The owner named the product
"Mosaic".

### Decision
Use "Mosaic" everywhere: crate names `mosaic-*`, the CLI binary `mosaic`, npm
scope `@mosaic/*`, and the crypto domain-separation prefix `mosaic/v1/`.
Secret Key display strings start with `M1-` (format version 1).

### Alternatives considered
Keeping "Tessera" internally was rejected. Two names cause confusion, and the
crypto labels become permanent once data exists, so this was the last cheap
moment to rename.

### Consequences
The `mosaic/v1/` labels are frozen once M1 ships. Renaming later would need a
format version bump. Trademark and namespace availability (crates.io, npm,
domains, stores) still has to be checked by the owner. See PROGRESS
"Needs owner".

## ADR-0002: Intermediate account key (AK) between KEK and vault keys

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief has the KEK wrap the vault keys directly. But several unlock
methods (password, OS quick unlock, hardware-key PRF) must each be able to
unlock everything, and a password change must stay cheap.

### Decision
Use a random 256-bit account key AK. Each unlock method wraps AK, and AK wraps
the vault keys and the identity private key (CRYPTO_SPEC §6–§8).

### Alternatives considered
- KEK wraps every VK directly: a password change and every new unlock method
  must re-wrap N vault keys, and quick-unlock methods would need N wrappings
  each.
- Deriving vault keys from KEK: changing the password would then force
  re-encrypting everything. Rejected.

### Consequences
A password change re-wraps exactly one key. One more key layer to review. The
brief's intent ("re-wraps keys, never re-encrypts items") is preserved.

## ADR-0003: OPAQUE input is the two-secret derived OPW, with Identity KSF

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
OPAQUE prevents offline guessing without server compromise. After a server
database compromise, an attacker can still run a dictionary attack on the
OPAQUE password input. The brief requires that a stolen server database yield
nothing brute-forceable.

### Decision
The OPAQUE password input is `OPW = HKDF(R, "mosaic/v1/opaque-password")`,
where `R` combines the Argon2id-stretched master password with the 128-bit
Secret Key. OPAQUE's internal KSF is `Identity` (CRYPTO_SPEC §5.4, §9.1). The
KEK is derived locally from the same root, never from OPAQUE's `export_key`.

### Alternatives considered
- Master password as the OPAQUE input with an Argon2 KSF: brute-forceable
  after a DB compromise, and it does not use the Secret Key. Rejected.
- Deriving the KEK from `export_key`: unlocking would need the server, which
  breaks offline-first. Rejected.
- SRP (as 1Password does): the brief mandates OPAQUE, and OPAQUE has a
  stronger security proof.

### Consequences
Argon2id runs once per unlock, not twice. Changing the master password or
Secret Key requires OPAQUE re-registration. New devices need pre-login
parameters, which are protected against account enumeration by fake responses.

## ADR-0004: HPKE (RFC 9180) Auth mode for sharing and emergency access

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The brief asks for "per-recipient X25519 public-key wrapping". A hand-rolled
X25519 + HKDF + AEAD scheme would be an invented protocol.

### Decision
Use HPKE with DHKEM(X25519, HKDF-SHA256), HKDF-SHA256 and ChaCha20-Poly1305 in
Auth mode, so recipients can verify which account sealed the key.

### Alternatives considered
- Custom ECIES: violates "no invented crypto".
- HPKE Base mode: the server could inject vault keys under an arbitrary sender.
- `age`: built for files. Its X25519 recipients have no sender
  authentication.

### Consequences
Adds the `hpke` crate, which needs vetting in M6. A residual risk remains:
the server is trusted to distribute public keys honestly unless contacts are
verified (THREAT_MODEL R3).

## ADR-0005: No `mlock` for secret memory (for now)

- Status: accepted (revisit in M9)
- Date: 2026-10-25
- Milestone: M0

### Context
Secrets can be paged to swap. Preventing that needs `mlock`/`VirtualLock`,
which means `unsafe` FFI or an extra dependency that uses `unsafe`.

### Decision
Use zeroization and short lifetimes only. Do not lock memory pages in M1–M8.
Keep core crates `#![forbid(unsafe_code)]`.

### Alternatives considered
The `memsec`/`region` crates: they add `unsafe` dependencies, and locked-page
limits differ by platform.

### Consequences
Residual risk R5. In M9, re-evaluate locking a small, fixed-size secret arena
inside a separately audited crate.

## ADR-0006: Shared TypeScript packages and pnpm

- Status: accepted
- Date: 2026-10-25
- Milestone: M0

### Context
The desktop app and the extension must share UI strings (DE/EN from day one)
and design tokens. The owner's tooling standard is pnpm.

### Decision
A pnpm workspace (`apps/*`, `packages/*`) with `packages/design-tokens`
(tokens and a WCAG contrast test) and `packages/i18n` (DE/EN ICU catalogs and
a parity test). The ICU formatter library is chosen in M3. Dependency
lifecycle scripts are disabled by default (`onlyBuiltDependencies: []`).

TypeScript is pinned to 6.0.x because typescript-eslint 8.x does not support
TypeScript 7 yet. Revisit when it does.

### Alternatives considered
- Duplicating tokens and strings per app: they drift.
- npm or yarn: the owner's standard is pnpm, and pnpm's strict
  `node_modules` layout reduces phantom-dependency risk.

### Consequences
`packages/` is a top-level directory not listed in the brief's suggested
layout.
