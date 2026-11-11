#!/usr/bin/env bash
# Generates the Kotlin and Swift bindings of mosaic-mobile-ffi (ADR-0089).
#
#   bindgen.sh [--out DIR] [--release] [kotlin|swift|all]
#
# Builds the library, runs the `uniffi-bindgen` binary of the crate (feature `bindgen`) in library
# mode (it reads crates/mobile-ffi/uniffi.toml), and writes the bindings under DIR/kotlin and DIR/swift
# (default: $TEMP/mosaic-bindings). The bindings are generated in CI and never committed. The script
# fails if the generator prints a warning.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bindgen.sh [--out DIR] [--release] [kotlin|swift|all]

  --out DIR   output directory (default: $TEMP/mosaic-bindings; DIR/kotlin and DIR/swift)
  --release   build and read the release library instead of the debug one
  kotlin|swift|all   which bindings to generate (default: all)

Needs cargo and the Rust toolchain of the repository. Run it from anywhere.
USAGE
}

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
out="${TEMP:-${TMPDIR:-/tmp}}/mosaic-bindings"
profile=debug
build_flags=()
languages=(kotlin swift)

while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --out) out="${2:?--out needs a directory}"; shift 2 ;;
    --release) profile=release; build_flags=(--release); shift ;;
    kotlin) languages=(kotlin); shift ;;
    swift) languages=(swift); shift ;;
    all) languages=(kotlin swift); shift ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

cd "$root"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

target_dir="${CARGO_TARGET_DIR:-}"
if [ -z "$target_dir" ]; then
  target_dir="$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' | sed 's#\\#/#g')"
fi

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) lib_name=mosaic_mobile_ffi.dll ;;
  Darwin) lib_name=libmosaic_mobile_ffi.dylib ;;
  *) lib_name=libmosaic_mobile_ffi.so ;;
esac

cargo build -p mosaic-mobile-ffi "${build_flags[@]}"
library="$target_dir/$profile/$lib_name"
if [ ! -f "$library" ]; then
  echo "error: $library was not built" >&2
  exit 1
fi

mkdir -p "$out"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
for language in "${languages[@]}"; do
  rm -rf "${out:?}/$language"
  mkdir -p "$out/$language"
  # --no-format: ktlint and swiftformat are not needed (and not required to be installed).
  cargo run -q -p mosaic-mobile-ffi --features bindgen --bin uniffi-bindgen "${build_flags[@]}" -- \
    generate --library "$library" --language "$language" \
    --no-format --out-dir "$out/$language" 2>"$log" || {
      cat "$log" >&2
      echo "error: bindings generation for $language failed" >&2
      exit 1
    }
  if grep -Eiq 'warn' "$log"; then
    cat "$log" >&2
    echo "error: the generator warned while generating $language" >&2
    exit 1
  fi
  echo "ok: $language bindings in $out/$language"
done
