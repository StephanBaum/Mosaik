# Mosaic mobile (M10)

The Android and iOS apps: their architecture, the FFI contract, security behaviour, build, tests, release and
manual checks. Decisions: ADR-0088 to ADR-0096. Normative crypto: [CRYPTO_SPEC.md](CRYPTO_SPEC.md) §6.2, §6.9
and §16. Residual risks: [THREAT_MODEL.md](THREAT_MODEL.md) §4.8, R125 to R140.

Mosaic is pre-audit. Nothing in this document has been reviewed from outside. Until the owner has an Apple
developer team and a Play Console account (OD-M10-2, OD-M10-3), everything here is built and tested only in CI
simulators and emulators.

## 1. Architecture

```
 Kotlin / Compose app ──┐                          ┌── SwiftUI app ─── Credential Provider extension
 Autofill Service ──────┤  generated bindings      │   (separate process, ~120 MiB memory limit)
 CredentialProvider ────┘  (UniFFI, not committed) └── MosaicKit (shared Swift framework)
            │  JNA                                         │  XCFramework (static)
            ▼                                              ▼
   crates/mobile-ffi  (mosaic-mobile-ffi: binding shell, generated scaffolding only, forbid(unsafe_code))
            │  plain Rust calls
            ▼
   crates/mobile      (mosaic-mobile: all logic, forbid(unsafe_code), host-tested)
            │
   mosaic-vault · mosaic-sync (http) · mosaic-core · mosaic-origin · mosaic-passkey · mosaic-totp
   mosaic-generator · mosaic-qr
```

- **The platform code holds** the UI, the OS keystores (through one foreign trait, `SecureStorage`),
  `BiometricPrompt` and `LocalAuthentication`, the clipboard, the camera, and the OS provider entry points.
- **Rust owns every policy:**
  - which secret is stored under which name, and when it is deleted;
  - the quick-unlock rules;
  - origin and app matching, and how far a browser's `webDomain` is trusted;
  - passkey ceremonies and their response JSON;
  - sync;
  - settings integrity.
- **The UI stays thin.** Screens render Rust records. Logic in Kotlin or Swift is limited to calling the OS.
- **Toolkits:** Jetpack Compose and SwiftUI, both native (ADR-0088). Minimum OS: Android API 28, iOS 17.
- **`mosaic-platform` is not used on mobile.** Its `keyring` and `arboard` do not build for Android, and the
  platform code lives in Kotlin and Swift.

## 2. The FFI contract

The binding surface is `crates/mobile-ffi/src/lib.rs`; its rustdoc is the reference. It mirrors
`mosaic_mobile::Core` one method at a time (ADR-0089).

**Rules:**
- **Formats.** IDs are canonical lowercase UUID strings. Times are Unix milliseconds.
- **Blocking calls.** Every method blocks, and there is no UniFFI async.
  - Kotlin: `CoreHolder` (one `MosaicCore` per process) runs calls on `Dispatchers.IO.limitedParallelism(1)`.
  - Swift: `CoreHolder` is an actor over a serial `DispatchQueue`.
  - `lock()` and `state()` bypass the queue.
- **Rust state.** `mosaic_mobile::Core` keeps its phase in a `Mutex`: `Closed`, `NoAccount`, `Locked`,
  `Unlocking`, `Unlocked`, `Joining` or `Busy`, plus the pending slots with their expiry.
  - The mutex is never held across Argon2id, the network, a `SecureStorage` callback or another core call.
  - A call during `Unlocking` answers `busy`. A `lock()` during `Unlocking` ends locked.
  - `lock()` takes the `OwnedSession`, so a running sync fails with `SyncError::Locked` and never deadlocks.
  - Every slot is dropped on lock, the pending join included. `close_store()` keeps an unexpired pending join (the
    iOS Safari handoff of ADR-0091), so the apps never call `lock()` while that handoff runs; an expired join is
    dropped by `lock()`, `state()` and `close_store()`. `Core` is `Send + Sync`, checked by a static assertion.
  - The `lock_requested` flag is read and cleared only while the mutex is held, so a `lock()` that arrives during a
    long call always wins (M10-G1-1).
  - Presence checks (`state()`, `sync_overview()`) use `SecureStorage::contains` outside the mutex and never read a
    stored secret (M10-G1-2). A secret read before a `lock()` is wiped instead of handed out (M10-G1-6).
  - Five wrong re-authentications in a row (`quick_unlock_enroll_begin`, `update_settings`) lock with
    `lock_requiring_password`, as on the desktop (ADR-0028 item 8, M10-G1-3).
  - Entry points are bound to their platform: the `*_ios_*` calls and `fill_target_ios` answer `invalidInput` on an
    Android core, the Android calls on an iOS core; the iOS extension context never fetches asset links (M10-G1-5,
    G1-8).
