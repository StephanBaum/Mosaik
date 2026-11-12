# Verify your download

Before you run Mosaic, check that the file is the one the project built. Mosaic is pre-audit: no
external audit has been done.

Each release has these files next to the downloads:

- `SHA256SUMS`: the SHA-256 of every file in the release.
- `SHA256SUMS.minisig`: a signature over `SHA256SUMS` made with the Mosaic release key.
- `mosaic-server-image.ref`: the server image name, pinned by digest.
- `SHA256SUMS.unsigned` and `reproducibility-report.txt`: for people who rebuild
  (`docs/REPRODUCIBLE_BUILDS.md`).

## Quick check (one minute)

Download your file and `SHA256SUMS` into the same folder, then run one command. It tells you whether
the file arrived complete and is the one listed. Replace the file name with yours.

Windows (PowerShell). It must print `True`:

```
(Get-FileHash .\Mosaic_0.1.0_x64_en-US.msi).Hash -eq ((Select-String -Path .\SHA256SUMS -Pattern 'Mosaic_0.1.0_x64_en-US.msi').Line -split ' ')[0]
```

macOS. It must print `OK`:

```
grep ' Mosaic_0.1.0_aarch64.dmg$' SHA256SUMS | shasum -a 256 -c -
```

Linux. It must print `OK` for your file:

```
sha256sum -c --ignore-missing SHA256SUMS
```

If it prints anything else, delete the file and download it again. Do not run it.

This check proves the file matches the list. It does not prove the list is genuine, because whoever
could swap the file could swap the list. The expert check below does.

## For experts

### 1. The release key

The public signing key will be published here and in the repository file `release/minisign.pub`
with the first release. Take it from the repository, not from the release page. Until then there is
no release to verify.

A file called `SHA256SUMS.DRY-RUN.minisig` or a key called `DRY-RUN-NOT-A-RELEASE-minisign.pub` belongs
to a test run, not a release. Do not use it.

### 2. Check the signature of `SHA256SUMS`

Install minisign: Linux `sudo apt install minisign` (or your package manager), macOS `brew install
minisign`, Windows `winget install jedisct1.minisign` (or `scoop install minisign`).

```
minisign -Vm SHA256SUMS -x SHA256SUMS.minisig -p release/minisign.pub
```

It must print `Signature and comment signature verified` and a trusted comment of the form
`mosaic <version> <git commit>`. Check that the version is the one you meant to download.
If it prints anything else, stop and do not run the files.

### 3. Check the file against the signed list

Use the quick check above, after step 2 passed. Then the list is the signed one.

### 4. Platform signatures (when the build has one)

Windows:

```
Get-AuthenticodeSignature .\Mosaic_0.1.0_x64_en-US.msi | Format-List Status, SignerCertificate, TimeStamperCertificate
```

macOS (hardened runtime and notarization):

```
spctl -a -vv /Applications/Mosaic.app
codesign -dv --verbose=4 /Applications/Mosaic.app
xcrun stapler validate Mosaic_0.1.0_aarch64.dmg
```

### Unsigned builds

Until the project has a Windows code-signing certificate and an Apple Developer account, releases
are **unsigned** on those platforms (THREAT_MODEL R119). The minisign check in step 2 and the
checksum are then your proof; the operating system cannot give you one.

- **Windows SmartScreen** shows "Windows protected your PC" and the publisher as unknown. After
  the checks pass, choose "More info", then "Run anyway".
- **macOS Gatekeeper** says Mosaic "cannot be opened because Apple cannot check it for malicious
  software". After the checks pass, open System Settings, Privacy & Security, scroll to the
  message about Mosaic and choose "Open Anyway". Unsigned macOS builds do not use the hardened
  runtime, so the limits in THREAT_MODEL R105 and R111 apply.
- **Linux** packages (deb, rpm) are covered by the minisign signature only.

### The server image (OCI digest)

`mosaic-server-image.ref` holds a name like `ghcr.io/stephanbaum/mosaic-server@sha256:<digest>`.
The digest is listed (through that file) in `SHA256SUMS`, so it is signed. To use it:

```
docker pull ghcr.io/stephanbaum/mosaic-server@sha256:<digest>
```

Docker refuses anything whose digest differs. The image is only available there once publishing to
that registry is switched on; until then load the OCI tar from the release with
`skopeo copy oci-archive:mosaic-server-<version>-oci.tar docker-daemon:mosaic-server:<version>`.

### The browser extension

The release holds `mosaic-extension-<version>-chromium.zip` and `-firefox.zip`. Store listings are
not available yet, so for now you install the zip yourself (Chromium: load it unpacked; Firefox:
`about:debugging`, temporary add-on). You can rebuild the zip from source and compare the hash:
`docs/REPRODUCIBLE_BUILDS.md`.

### SBOMs

`*.cdx.json` are CycloneDX 1.5 lists of the components in each binary and in the extension and
desktop JavaScript. Mosaic's own components say `NOASSERTION` for the license, because none is chosen yet.
