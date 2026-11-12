#!/usr/bin/env bash
# Builds the Rust core for iOS (ADR-0095): static libraries, Swift bindings and MosaicFFI.xcframework.
#
#   build-ios-xcframework.sh <debug|release> [--features f] [--with-x86_64]
#
# macOS only (needs xcodebuild, lipo and the Rust targets installed with `rustup target add`, never in
# rust-toolchain.toml). Steps:
#   1. `cargo rustc -p mosaic-mobile-ffi --lib --crate-type staticlib --target <t>` for aarch64-apple-ios and
#      aarch64-apple-ios-sim (and x86_64-apple-ios with --with-x86_64, merged into the simulator slice);
#   2. the Swift bindings through scripts/mobile/bindgen.sh (the library without features, as shipped)
#      into apps/mobile/ios/Generated/Bindings;
#   3. `xcodebuild -create-xcframework` into apps/mobile/ios/MosaicFFI/MosaicFFI.xcframework.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build-ios-xcframework.sh <debug|release> [--features f] [--with-x86_64]

  debug|release   the Cargo profile
  --features f    Cargo features of mosaic-mobile-ffi, e.g. test-hooks (debug builds for tests only;
                  release builds must have none: the result is checked with check-no-test-hooks.sh)
  --with-x86_64   also build x86_64-apple-ios and merge it into the simulator slice (Intel Macs)

Needs macOS with Xcode, plus `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
(and x86_64-apple-ios with --with-x86_64).
Output: apps/mobile/ios/MosaicFFI/MosaicFFI.xcframework and apps/mobile/ios/Generated/Bindings/*.swift
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  debug|release) profile="$1"; shift ;;
  *) usage >&2; exit 2 ;;
esac

features=""
with_x86=0
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --features) features="${2:?--features needs a value}"; shift 2 ;;
    --with-x86_64) with_x86=1; shift ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$profile" = release ] && [ -n "$features" ]; then
  echo "error: release builds take no features (test hooks must not ship)" >&2
  exit 2
fi
for tool in xcodebuild lipo cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: $tool not found (this script needs macOS with Xcode and cargo)" >&2
    exit 1
  fi
done

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
# The deployment target of the app (project.yml); keeps the Rust objects from a newer minimum.
export IPHONEOS_DEPLOYMENT_TARGET=17.0

ios=apps/mobile/ios
work="$ios/build/xcframework"
bindings="$ios/Generated/Bindings"
out="$ios/MosaicFFI/MosaicFFI.xcframework"

target_dir="${CARGO_TARGET_DIR:-}"
if [ -z "$target_dir" ]; then
  target_dir="$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
fi

device_targets=(aarch64-apple-ios)
sim_targets=(aarch64-apple-ios-sim)
[ "$with_x86" -eq 1 ] && sim_targets+=(x86_64-apple-ios)

args=(--lib --crate-type staticlib --locked)
[ "$profile" = release ] && args+=(--release)
[ -n "$features" ] && args+=(--features "$features")

for target in "${device_targets[@]}" "${sim_targets[@]}"; do
  cargo rustc -p mosaic-mobile-ffi --target "$target" "${args[@]}"
  lib="$target_dir/$target/$profile/libmosaic_mobile_ffi.a"
  if [ ! -f "$lib" ]; then
    echo "error: $lib was not built" >&2
    exit 1
  fi
  if [ "$profile" = release ]; then
    bash scripts/mobile/check-no-test-hooks.sh "$lib"
  fi
done

rm -rf "$work" "$bindings" "$ios/MosaicFFI"
mkdir -p "$work/device" "$work/sim" "$work/headers" "$bindings"

cp "$target_dir/aarch64-apple-ios/$profile/libmosaic_mobile_ffi.a" "$work/device/libmosaic_mobile_ffi.a"
sim_libs=()
for target in "${sim_targets[@]}"; do
  sim_libs+=("$target_dir/$target/$profile/libmosaic_mobile_ffi.a")
done
if [ "${#sim_libs[@]}" -gt 1 ]; then
  lipo -create "${sim_libs[@]}" -output "$work/sim/libmosaic_mobile_ffi.a"
else
  cp "${sim_libs[0]}" "$work/sim/libmosaic_mobile_ffi.a"
fi

# Swift bindings of the library without features (the shipped API); features only add test symbols.
bash scripts/mobile/bindgen.sh --out "$work/generated" swift
found_swift=0
for f in "$work/generated/swift"/*.swift; do
  [ -f "$f" ] || continue
  cp "$f" "$bindings/"
  found_swift=1
done
if [ "$found_swift" -eq 0 ]; then
  echo "error: the bindings generator wrote no Swift file" >&2
  exit 1
fi
for f in "$work/generated/swift"/*.h; do
  [ -f "$f" ] && cp "$f" "$work/headers/"
done
modulemap="$(ls "$work/generated/swift"/*.modulemap 2>/dev/null | head -n 1 || true)"
if [ -z "$modulemap" ] || ! ls "$work/headers"/*.h >/dev/null 2>&1; then
  echo "error: the bindings generator wrote no header or module map" >&2
  exit 1
fi
cp "$modulemap" "$work/headers/module.modulemap"

mkdir -p "$ios/MosaicFFI"
xcodebuild -create-xcframework \
  -library "$work/device/libmosaic_mobile_ffi.a" -headers "$work/headers" \
  -library "$work/sim/libmosaic_mobile_ffi.a" -headers "$work/headers" \
  -output "$out"

if [ ! -d "$out" ]; then
  echo "error: $out was not created" >&2
  exit 1
fi
echo "ok: $profile XCFramework in $out and bindings in $bindings"
