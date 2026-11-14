# Release runbook (for the owner)

How a Mosaic release is built, signed and published. The design is in CRYPTO_SPEC §17 and
ADR-0081, ADR-0082 and ADR-0086. What users do with the result is in `docs/VERIFY.md`. The
workflow is `.github/workflows/release.yml`; its helper scripts are in `scripts/release/`.

Mosaic is pre-audit: no external audit has been done. Every release is a prerelease and says so.

## What the workflow does

- Runs for a `v*` tag (a real release) or by hand with `dry_run` (the default).
- Checks that the tag, the workspace version, `tauri.conf.json` and the extension version agree,
  that no Cargo features are on for the shipped crates, and that no text claims what Mosaic never
  claims (`scripts/release/wording-check.mjs`).
- Builds the CLI archives and macOS `.pkg`, the desktop installers (MSI, DMG and app, deb, rpm),
  the extension zips, the signed Android APK and the server image (an OCI tar, never pushed unless
  you turn it on). The Android job runs the same release checks as `mobile.yml` (no Cargo features,
  no test hooks in the libraries, also checked inside the finished APK) and fails on a debuggable
  APK or a `versionName` that differs from the tag.
- Makes CycloneDX SBOMs for `mosaic`, `mosaic-server`, `mosaic-desktop` and `mosaic-native-host`,
  for the extension and desktop JS dependencies, and for the image (with the base image digests).
- Builds the Linux CLI, the server binary, the image and the extension zips twice in different
  directories and fails if any hash differs.
- Writes `SHA256SUMS`, signs it with minisign, then verifies its own output with the commands from
  `docs/VERIFY.md`. Then it creates a **draft** GitHub release. It never publishes by itself.

## One-time setup

Do this on a machine you trust, offline if you can.

1. **Generate the two minisign keys** (install `minisign`):
   - release key: `minisign -G -p release-minisign.pub -s release-minisign.key`
   - updater key: `pnpm --filter @mosaic/desktop tauri signer generate -w updater.key`
   Choose a long password for each, and a different one per key.