- **Secrets.** Secret inputs are `Vec<u8>` UTF-8 (`ByteArray` in Kotlin, `Data` in Swift). Secret outputs are
  only `Arc<SecretValue>`, with `expose_bytes`, `len` and `wipe`. CRYPTO_SPEC §6.9.5 has the full rules.
- **Errors.** `MobileError` has unit variants only. `code()` is a camelCase string, and `mobile_error_codes()`
  lists them all. Errors carry no message, ID or value. The i18n key of each code is `error.<code>`.
  - **Shared with the desktop** (`docs/DESKTOP_IPC.md`):
    - store and session: `internal`, `busy`, `storeInUse`, `storeUnavailable`, `noAccount`, `accountExists`,
      `locked`;
    - credentials: `emptyPassword`, `wrongCredentials`, `secretKeyRequired`, `secretKeyMalformed`,
      `secretKeyMistyped`;
    - quick unlock and re-authentication: `passwordRequired`, `quickUnlockExpired`, `quickUnlockInvalidated`,
      `reauthRequired`;
    - items: `notFound`, `invalidInput`, `vaultUnavailable`, `itemTypeNotAllowed`;
    - one-time codes: `otpQrNotOtp`, `otpQrMigration`, `otpQrMultiple`, `otpQrInvalid`, `otpUriInvalid`,
      `otpKeyInvalid`;
    - sync: `syncNotConfigured`, `syncSignedOut`, `syncTwoFactorRequired`, `syncTwoFactorInvalid`,
      `syncHandoffExpired`, `syncRateLimited`, `syncPasswordChanged`, `syncAccountRekeyed`, `syncFailed`, and
      the server codes of `sync_check_server`.
  - **New in M10:**
    - context and onboarding: `passwordInApp`, `folderAccountUnsupported`, `setupCodeInvalid`,
      `notEnoughMemory`;
    - autofill: `noMatch`, `insecureOrigin`, `appNotLinked`, `foreignFrame`;
    - passkeys: `assetLinksUnavailable`, `appNotVerified`, `rpIdInvalid`, `passkeyNotFound`, `passkeyExcluded`,
      `passkeyUnsupportedAlgorithm`, `passkeySessionExpired`;
    - other: `qrNotFound`, `storageUnavailable`.
- **Contexts.** `CoreContext::App` and `CoreContext::IosAutofillExtension`. The extension context refuses the
  password unlock (`passwordInApp`) and every join, sync, recovery, reset, enrolment or settings call
  (`invalidInput`) before any work (CRYPTO_SPEC §6.9.3).
- **Configuration.** `CoreConfig { store_dir, platform, context, boot_id, trust_anchors_der }`:
  - `boot_id` is 1 to 32 bytes (CRYPTO_SPEC §6.9.2);
  - `trust_anchors_der` is filled on Android only (§6.9.6).
- **Bindings.**
  - `crates/mobile-ffi/uniffi.toml`: Kotlin package `io.github.stephanbaum.mosaic.ffi`, cdylib name
    `mosaic_mobile_ffi`, Swift module `MosaicFFI`.
  - The bindings are generated in CI and never committed.

## 3. Security behaviour per platform

