# Dependencies

Inventory of every direct production dependency, with the reason and the ADR that admits it, plus the size of
each shipped binary's dependency tree. Policy: ADR-0086. Crypto crates are fixed by
[CRYPTO_SPEC.md §2](CRYPTO_SPEC.md) and never change without an ADR. Dev-dependencies are not listed.

All versions are exact (`=`) pins in the root `Cargo.toml` `[workspace.dependencies]` (Rust) or in the
`package.json` files (JS, no ranges). Default features are off for every crate that has any worth turning off;
the entries that keep defaults (`lexopt`, `rpassword`, `ctrlc`, `open`, `qrcodegen`, `libc`,
`rustls-platform-verifier`) have no default features, or only `std`. `deny.toml` enforces the license list and
the bans.

## Rust: crypto (CRYPTO_SPEC §2; ADR-0007 unless noted)

| Crate | Used by | Reason |
|---|---|---|
| `argon2` | core, import | Argon2id key derivation; Argon2d/id for KDBX import (ADR-0021) |
| `hkdf`, `hmac`, `sha2`, `sha1` | core, server, sync, totp, insights, import, passkey, cli, desktop | HKDF-SHA-256, HMAC, hashes, OTP (SHA-1/256/512), breach-check prefix (ADR-0008, ADR-0067) |
| `chacha20poly1305`, `aead-stream` | core, server | XChaCha20-Poly1305 and its STREAM mode for attachments |
| `hpke` | core | HPKE mode Auth for shares and emergency access (ADR-0004, ADR-0049) |
| `opaque-ke`, `sha2_0_10` (`sha2` 0.10) | sync | OPAQUE login; opaque-ke 4.0 is written against digest 0.10 (ADR-0046) |
| `p256` | passkey | ES256 passkeys (ADR-0039) |
| `passkey-auth` | server | WebAuthn relying party for the sync-account second factor (ADR-0078) |
| `ssh-key`, `ed25519-dalek`, `bcrypt-pbkdf` | ssh-agent | OpenSSH keys and Ed25519 / P-256 signing; the last two only turn on `zeroize` in the tree (ADR-0025) |
| `aes`, `cbc`, `chacha20`, `salsa20` | import | KDBX 4 read-only primitives, never for Mosaic data (ADR-0021) |
| `getrandom` | core, generator, sync, server, desktop | OS CSPRNG, called directly (ADR-0011, ADR-0049) |
| `zeroize`, `subtle` | most crates | Zeroizing secrets, constant-time compare |
| `rustls` (`ring` provider), `rustls-platform-verifier` | sync | TLS 1.3 client, OS trust store (ADR-0046) |
| `unicode-normalization` | core, sync, cli | NFC of passwords and emails |
| `base64ct` | core, import, passkey, platform, server, sync, vault, ssh-agent | Constant-time base64 / base64url |

## Rust: data, formats and tooling

| Crate | Used by | Reason | ADR |
|---|---|---|---|
| `serde`, `serde_json` | most crates | Item model, wire types, IPC | ADR-0009 |
| `thiserror` | all library crates | Error types | ADR-0007 |
| `rusqlite` (bundled SQLite) | vault, server | Local store and server database | ADR-0010, ADR-0044 |
| `flate2`, `quick-xml`, `zip` | import | gzip, KDBX XML, 1PUX archives | ADR-0021 |
| `psl`, `url`, `idna` | origin, insights, sync | Exact-origin matching, IDN and look-alike checks | ADR-0037 |
| `image`, `zune-jpeg`, `zune-core`, `rqrr`, `qrcodegen` | qr | PNG/JPEG decode and QR decode for 2FA setup; `qrcodegen` writes test fixtures only | ADR-0031 |
| `tokio` | native-messaging, ssh-agent, server, desktop | Async I/O for the host, the agent and the server | ADR-0025, ADR-0036, ADR-0044 |
| `axum`, `tower`, `tower-http` | server | HTTP server, body limit, timeout, panic catching | ADR-0044 |
| `reqwest` (blocking, rustls) | sync | HTTP client of the sync transport | ADR-0046 |
| `toml` | server | Strict server config | ADR-0044 |
| `tracing`, `tracing-subscriber` (`fmt`, `std`) | server | Request logs without bodies | ADR-0044 |
| `libc` | sync (unix) | `O_NONBLOCK` for folder-backend files | ADR-0043 |
| `lexopt`, `rpassword`, `ctrlc`, `rustix` (unix), `winapi-util` (windows) | cli | Argv, no-echo prompt, Ctrl-C, terminal mode | ADR-0074, ADR-0086 |

## Rust: desktop shell (apps/desktop/src-tauri)

