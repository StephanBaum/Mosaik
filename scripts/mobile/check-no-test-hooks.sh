#!/usr/bin/env bash
# Fails if a release library of the mobile core contains the test hooks (ADR-0089, spec 3.3).
#
#   check-no-test-hooks.sh <library.so|library.a|library.dylib|library.dll>
#
# The test hooks (`mosaic_mobile::test_hooks::TestHooks`) are built only for the JVM unit tests, the
# instrumented tests and the XCTest builds. This lists the symbols with `nm` (or `llvm-nm`) and fails
# if one mentions them. Release libraries are stripped of local symbols, so a hit means a feature leak.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-no-test-hooks.sh <library>

  library   a release .so (Android), .a (iOS static library), .dylib or .dll

Needs nm or llvm-nm on the PATH. Exit code 0: no test-hooks symbol; 1: found; 2: usage or tool error.
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
# -C demangles; archives list every member. A tool failure is an error, not a pass.
if ! "$nm_tool" -C "$library" >"$symbols" 2>/dev/null && ! "$nm_tool" "$library" >"$symbols" 2>/dev/null; then
  echo "error: $nm_tool could not read $library" >&2
  exit 2
fi

if grep -Eiq 'test_?hooks' "$symbols"; then
  echo "error: $library contains test-hooks symbols:" >&2
  grep -Ei 'test_?hooks' "$symbols" | head -20 >&2
  exit 1
fi
echo "ok: $library has no test-hooks symbol"