| Topic | Android | iOS | Reference |
|---|---|---|---|
| Secret Key and session token | Keystore key `mosaic-secure-store` (AES-256-GCM, unlocked device required, StrongBox when available); one AEAD file per value under `noBackupFilesDir/secure/` | Keychain generic password, `WhenUnlockedThisDeviceOnly`, app-private group | CRYPTO_SPEC §6.9.1, ADR-0090 |
| Quick unlock | Keystore AES key per enrolment, Class 3 biometrics, invalidated by enrolment changes; API 28–29 per-use branch; platform byte `0x04` | Secure Enclave P-256 key in the shared keychain group, `.biometryCurrentSet`; platform byte `0x03` | CRYPTO_SPEC §6.2, ADR-0090 |
| When quick unlock applies | once the master password has unlocked since the device booted (boot marker, store v11), within the maximum age (default 14 days) | same; the extension uses the app's marker | CRYPTO_SPEC §6.9.2 |
| Memory | free-memory check before Argon2id (`notEnoughMemory`) | the same check; the extension never runs Argon2id | CRYPTO_SPEC §6.9.3, §6.9.4 |
| Store | `noBackupFilesDir/mosaic/store.sqlite`; backups and device transfer off | app group, excluded from backup, `NSFileProtectionComplete`; closed on background and after each extension request | ADR-0094 |
| Lock | background (default immediately), idle 5 min, screen off, new process; provider grace of at most 60 s inside a provider flow | background, idle 5 min, device lock, end of each extension request | ADR-0094 |
| Clipboard | `EXTRA_IS_SENSITIVE`; cleared after 30 s unless the user copied something else | `.localOnly`, `.expirationDate` | ADR-0094 |
| Screen | `FLAG_SECURE` on every window; no recents screenshot (API 33+); content sensitivity (API 35+) | privacy cover; hidden while captured; `.privacySensitive()` | ADR-0094 |
| Secret input | native `EditText` read into byte arrays and wiped; no personalized IME learning; excluded from other autofill services | `UITextField` with secure entry (not `SecureField`) | CRYPTO_SPEC §6.9.5 |
| TLS | system anchors from the default trust manager; no user CAs; TLS 1.3 only | `rustls-platform-verifier` (Security.framework) | CRYPTO_SPEC §2, §6.9.6, ADR-0091 |
| Autofill | Autofill Service with per-dataset authentication; values only after a tap; `webDomain` only from allowlisted browsers; app logins bound to package and exactly one current signer (`apkContentsSigners`, or the last entry of `signingCertificateHistory`; more than one is refused, ADR-0092 amendment); `fill_candidates` returns a fill session bound to the resolved target and `fill_values`/`fill_otp` take that session, never a target from the platform (M10-G1-12); no compatibility mode, inline suggestions or save | Credential Provider: quick unlock, matching items first, `userInteractionRequired` for silent requests, one-time codes on 18+ | ADR-0092 |
| Passwords via Credential Manager | API 34+; entries without values; the value only from `CredentialActivity` | — | ADR-0092 |
| Passkeys | Credential Manager API 34+; privileged browsers in hash mode with a double allowlist check; other apps need asset-links delegation; UV always | Credential Provider 17+, hash mode; UV from the biometric unlock; a ceremony without UV is refused, and the extension signs only with a session from `unlock_quick` in that process (M10-G1-4) | CRYPTO_SPEC §16, ADR-0093 |
| Identity store | — | passkey identities with the passkey setting; password identities only with OD-M10-7 (default off) | ADR-0092, R132 |
| Setup code | scanned with CameraX and decoded in Rust (no ML Kit) | scanned with `AVCaptureMetadataOutput` | CRYPTO_SPEC §6.9.7, ADR-0096 |

Limits that users notice:
- **Large vaults in the iOS extension.** `UnlockedSession::load` decrypts every item, and a vault above about
  5 000 items may exceed the ~120 MiB limit. A lazy-load mode for the vault would be a later API change.
- **Accounts with the `strong` or `max` Argon2 profile** (512 MiB, 1 GiB) may not unlock on a phone with little
  free memory. The phone says "This phone needs more free memory to unlock. Close other apps and try again."
  Profiles never decrease (CRYPTO_SPEC §5.3), so such an account may not work on low-memory phones at all.
- **Web autofill on Android** works only in browsers that send `webDomain` natively and are on the allowlist.
  Chrome needs "Autofill using another service" turned on.
- **Hosted accounts only.** A folder account gets: "This account syncs through a folder. The phone apps support
  Mosaic sync for now."
- **Re-key or Secret Key change elsewhere.** It needs "Set up this phone again". Unsynced local edits are then
  lost; the app shows their count and asks the user to type the email.

## 4. Build

CI-first (ADR-0095). Native builds run in `.github/workflows/mobile.yml`:
- `android`: cargo-ndk for `arm64-v8a` and `x86_64`, the host library for the JVM tests, bindgen, Gradle unit
  tests, lint and assemble.
- `android-release-check`: a release library without features, a check for test hooks, and an R8 release
  build.
- `android-instrumented`: emulator API 34, x86_64, no Play services.
- `ios`: static libraries, the XCFramework, XcodeGen, simulator tests, and a release compile check for devices.

Scripts (`scripts/mobile/`, bash, `set -euo pipefail`, each with `--help`):

