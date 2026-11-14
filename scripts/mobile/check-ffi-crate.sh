#!/usr/bin/env bash
# Guards the binding shell (ADR-0089, spec 3.3): mosaic-mobile-ffi is the only crate that
# may expand generated low-level code, and it holds no logic of its own.
#
#   check-ffi-crate.sh
#
# Fails if
#   (a) the keyword of low-level code appears anywhere under crates/mobile-ffi/src,
#   (b) a file other than lib.rs, types.rs, convert.rs or bin/uniffi-bindgen.rs exists there, or
#   (c) the lib.rs, the main.rs or any src/bin/*.rs of any workspace crate, this crate
#       included, lacks the crate-level forbid of unsafe code.
# It reads the sources as text and needs no cargo.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-ffi-crate.sh

Checks crates/mobile-ffi/src (no hand-written low-level code, only the allowed files) and that every
workspace crate, mosaic-mobile-ffi and every src/bin/*.rs included, forbids unsafe code at its crate
root. Exit code 0 when all checks pass.
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ffi="crates/mobile-ffi/src"
fail=0

# (a)
if hits="$(cd "$root" && grep -rnw 'unsafe' "$ffi" 2>/dev/null)" && [ -n "$hits" ]; then
  echo "error: the keyword of low-level code appears in $ffi:" >&2
  echo "$hits" >&2
  fail=1
fi

# (b)
allowed=(lib.rs types.rs convert.rs bin/uniffi-bindgen.rs)
while IFS= read -r file; do
  rel="${file#"$root/$ffi/"}"
  ok=0
  for a in "${allowed[@]}"; do
    [ "$rel" = "$a" ] && ok=1
  done
  if [ "$ok" -eq 0 ]; then
    echo "error: $ffi/$rel is not one of: ${allowed[*]}" >&2
    fail=1
  fi
done < <(find "$root/$ffi" -type f | sort)

# (c)
check_root() {
  local file="$1"
  [ -f "$root/$file" ] || return 0
  if ! grep -Eq '^#!\[forbid\(unsafe_code\)\]' "$root/$file"; then
    echo "error: $file lacks #![forbid(unsafe_code)]" >&2
    fail=1
  fi
}
for dir in "$root"/crates/*/ "$root"/apps/desktop/src-tauri/; do
  rel="${dir#"$root/"}"
  rel="${rel%/}"
  check_root "$rel/src/lib.rs"
  check_root "$rel/src/main.rs"
  for bin in "$root/$rel"/src/bin/*.rs; do
    [ -f "$bin" ] || continue
    check_root "${bin#"$root/"}"
  done
done

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "ok: $ffi holds no low-level code and only its allowed files; every crate root and binary forbids it"
