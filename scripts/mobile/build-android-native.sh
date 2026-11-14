#!/usr/bin/env bash
# Builds the Rust core for Android (ADR-0095) with cargo-ndk and puts the libraries where Gradle packs them.
#
#   build-android-native.sh <debug|release> [--features f]
#
# Runs `cargo ndk -t arm64-v8a -t x86_64 -o apps/mobile/android/core/src/main/jniLibs build -p mosaic-mobile-ffi`.
# CI only: it needs cargo-ndk, the Android NDK (ANDROID_NDK_HOME) and the Rust targets
# aarch64-linux-android and x86_64-linux-android (`rustup target add`, never in rust-toolchain.toml).
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build-android-native.sh <debug|release> [--features f]

  debug|release   the Cargo profile
  --features f    Cargo features of mosaic-mobile-ffi, e.g. test-hooks (debug builds for tests only;
                  release builds must have none: scripts/mobile/check-no-test-hooks.sh checks the result)

Environment: ANDROID_NDK_HOME (the NDK root) and cargo-ndk on the PATH.
Output: apps/mobile/android/core/src/main/jniLibs/{arm64-v8a,x86_64}/libmosaic_mobile_ffi.so
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  debug|release) profile="$1"; shift ;;
  *) usage >&2; exit 2 ;;
esac

features=""
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --features) features="${2:?--features needs a value}"; shift 2 ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$profile" = release ] && [ -n "$features" ]; then
  echo "error: release builds take no features (test hooks must not ship)" >&2
  exit 2
fi
if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "error: cargo-ndk is not installed (cargo install cargo-ndk --locked)" >&2
  exit 1
fi
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "error: ANDROID_NDK_HOME is not set" >&2
  exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

args=(build -p mosaic-mobile-ffi --locked)
[ "$profile" = release ] && args+=(--release)
[ -n "$features" ] && args+=(--features "$features")

# --platform 28 is the app's minSdk.
cargo ndk --platform 28 -t arm64-v8a -t x86_64 \
  -o apps/mobile/android/core/src/main/jniLibs "${args[@]}"

for abi in arm64-v8a x86_64; do
  lib="apps/mobile/android/core/src/main/jniLibs/$abi/libmosaic_mobile_ffi.so"
  if [ ! -f "$lib" ]; then
    echo "error: $lib was not built" >&2
    exit 1
  fi
done
echo "ok: $profile libraries in apps/mobile/android/core/src/main/jniLibs"