| Script | Does |
|---|---|
| `build-android-native.sh <debug\|release> [--features f]` | `cargo ndk -t arm64-v8a -t x86_64 -o apps/mobile/android/core/src/main/jniLibs build -p mosaic-mobile-ffi [--release]` |
| `build-ios-xcframework.sh <debug\|release> [--features f]` | static libraries for `aarch64-apple-ios` and `aarch64-apple-ios-sim`, Swift bindings, `xcodebuild -create-xcframework` into `apps/mobile/ios/MosaicFFI/` |
| `bindgen.sh <kotlin\|swift> <lib> <out>` | `cargo run -p mosaic-mobile-ffi --features bindgen --bin uniffi-bindgen -- generate --library <lib> --language <l> --out-dir <out> --config crates/mobile-ffi/uniffi.toml` |
| `check-ffi-crate.sh` | no `unsafe` word in `crates/mobile-ffi/src`, only the allowed files, `#![forbid(unsafe_code)]` in every crate root (ADR-0089) |
| `check-no-test-hooks.sh <libs…>` | fails on `testhooks` symbols in release libraries |
| `check-android-deps.sh` | the release classpath against `apps/mobile/android/dependency-allowlist.txt`; no Firebase, Play services, analytics, crash reporting or ML Kit |
| `check-ios-deps.sh` | no remote Swift package in `project.yml` |
| `check-no-logging.sh` | no `android.util.Log`, `println(`, `print(`, `NSLog`, `os_log`, `Logger(` or `debugPrint` in app sources |

Local loop on the development PC (Windows), one command at a time, with `CARGO_BUILD_JOBS=1`:

```sh
cargo fmt --all --check
cargo clippy -p mosaic-mobile -p mosaic-mobile-ffi --all-targets -- -D warnings
cargo test -p mosaic-mobile
cargo build -p mosaic-mobile-ffi                 # target/debug/mosaic_mobile_ffi.dll
bash scripts/mobile/bindgen.sh kotlin target/debug/mosaic_mobile_ffi.dll "$TEMP/m10-bindings"   # sanity only
bash scripts/mobile/check-ffi-crate.sh
pnpm --filter @mosaic/i18n test && pnpm --filter @mosaic/i18n export:mobile
pnpm --filter @mosaic/design-tokens test && pnpm --filter @mosaic/design-tokens export:mobile
```

Gradle, Kotlin compiles, cargo-ndk and everything for iOS run in CI. Gradle may resolve dependencies locally
only to write `gradle/verification-metadata.xml`, when no other build runs.

**Committed:** sources, `project.yml`, the Gradle files and wrapper jar, `verification-metadata.xml`, the
exported strings and tokens, and the fonts.

**Generated and git-ignored:** the bindings, `jniLibs/`, `MosaicFFI.xcframework`, the `.xcodeproj`, `build/`,
`.gradle/` and `local.properties`.

**Android dependencies.** The reasons are in `docs/DEPENDENCIES.md`. Permissions: `INTERNET`, `USE_BIOMETRIC`,
and `CAMERA` (asked at runtime, for scanning only). No other permission. Package visibility uses a `<queries>`
element for launcher apps only; no `QUERY_ALL_PACKAGES` (ADR-0092).

**iOS** has no third-party dependency, no Swift packages from outside the repo, and no CocoaPods.

## 5. Tests and how to run them

Rust tests run on the host with `cargo test -p <crate>`, locally and in `ci.yml`. Kotlin, Swift and instrumented
tests run in `mobile.yml`. The names below are binding: each track writes the tests in its scope.

**`mosaic-core`:**
- `setup_code_round_trips`
- `setup_code_rejects_wrong_prefix_version_keys_and_oversize`
- `setup_code_checks_the_secret_key_checksum`
- `setup_code_normalizes_email_and_origin`
- the existing label uniqueness test covers the new labels
- fuzz target `setup_code`

**`mosaic-vault`:**
- `boot_marker_lets_quick_unlock_in_a_new_session_same_boot`
- `boot_marker_from_another_boot_requires_password`
- `quick_unlock_without_boot_marker_requires_password`
- `record_boot_marker_refused_after_quick_unlock`
- `boot_marker_deleted_with_every_quick_unlock_deletion`
- `boot_marker_deleted_by_lock_requiring_password`
- `boot_marker_deleted_at_failed_quick_limit`
- `boot_id_must_be_1_to_32_bytes`
- `schema_v10_fixture_upgrades_to_v11` (fixture `crates/vault/tests/fixtures/store_v10.sqlite`)
- `desktop_app_start_rule_unchanged_without_marker`