| Crate | Reason | ADR |
|---|---|---|
| `tauri`, `tauri-build`, `tauri-plugin-single-instance`, `tauri-plugin-global-shortcut` | App shell; no default features | ADR-0019, ADR-0020 |
| `arboard`, `keyring` | Clipboard with history exclusion; Secret Key in the OS keystore | ADR-0020 |
| `rfd` | Native save dialog (Emergency Kit) | ADR-0020 |
| `pdf-writer` | Emergency Kit PDF | ADR-0020 |
| `windows`, `security-framework`, `security-framework-sys`, `zbus` | Windows Hello, Touch ID, Linux lock signals | ADR-0028, ADR-0020 |
| `win-screenshot`, `x11rb` | "Scan screen" for QR codes | ADR-0031 |
| `open` | Opens the browser for the WebAuthn handoff | ADR-0077 |

## Mobile (M10; planned, added by the tracks named)

Rust, in `crates/mobile-ffi` only (`mosaic-mobile` adds no new crate; it uses the workspace crates):

| Crate | Reason | ADR |
|---|---|---|
| `uniffi` `=0.32.2` (default features off; `cli` only behind the `bindgen` feature of the `uniffi-bindgen` binary) | Generates the Kotlin and Swift bindings and the FFI scaffolding of the mobile library. Not a crypto dependency. MPL-2.0, with per-crate exceptions in `deny.toml` for its eight crates; the bindgen-only `basic-toml` advisory (RUSTSEC-2026-0374) is ignored with a reason (T3b) | ADR-0089 |
| `zeroizing-alloc` `=0.1.1` | Global allocator of the mobile library: wipes every freed Rust heap block, including UniFFI's argument and return buffers. About 100 lines, `no_std`, no dependencies, MIT OR Apache-2.0 (T3b) | ADR-0089 |

`rustls-platform-verifier` moves to a `cfg(not(target_os = "android"))` dependency of `mosaic-sync`; Android uses
`rustls`'s own `WebPkiServerVerifier` (no new crate, ADR-0091). `qrcodegen` moves from a dev-dependency to a
desktop dependency for the setup code (T13, ADR-0096).

Android (`apps/mobile/android`, T5 and later). Every artifact is pinned in the Gradle version catalog and
checked by Gradle dependency verification. The release classpath must match
`apps/mobile/android/dependency-allowlist.txt`.

| Dependency | Reason |
|---|---|
| `net.java.dev.jna:jna` (`@aar`; the plain jar for JVM tests) | UniFFI's Kotlin bindings call the native library through JNA |
| `androidx.core:core-ktx`, `androidx.activity:activity-compose` | The activity host for Compose |
| Compose BOM: `ui`, `foundation`, `material3`; `ui-tooling` in debug only | The UI toolkit (ADR-0088) |
| `androidx.lifecycle:lifecycle-{runtime-compose,viewmodel-compose,process}` | View models; `ProcessLifecycleOwner` for the lock on background |
| `androidx.biometric:biometric` | `BiometricPrompt` with `CryptoObject` and `BIOMETRIC_STRONG`, also on API 28 and 29 |
| `androidx.credentials:credentials` | Provider request and response types (`PendingIntentHandler`, `CallingAppInfo`) on API 34+ |
| `androidx.camera:camera-{core,camera2,lifecycle,view}` | The QR scanner preview and luma frames; decoding stays in Rust |
| `org.jetbrains.kotlinx:kotlinx-coroutines-android` | The serial core dispatcher |
| Tests only: `junit:junit`, `kotlinx-coroutines-test`, `androidx.test:{runner,rules,ext-junit}`, `androidx.test.uiautomator:uiautomator`, `androidx.compose.ui:ui-test-junit4` | Unit, instrumented and UI tests |

There is no Firebase, no Google Play services, no ML Kit, no OkHttp, no analytics and no crash reporting
(`scripts/mobile/check-android-deps.sh`).

iOS (`apps/mobile/ios`): no third-party dependency. There are no Swift packages from outside the repo and no
CocoaPods (`scripts/mobile/check-ios-deps.sh`). XcodeGen is a CI build tool, pinned by a SHA-256 of its release
zip.

## JS production dependencies

| Package | Used by | Reason | ADR |
|---|---|---|---|
| `react`, `react-dom` | extension, desktop | UI | ADR-0019, ADR-0035 |
| `@fontsource/dm-sans`, `@fontsource/ibm-plex-mono` | extension, desktop | Self-hosted fonts, no font CDN | ADR-0019 |
| `@tauri-apps/api` | desktop | IPC bridge | ADR-0019 |
| `@mosaic/design-tokens`, `@mosaic/i18n` | extension, desktop | Workspace packages | ADR-0006 |
| `intl-messageformat` (with `@formatjs/fast-memoize`, `@formatjs/icu-messageformat-parser`, `@formatjs/icu-skeleton-parser`) and `scheduler` | through `@mosaic/i18n`, `react-dom` | ICU message formatting (DE and EN plurals); React's scheduler | ADR-0006 |