2. **Back them up offline.** Copy both secret keys to two encrypted USB drives, kept in different
   places. Store the passwords separately from the drives (a paper copy in a safe, or your own
   password manager's offline export). Never put a key or password in the repository, in chat or
   in an issue. The backup lets you sign a successor key if the CI secrets are ever lost.
3. **Publish the release public key.** Copy `release-minisign.pub` to `release/minisign.pub`, and put
   the same key text into `SECURITY.md` and `docs/VERIFY.md`. Commit that. The repository holds no
   placeholder key: until you do this, a tag run stops in `prepare`.
4. **Create the GitHub environment `release`** (Settings, Environments): you as the required
   reviewer, and deployment only from `v*` tags. Add a tag protection rule (a ruleset) for `v*` so
   only repository admins can create them. The `prepare` job also fails a real release unless the
   tagged commit is an ancestor of `origin/main`, so a tag on a commit that never went through the
   main ruleset (pull request, required checks, signed commits) cannot ship.
5. **Add the environment secrets** (not repository secrets):

   | Secret | Needed for |
   |---|---|
   | `MINISIGN_SECRET_KEY` | the release key file, as text. **Required.** |
   | `MINISIGN_PASSWORD` | its password. **Required.** |
   | `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the updater `.sig` files (optional; without them they are not built) |
   | `WINDOWS_SIGN_MODE` (`trusted-signing` or `pfx`), then `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TRUSTED_SIGNING_ENDPOINT`, `AZURE_TRUSTED_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE`, or `WINDOWS_PFX_BASE64`, `WINDOWS_PFX_PASSWORD` | Authenticode (optional) |
   | `APPLE_CERTIFICATE` (base64 `.p12` with the Developer ID Application and Installer identities), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_INSTALLER_IDENTITY`, `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_P8` | Developer ID and notarization (optional) |

   | `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` | the signed Android APK (optional, see "Android signing key" below) |

   Without the Windows or Apple secrets the builds ship **unsigned**, with a warning in the log
   (OD-M9-2, OD-M9-3). A half-set group fails the job rather than shipping unsigned by mistake.
   The Android group follows the same rule, with one difference: with **none** of the four set, a tag
   run skips the Android job with a warning and the release ships **without an APK**.
6. **Image push (optional).** The image is built and checksummed but not pushed. To push it to
   `ghcr.io/stephanbaum/mosaic-server`, set the repository variable `PUBLISH_IMAGE` to `true`.

## Android signing key

Do this once, on a machine you trust, before the first release that should carry an APK. Android
updates only install over an app signed with the **same key**. **If you lose the keystore or its
password, nobody can update the app**: users must uninstall it (and lose local data) and install a
new build signed with a new key. Treat it like the minisign keys.

1. **Create the keystore** (JDK 17 or newer; RSA 4096 shown, an EC key also works):

   ```
   keytool -genkeypair -keystore mosaic-release.keystore -storetype PKCS12 -alias mosaic-release      -keyalg RSA -keysize 4096 -validity 9125 -dname "CN=Mosaic, O=Mosaic" -v
   ```

   `-validity 9125` is 25 years. `keytool` asks for a password; use a long random one (a password
   manager can generate it). With PKCS12 the key password equals the store password; you may still
   store it as a separate secret.
2. **Back it up offline**, like the minisign keys: the keystore file on two encrypted drives in
   different places, the password stored separately. Never put it in the repository, in chat or in
   an issue.
3. **Print the signer fingerprint** and commit it:

   ```
   keytool -list -v -keystore mosaic-release.keystore -alias mosaic-release | grep 'SHA256:'
   ```

   Write the 64 hex digits (without colons) as one line to `release/android-signer.sha256`, and put
   the same value, with colons, into `SECURITY.md` and `docs/VERIFY.md` (the Android section). A tag
   run that builds an APK **fails** if this file is missing, or if the APK's signer differs from it.
4. **Add four secrets to the environment `release`**:

   | Secret | Value |
   |---|---|
   | `ANDROID_KEYSTORE_BASE64` | `base64 -w0 mosaic-release.keystore` (Windows: `[Convert]::ToBase64String([IO.File]::ReadAllBytes("mosaic-release.keystore"))`) |
   | `ANDROID_KEYSTORE_PASSWORD` | the keystore password |
   | `ANDROID_KEY_ALIAS` | `mosaic-release` |
   | `ANDROID_KEY_PASSWORD` | the key password |

   The job decodes the keystore into the runner's temp folder, signs through a Gradle init script
   (`scripts/preview/android-signing.init.gradle`, shared with the preview build) and deletes the
   keystore in a step that always runs. No secret is printed. A dry run uses a throwaway key made in
   the job and uploads nothing.
5. **Google Play App Signing** is optional and later: if the app goes to Google Play, you can enrol
   the same key as the upload key. This is not part of the first release.

## Making a release

1. Bump the version everywhere in one pull request: `[workspace.package]` in `Cargo.toml` (and
   `Cargo.lock`), `apps/desktop/src-tauri/tauri.conf.json`, `apps/extension/package.json`. Also keep
   `versionName` in `apps/mobile/android/app/build.gradle.kts` (raise `versionCode` for every
   release that ships an APK) and `MARKETING_VERSION` in `apps/mobile/ios/project.yml` equal; the
   Android job fails if the APK's `versionName` differs from the tag. The first release is
   `v0.1.0`, a prerelease (every release is, while Mosaic is pre-audit).
2. Run a **dry run** from the Actions tab (Release, Run workflow, keep `dry_run` on). It needs no
   secrets, uses an ephemeral key, signs `SHA256SUMS.DRY-RUN.minisig`, publishes nothing and uploads
   no artifacts. It proves the pipeline builds and verifies.
3. Tag and push: `git tag -s v0.1.0 -m "Mosaic 0.1.0" && git push origin v0.1.0`.
4. Approve the pending deployments to `release` (one click covers the signing jobs). A tag run
   without `MINISIGN_SECRET_KEY` and `MINISIGN_PASSWORD` **fails**; it never becomes a dry run.
5. Download the draft release's files and check them yourself with `docs/VERIFY.md`, on a machine
   that did not build them. Install the MSI or DMG once.
6. Publish the draft.

## Rotating a key

Make the new key (setup step 1). Sign its public half with the **old** secret key
(`minisign -S -s old.key -m new.pub -x release/minisign.pub.next.minisig`), commit `new.pub` as
`release/minisign.pub` and the signature as `release/minisign.pub.next.minisig`, then replace the
two secrets. Announce it in the README and a GitHub advisory. A leaked key is rotated, not restored.
If the CI secrets are lost but the offline backup is not, sign the successor with the backup.
There is no transparency log (THREAT_MODEL R118).

## What is not covered

- Unsigned builds until you add certificates (R119). SmartScreen and Gatekeeper will warn.
- The Android APK is not part of the SBOM job or the reproducibility job (Gradle and the NDK are
  not pinned for a bit-for-bit rebuild yet), and no AAB is built (the dependency pins in
  `apps/mobile/android/gradle/verification-metadata.xml` were generated for the APK build only).
  Android is not covered by `docs/REPRODUCIBLE_BUILDS.md`. iOS has no release build yet.
- The in-app updater is off; only the `.sig` files are built (OD-M9-5).
- No GitHub provenance attestations (OD-M9-15) and no license statement yet; the SBOMs say
  `NOASSERTION` for Mosaic's own components (OD-M9-11).
- Desktop bundles and signed binaries are not reproducible (`docs/REPRODUCIBLE_BUILDS.md`).
- `trusted-signing-cli` (Windows, Azure mode) is installed with `cargo install` at a pinned version
  and is not covered by `cargo deny`; it runs only on the release runner.