**`mosaic-passkey`:**
- `assert_hash_mode_signs_auth_data_and_given_hash`
- `create_hash_mode_has_no_client_data_json`
- `hash_mode_equals_json_mode_for_the_same_client_data`
- `android_app_origin_is_apk_key_hash_b64url`
- `json_mode_vectors_unchanged`

**`mosaic-origin`:**
- app URLs: `android_app_url_parses_package_and_cert`, `android_app_url_without_cert_never_matches`,
  `android_app_matches_only_equal_package_and_current_signer`, `android_app_url_rejects_bad_package_names`
- web domains and iOS identifiers: `web_domain_from_untrusted_app_is_ignored`,
  `web_domain_from_allowlisted_browser_is_page_origin`, `web_scheme_http_from_browser_is_refused_without_allow_http`,
  `ios_domain_identifier_is_https_origin`, `ios_url_identifier_keeps_scheme_host_port`,
  `ios_punycode_identifier_carries_idn_warning`
- field selection: `select_fields_excludes_foreign_frames`, `select_fields_refuses_focus_in_foreign_frame`,
  `select_fields_by_hints_autocomplete_and_input_type`, `select_fields_ignores_invisible_nodes`,
  `select_fields_finds_otp_by_one_time_code`, `select_fields_de_and_en_keywords`
- asset links and the allowlist: `assetlinks_get_login_creds_matches_package_and_cert`,
  `assetlinks_handle_all_urls_accepted_for_passkeys_only`, `assetlinks_wrong_cert_or_relation_does_not_match`,
  `assetlinks_rejects_malformed_and_oversized`, `privileged_browser_allowlist_parses_and_has_chrome_and_firefox`
- fuzz targets `assetlinks_parse` and `field_selection` in `crates/origin/fuzz` (the crate exists after M9 T11;
  add the targets to it)

**`mosaic-sync`:**
- `anchors_trust_is_tls13_only_with_ring`
- `anchors_trust_without_anchors_is_refused`
- `assetlinks_fetch_refuses_redirects_and_non_200`
- `assetlinks_fetch_caps_the_body`

**`mosaic-qr`:**
- `setup_payload_is_classified_and_returned`
- `scan_drops_other_payloads_unread`

**`mosaic-mobile`, core (T3a):**
- open and close: `open_new_dir_reports_no_account`, `close_store_releases_the_file_lock`,
  `reopen_after_close_restores_locked_state`, `core_is_send_and_sync`
- join (loopback server): `join_hosted_unlocks_and_stores_secret_key_and_token`, `join_hosted_totp_second_factor`,
  `join_hosted_wrong_password_is_wrong_credentials`, `join_hosted_webauthn_begin_returns_handoff_url_and_waits`,
  `join_refused_when_account_exists`, `pending_join_holds_no_store`, `pending_join_expires_after_five_minutes`,
  `folder_setup_code_or_config_is_folder_account_unsupported`
- unlock: `unlock_password_uses_stored_secret_key`, `unlock_password_records_boot_marker`,
  `wrong_secret_key_checksum_is_mistyped`, `extension_context_refuses_password_unlock_join_and_sync`,
  `extension_quick_unlock_after_app_password_unlock_same_boot`, `quick_unlock_refused_after_reboot`,
  `required_unlock_memory_matches_profile_plus_margin`
- quick unlock: `quick_unlock_enroll_requires_password_and_returns_aad`,
  `quick_unlock_pending_slot_expires_and_is_dropped_on_lock`, `quick_unlock_params_ios_strict_decode`,
  `quick_unlock_params_android_strict_decode`, `quick_unlock_platform_byte_must_match_platform`,
  `quick_unlock_platform_failed_deletes_wrapping_and_returns_alias`, `quick_unlock_replaced_returns_old_alias`
- lock and concurrency: `lock_drops_session_slots_and_secret_values`, `lock_during_unlock_ends_locked`,
  `lock_during_sync_fails_sync_without_deadlock`, `concurrent_calls_during_unlock_answer_busy`
- items: `items_and_detail_never_contain_concealed_values`, `reveal_returns_secret_value_and_wipe_empties_it`,
  `save_login_creates_and_edits_with_history`, `save_login_password_none_keeps_password`,
  `trash_item_moves_to_trash`, `otp_code_and_remaining_seconds`, `otp_pending_preview_has_no_secret_and_expires`,
  `generate_password_respects_options`