## Size of the shipped binaries

Unique entries of `cargo tree -e normal -p <crate> --prefix none | sort -u | wc -l` on Windows (target-specific
crates of other platforms are not in the count), and the JS production package count from
`pnpm list --prod --depth Infinity --json` (unique `name@version`, the workspace packages included).

| Binary | Before T7 | After T7 |
|---|---|---|
| `mosaic` (`mosaic-cli`) | 298 | 298 |
| `mosaic-server` | 253 | 252 |
| `mosaic-desktop` | 507 | 507 |
| `mosaic-native-host` (`mosaic-native-messaging`) | 79 | 79 |
| `@mosaic/extension` (JS) | 11 | 11 |
| `@mosaic/desktop` (JS) | 12 | 12 |

The JS side has nothing to trim: every package is a direct dependency or the transitive closure of one of the
rows above (the extension's 11 and the desktop's 12 differ only by `@tauri-apps/api`). The Rust tree was already small: every workspace crate takes its dependencies through exact pins with default features
off, and `cargo machete` (run once, 0.9.2, then every hit checked by hand) found no unused production
dependency.

## What T7 changed

- `tracing-subscriber` loses its `ansi` feature: removes `nu-ansi-term` from the server tree, and the server's
  stderr logs carry no escape codes (they go to a journal or a container log, not a terminal).
- `hex-literal` removed from the dev-dependencies of `mosaic-native-messaging` (no use).
- `deny.toml`: `ring` banned except for `rustls` and `rustls-webpki`; `serde_cbor` and `rsa` banned (neither is
  in the tree: `passkey-auth` 0.1.3 uses `ciborium` and has no `rsa`, ADR-0078); the stale ignore
  RUSTSEC-2024-0429 removed (no crate in the tree matches it).
- Dependabot covers all 13 fuzz lockfiles.

## Left as is

`cargo machete` reports, all false positives, checked by hand:

- `mosaic-ssh-agent`: `bcrypt-pbkdf` and `ed25519-dalek` are not called; they exist to turn on `zeroize` in the
  copies `ssh-key` builds (see the comment in `Cargo.toml`).
- `mosaic-desktop-fuzz` and `mosaic-sync-fuzz`: the targets include the shell's and the crate's modules by
  path, and those modules use the listed crates.

Not trimmed:

- `tauri` feature `compression` (pulls `brotli`): ADR-0019 keeps compressed embedded assets; dropping it grows
  the bundle and saves the `brotli` crates.
- `ssh-key` stays at `=0.7.0-rc.11` (M3-R13). The newest on crates.io is `0.7.0-rc.12`; no final 0.7.0 exists.
  The stable 0.6.7 pulls `p256` 0.13 and `crypto-bigint` 0.5 (RUSTSEC-2026-0387). Revisit when 0.7.0 is final.

### Duplicate majors (`multiple-versions = "warn"`)

None can be removed by bumping a direct dependency: each pair has one side that only a third-party crate selects.

| Crate pair | Held by |
|---|---|
| `p256`, `ecdsa`, `elliptic-curve`, `crypto-bigint`, `primeorder`, `pkcs8`, `spki`, `der`, `sec1`, `rfc6979`, `signature`, `ff`, `group`, `base16ct`, `pem-rfc7468`, `const-oid` (0.13 / 0.14 generations) | `passkey-auth` and `opaque-ke` (RustCrypto 0.13 generation) against `ssh-key` 0.7, `p256` 0.14 (0.14 generation); RUSTSEC-2026-0387 is reasoned in `deny.toml` |
| `curve25519-dalek` 4 / 5, `ed25519-dalek` 2 / 3, `ed25519` 2 / 3 | `passkey-auth` (2) and `opaque-ke` (4) against `ssh-key` (3) and `hpke` (5) |
| `sha2`, `hmac`, `hkdf`, `digest`, `block-buffer`, `crypto-common`, `cpufeatures` 0.10 / 0.11 generations | `opaque-ke` 4.0 (digest 0.10), `passkey-auth`; Mosaic's own code is on 0.11 (ADR-0046) |
| `getrandom` 0.2 / 0.3 / 0.4, `rand_core` 0.6 / 0.10 | `ring`, `rand` 0.8 (`passkey-auth`), `tauri`; ours is 0.4 |
| `base64` 0.22 / 0.23 | `passkey-auth` (0.22) |
| `syn` 2 / 3, `windows-sys` 0.59 / 0.60 / 0.61, `windows-targets`, `hashbrown` 0.12 / 0.17, `indexmap` 1 / 2, `keyboard-types` | Tauri's tree (`schemars`, `global-hotkey`), `keyring`, `rfd`, `rpassword` |
| `miniz_oxide` 0.8 / 0.9 | `png` (0.8) against `flate2` (0.9) |
