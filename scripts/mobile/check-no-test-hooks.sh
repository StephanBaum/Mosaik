#!/usr/bin/env bash
# Fails if a release library of the mobile core contains the test hooks (ADR-0089, spec 3.3).
#
#   check-no-test-hooks.sh <library.so|library.a|library.dylib|library.dll>
#
# The test hooks (`mosaic_mobile::test_hooks::TestHooks`) are built only for the JVM unit tests, the
# instrumented tests and the XCTest builds (and `mosaic-mobile` refuses to compile them without debug
# assertions). This lists the symbols with `nm` (or `llvm-nm`) and fails if one mentions them. A shared
# library is read through its dynamic symbol table (`nm -D --defined-only`, where the tool has it); a
# listing that comes back empty is an error, never a pass (G1-7).
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-no-test-hooks.sh <library>

  library   a release .so (Android), .a (iOS static library), .dylib or .dll

Needs nm or llvm-nm on the PATH. Exit code 0: no test-hooks symbol; 1: found; 2: usage or tool error
(including a library whose symbol listing is empty).
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") usage >&2; exit 2 ;;
esac

library="$1"
if [ ! -f "$library" ]; then
  echo "error: $library is not a file" >&2
  exit 2
fi

nm_tool=""
for candidate in llvm-nm nm; do
  if command -v "$candidate" >/dev/null 2>&1; then
    nm_tool="$candidate"
    break
  fi
done
if [ -z "$nm_tool" ]; then
  echo "error: neither llvm-nm nor nm is installed" >&2
  exit 2
fi

symbols="$(mktemp)"
trap 'rm -f "$symbols"' EXIT

# The dynamic symbol table of a shared library, the whole table of anything else. `-C` demangles;
# archives list every member. A tool failure is an error, not a pass.
list_symbols() {
  case "$library" in
    *.so)
      "$nm_tool" -C -D --defined-only "$library" 2>/dev/null
      ;;
    *.dylib)
      # Apple's nm has no -D; llvm-nm and GNU nm read the export table with --defined-only.
      "$nm_tool" -C --defined-only "$library" 2>/dev/null || "$nm_tool" -U "$library" 2>/dev/null
      ;;
    *)
      "$nm_tool" -C --defined-only "$library" 2>/dev/null || "$nm_tool" -C "$library" 2>/dev/null
      ;;
  esac
}

if ! list_symbols >"$symbols"; then
  echo "error: $nm_tool could not read $library" >&2
  exit 2
fi
if [ ! -s "$symbols" ]; then
  echo "error: $nm_tool listed no symbol of $library (a stripped or wrong file: this is not a pass)" >&2
  exit 2
fi

if grep -Eiq 'test_?hooks' "$symbols"; then
  echo "error: $library contains test-hooks symbols:" >&2
  grep -Ei 'test_?hooks' "$symbols" | head -20 >&2
  exit 1
fi
echo "ok: $library has no test-hooks symbol"