- sync (two cores and a loopback server): `sync_now_pushes_edit_and_pulls_remote`,
  `sync_password_changed_elsewhere_then_recover`, `recover_password_deletes_quick_unlock_and_marker`,
  `rekey_elsewhere_reports_account_rekeyed`, `reset_device_wipes_store_secrets_and_returns_aliases`
- settings and secrets: `settings_mac_rejects_tampered_file`, `settings_ranges_are_clamped`,
  `secret_value_debug_is_redacted`, and `secure_store_aad_binds_version_and_name` (the AAD of CRYPTO_SPEC §6.9.1)

**`mosaic-mobile`, providers (T3b):**
- autofill: `fill_candidates_exact_origin_only`, `fill_candidates_etld1_opt_in`,
  `fill_candidates_http_refused_without_allow_http`, `fill_candidates_android_app_requires_cert_binding`,
  `fill_candidates_skip_trashed_and_hardware_protected`, `fill_candidates_have_no_secret_fields`,
  `fill_values_rechecks_the_match`, `fill_values_refuses_other_origin_candidate`, `fill_values_locked_is_locked`,
  `fill_otp_for_matching_item_only`, `link_android_app_writes_cert_bound_url`,
  `asset_links_status_verified_not_listed_unavailable`
- passkeys: `passkey_android_app_requires_asset_links_delegation`, `passkey_android_privileged_browser_hash_mode`,
  `passkey_android_untrusted_platform_origin_is_refused`, `passkey_android_rp_id_validated_against_origin`,
  `passkey_android_get_response_json_parses_independently`,
  `passkey_android_create_response_json_parses_independently`, `passkey_create_stores_item_before_response`,
  `passkey_create_excluded_credential_refused`, `passkey_assert_persists_sign_count_before_release`,
  `passkey_refuses_without_user_verification`, `passkey_ios_assert_hash_mode`, `passkey_ios_register_hash_mode`,
  `passkey_sessions_expire_and_drop_on_lock`, `passkey_identities_carry_no_key_material`,
  `passkey_request_json_rejects_oversized_and_unknown_types`
- fuzz target `crates/mobile/fuzz/fuzz_targets/passkey_request_json.rs`
- setup code: `scan_setup_qr_returns_code_and_drops_others`
- The "independent parser" tests decode the response JSON with `serde_json::Value` and check every member,
  including the base64url decodes and a P-256 verification with `p256::ecdsa::VerifyingKey`.

**`mosaic-mobile-ffi`:**
- `error_codes_unique_camel_case_and_complete`
- `conversions_round_trip_for_every_record`
- `ffi_crate_has_no_logic`

**Kotlin JVM tests** (CI; JNA loads the host library through `uniffi.component.mosaic.libraryOverride`):
- `:core`: `CoreSmokeTest.versionMatchesRust`, `CoreSmokeTest.lockedCoreRefusesItems`,
  `CoreHolderTest.callsRunSerially`, `CoreHolderTest.lockPreemptsTheQueue`, `SecretBytesTest.wipesAfterUse`,
  `SecretBytesTest.utf8EncodingWithoutString`
- `:app`, errors and lock policy: `ErrorStringsTest.everyMobileErrorCodeHasAStringResource`,
  `LockPolicyTest.locksImmediatelyOnBackgroundByDefault`, `LockPolicyTest.idleTimeoutLocks`,
  `LockPolicyTest.screenOffLocks`, `LockPolicyTest.providerGraceEndsAfter60s`,
  `LockPolicyTest.providerGraceEndsOnScreenOff`
- `:app`, clipboard, keys, structure and strings: `ClipboardGuardTest.marksClipSensitive`,
  `ClipboardGuardTest.keepsForeignClipWhenReadable`, `ClipboardGuardTest.clearsWhenUnreadable`,
  `QuickUnlockKeyParamsTest.matchesCryptoSpecPerApiLevel`, `StructureReaderTest.mapsNodesHintsAndWebDomain`,
  `L10nTest.argumentOrderMatchesCatalog`

**Android instrumented tests** (CI emulator API 34):
- storage, keys and trust: `SecureStorageInstrumentedTest.roundTripDeleteAndAadBinding`,
  `QuickUnlockKeystoreTest.keyIsAuthBoundAndInvalidatedByEnrollment`, `TrustAnchorsTest.containsIsrgRootX1AndNoUserCas`
