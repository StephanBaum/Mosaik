# Reproducible builds

Anyone can rebuild some Mosaic release files from the source and compare the hashes. This page says
which, how, and what does not work. Design: ADR-0086, THREAT_MODEL R120.

## What is reproducible

On Linux, with the pinned toolchain, building twice in two differently named directories gives the
same bytes for:

- the `mosaic` and `mosaic-native-host` binaries and the CLI archive `mosaic-cli-<version>-x86_64-unknown-linux-gnu.tar.gz`;
- the `mosaic-server` binary;
- the server image (an OCI tar, built without a cache);
- both extension zips (`mosaic-extension-<version>-chromium.zip` and `-firefox.zip`).

The release workflow's `reproducibility` job does exactly this on every run and fails on any
difference. It publishes the hashes as `reproducibility-report.txt`.

## How

- The Rust toolchain is pinned by `rust-toolchain.toml` and every build uses `--locked`.
- `SOURCE_DATE_EPOCH` is the commit time (`git log -1 --format=%ct`).
- Paths are remapped and the build ID is dropped:
  `RUSTFLAGS="--remap-path-prefix=<source dir>=/mosaic --remap-path-prefix=$CARGO_HOME=/cargo -C link-arg=-Wl,--build-id=none"`.
- Archives are deterministic: sorted entries, owner 0, fixed times (`tar --sort=name`, `zip -X`,
  `gzip -n`).
- The image build sets `SOURCE_DATE_EPOCH`, the same remaps for `/src` and the cargo home, and
  `rewrite-timestamp=true`, and builds without provenance attestations.

## Rebuild it yourself (Linux)

You need the toolchain from `rust-toolchain.toml`, `pnpm`, Node 22, `zip`, GNU `tar`, and `$HOME/.cargo`
as your cargo home (set `CARGO_HOME` to move it; the script remaps whatever it is).

```
git clone https://github.com/StephanBaum/Mosaik.git && cd Mosaik
git checkout v0.1.0
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
mkdir -p /tmp/mosaic-src /tmp/mosaic-out
git archive HEAD | tar -x -C /tmp/mosaic-src
bash /tmp/mosaic-src/scripts/release/repro-build.sh /tmp/mosaic-src /tmp/mosaic-out
cat /tmp/mosaic-out/SHA256SUMS.repro
```

Compare the lines with `reproducibility-report.txt` of the release, or hash the files in the release
directly (`sha256sum mosaic-cli-*.tar.gz mosaic-extension-*.zip`). To rebuild only the extension zips:
`pnpm install --frozen-lockfile && bash scripts/release/package-extension.sh --build out`.

The image: build with the same Dockerfile and compare the digest.

```
docker buildx create --driver docker-container --use
docker buildx build --no-cache -f deploy/Dockerfile --build-arg SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH \
  --provenance=false --sbom=false --output type=oci,dest=image.tar,rewrite-timestamp=true .
tar -xOf image.tar index.json   # the manifest digest must equal the one in mosaic-server-image.ref
```

A different BuildKit version can change the OCI layout. The release log prints the BuildKit version used.

## What is not reproducible

- **Desktop bundles** (MSI, DMG, deb, rpm): WiX, DMG creation and packaging embed times and
  identifiers. The Tauri build is not bit for bit repeatable.
- **Signed files**: an Authenticode or Developer ID signature carries a certificate and a timestamp.
  `SHA256SUMS.unsigned` lists the hashes **before** platform signing so a rebuilder can compare the
  unsigned content. On Windows the signing script records them. For a macOS build that is signed in the
  same step, no pre-signing copy exists, so those files are not listed.
- **Windows and macOS CLI archives**: not checked by the reproducibility job.
- Toolchain and dependency sources must be the same as ours; a different Rust version gives different bytes.

## SBOMs

`mosaic.cdx.json`, `mosaic-server.cdx.json`, `mosaic-desktop.cdx.json` and `mosaic-native-host.cdx.json`
come from `cargo cyclonedx` (pinned through `taiki-e/install-action`). `mosaic-extension.cdx.json` and
`mosaic-desktop-frontend.cdx.json` come from `scripts/release/sbom-js.mjs`, which reads
`pnpm list --prod` and the hashes in `pnpm-lock.yaml`. `mosaic-server-image.cdx.json` is the server SBOM
plus the image digest and the base image digests. Their timestamps are `SOURCE_DATE_EPOCH`.
