#!/usr/bin/env bash
# Builds the reproducible Linux artifacts in one source tree (M9 T9, ADR-0086).
#
#   SOURCE_DATE_EPOCH=<n> repro-build.sh <srcdir> <outdir>
#
# <srcdir> is a clean export of the repository (git archive). Builds `mosaic`,
# `mosaic-native-host` and `mosaic-server` with --locked, path remapping and no build ID,
# packs the CLI tarball, builds the extension and packs both zips. Writes the artifacts and
# <outdir>/SHA256SUMS.repro. The release workflow runs this twice in two different
# directories and compares the two SHA256SUMS.repro files.
set -euo pipefail

src="$(cd "${1:?usage: repro-build.sh <srcdir> <outdir>}" && pwd)"
mkdir -p "${2:?outdir}"
out="$(cd "$2" && pwd)"
: "${SOURCE_DATE_EPOCH:?SOURCE_DATE_EPOCH is required}"
export SOURCE_DATE_EPOCH
export CARGO_INCREMENTAL=0
export TZ=UTC
export CARGO_TARGET_DIR="$src/target"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="--remap-path-prefix=$src=/mosaic --remap-path-prefix=$cargo_home=/cargo -C link-arg=-Wl,--build-id=none"

cd "$src"
cargo build --release --locked -p mosaic-cli -p mosaic-server -p mosaic-native-messaging
mkdir -p "$out/bin"
for b in mosaic mosaic-native-host mosaic-server; do
  install -m 0755 "target/release/$b" "$out/bin/$b"
done
cp "$out/bin/mosaic-server" "$out/mosaic-server-linux-x86_64"
bash scripts/release/package-cli.sh x86_64-unknown-linux-gnu "$out/bin" "$out"

pnpm install --frozen-lockfile
bash scripts/release/package-extension.sh --build "$out"

rm -rf "$out/bin"
bash scripts/release/checksums.sh "$out" --out SHA256SUMS.repro