- screens and unlock: `ScreenSecurityTest.everyActivityHasFlagSecure`, `UnlockFlowTest.passwordUnlockShowsVault`
- autofill: `AutofillInstrumentedTest.fillsLinkedAppAfterDatasetTap`, `AutofillInstrumentedTest.noDatasetValuesBeforeTap`,
  `AutofillInstrumentedTest.ignoresWebDomainFromNonBrowserApp`, `AutofillInstrumentedTest.excludesForeignFrameFields`.
  The test client activity declares a `MAIN`/`LAUNCHER` intent filter, so it is visible under the `<queries>`
  rule (ADR-0092).
- screenshots: `ScreenshotTest.mainScreensLightDarkEnDe`

**Swift** (CI simulator):
- `MosaicKitTests`, platform pieces: `KeychainSecureStorageTests.testRoundTripDeleteAccessibilityAndNoSync`,
  `SecureEnclaveQuickUnlockTests.testAccessControlFlagsAndTag`, `BootIdTests.testStableWithinBootAnd16Bytes`,
  `ServiceIdentifierTests.testDomainAndUrlTargets`, `PasteboardTests.testLocalOnlyAndExpiryOptions`,
  `CoreHolderTests.testLockPreemptsAndWipesReveals`, `PrivacyCoverTests.testCoverOnResignActive`,
  `CaptureObserverTests.testRevealDisabledWhileCaptured`, `L10nTests.testEveryErrorCodeLocalized`
- `MosaicKitTests`, extension and lifecycle: `ExtensionContextTests.testPasswordUnlockRefusedInExtension`,
  `CredentialProviderLogicTests.testPasswordFlowWithTestVault`,
  `CredentialProviderLogicTests.testPasskeyAssertAndRegisterHashMode`,
  `IdentityStoreSyncTests.testPasswordIdentitiesOnlyWhenOptedIn`, `StoreLifecycleTests.testCloseOnBackgroundReleasesLock`,
  `MemoryTests.testUnlock5000ItemsUnder60MiB`
- `MosaicUITests`: `testPasswordUnlockShowsVault`, `testAccessibilityAuditMainScreens`, `testScreenshotsLightDarkEnDe`

**TypeScript:**
- `packages/i18n/src/mobileExport.test.ts`
- `packages/design-tokens/src/mobileExport.test.ts`
- the desktop setup-code tests (`setup_code_requires_reauth`, `setup_code_refused_for_folder_account`,
  `setup_code_expires_after_60_s`, `setup_code_modules_decode_to_payload`, and the Playwright case
  `setup-code dialog hides on blur and lock`)

## 6. Release (owner-gated)

Nothing is submitted to a store in M10. Each step waits for an owner decision. Until it is made, the default
applies.

| ID | Decision | Default |
|---|---|---|
| OD-M10-1 | Bundle ID, applicationId, extension ID, app group and keychain groups. They cannot change after the first store upload | `io.github.stephanbaum.mosaic` (as the desktop identifier); extension `….autofill`; `group.io.github.stephanbaum.mosaic`; `<prefix>io.github.stephanbaum.mosaic.shared`. Configurable through `MOSAIC_BUNDLE_ID` and the Gradle property `mosaicAppId` |
| OD-M10-2 | Apple Developer Program team (paid): needed for device builds, TestFlight and the App Store, and to test the Secure Enclave and the provider on a device | none: CI builds unsigned simulator builds and a device compile check; device checks wait |
| OD-M10-3 | Google Play Console and Play App Signing, with an upload key as a secret of the CI `release` environment | none: unsigned debug and release builds only |
| OD-M10-4 | Distribution outside the stores (a signed APK on GitHub Releases, F-Droid) | no; revisit after OD-M10-3 |
| OD-M10-5 | Minimum OS | iOS 17; Android API 28. Raise Android to API 30 to drop the API 28–29 key branch and the Android versions without security updates |
| OD-M10-6 | Store listings (DE/EN text, screenshots, privacy labels "Data not collected", the "pre-audit" wording) and store names: is "Mosaic" free on both stores? | drafted below, not submitted |
| OD-M10-7 | iOS password identities in the system identity store (QuickType suggestions) | off; passkey identities on with the passkey setting |
| OD-M10-8 | Android inline (keyboard) suggestions | off |
| OD-M10-9 | 32-bit Android (`armv7`) | no |
| OD-M10-10 | Desktop setup code (a QR code with the Secret Key) | yes, behind re-authentication, 60 s |
| OD-M10-11 | Folder sync on mobile (M10b: iOS iCloud Drive and File Provider first; Android SAF later or never) | deferred |
| OD-M10-12 | Hosted default URL in the apps (depends on OD-M9-6) | a custom URL is required until a domain exists |
| OD-M10-13 | CI budget if the repository becomes private (macOS minutes) | iOS UI tests nightly only; build and unit tests on PRs |
| OD-M10-14 | Local Android NDK on the development PC | no (CI-first) |
| OD-M10-15 | Export compliance (`ITSAppUsesNonExemptEncryption`, US EAR self-classification) and the Play encryption declaration | declare that the app uses encryption; the owner files what the classification requires before the first upload |
| OD-M10-16 | Mobile design mockups before T8 and T11 (the lesson of the M3 fidelity gap) | the agents derive screens from the desktop design language and tokens; the owner reviews the G4 screenshots, and a design-pass track follows if needed |

**Store listing draft** (OD-M10-6, not submitted):
- *Name:* Mosaic Password Manager.
- *Short text (EN):* "Your passwords, passkeys and one-time codes. Encrypted on your phone before they sync."
- *Short text (DE):* "Deine Passwörter, Passkeys und Einmalcodes. Auf deinem Handy verschlüsselt, bevor sie
  synchronisieren."
- *Long text, key points:*
  - zero-knowledge sync: the server only stores encrypted data;
  - your master password and Secret Key never leave your devices;
  - autofill and passkeys in apps and browsers;
  - no ads, no tracking, no analytics;
  - pre-audit: no outside security review yet.
- *Privacy labels:* "Data not collected" on both stores. The app talks only to the user's Mosaic server and, for
  passkeys in Android apps, to the website's own `/.well-known/assetlinks.json`.

Before the first upload:
- signing keys exist in the CI `release` environment (OD-M10-2, OD-M10-3);
- the export compliance is filed (OD-M10-15);
- a mobile SBOM (CycloneDX for Gradle) if the owner wants it;
- the manual checks below pass on real devices.

## 7. Manual check list (devices)

These need real hardware, and they wait for OD-M10-2 and OD-M10-3. Record each result in PROGRESS.

**Android** (one device with StrongBox and one without; API 28 or 29 if available; API 34 or later):
- [ ] Quick unlock with a fingerprint and with face unlock (Class 3 only). A Class 2 face unlock is not offered.
- [ ] Adding a fingerprint invalidates quick unlock, and the next unlock asks for the master password.
- [ ] After a reboot, the master password is required. After a password unlock, quick unlock works in the app
  and in the providers until the next reboot.
- [ ] Autofill in a linked app; "No login for X" in an unlinked app; a sideloaded copy signed with another key
  is not filled.
- [ ] Web autofill in Chrome (with "Autofill using another service" on), Firefox and Samsung Internet; a
  cross-origin iframe is not filled.
- [ ] Credential Manager: password and passkey sign-in in a real app, for example a site with
  `assetlinks.json`; the `appNotVerified` path for an app without one.
- [ ] Passkey registration and sign-in on a real website in Chrome.
- [ ] The clipboard clears after 30 s; Gboard's clipboard history does not show the copied value (API 33+).
- [ ] The recents screen shows no vault content; a screen recording shows a black window.
- [ ] Backups: `adb backup` and device-to-device transfer contain no store and no secret.
- [ ] Unlocking a `standard` account takes less than 3 s on a mid-range phone; a `max` account on a 4 GB phone
  gives `notEnoughMemory`, not a crash.

**iOS** (a Face ID device and a Touch ID device, iOS 17 and 18):
- [ ] Quick unlock with Face ID in the app and in the AutoFill extension. A biometry change invalidates it.
- [ ] After a reboot, the extension says "Open Mosaic and unlock it once with your master password."
- [ ] Password autofill in Safari and in an app with associated domains; QuickType suggestions only with
  OD-M10-7 on.
- [ ] Passkey registration and sign-in on a real website in Safari, and in an app with associated domains.
- [ ] One-time-code fill on iOS 18.
- [ ] iCloud Keychain does not offer to save Mosaic's master password.
- [ ] The app switcher shows the privacy cover; a screen recording hides revealed values.
- [ ] Extension memory with a large vault (5 000 items) stays under the limit.
- [ ] The app and the extension never crash with `0xdead10cc` (the store is closed on background).

**Both:**
- [ ] Join with a desktop setup code, then with a TOTP second factor, then with the WebAuthn handoff in the
  system browser.
- [ ] Change the master password on the desktop, then recover on the phone. Re-key on the desktop, then use
  "Set up this phone again".
- [ ] TalkBack and VoiceOver on the main screens; font scaling to 200 %; DE and EN.
