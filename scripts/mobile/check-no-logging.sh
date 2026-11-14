#!/usr/bin/env bash
# Fails if app code logs (docs/MOBILE.md: no logging of secrets, no telemetry).
#
#   check-no-logging.sh
#
# Scans the Android sources under apps/mobile/android/{app,core}/src/main for android.util.Log, Log.x(,
# println(, print(, printStackTrace and Logger(, and the iOS sources (when present) for print(, NSLog,
# os_log, Logger( and debugPrint. Tests are not scanned. Exit code 0: clean; 1: found; 2: usage.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-no-logging.sh

Scans app code (not tests) for logging calls. Run from anywhere; it needs only grep.
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

status=0
scan() {
  local label="$1" pattern="$2"
  shift 2
  local dirs=()
  for d in "$@"; do
    [ -d "$d" ] && dirs+=("$d")
  done
  [ "${#dirs[@]}" -gt 0 ] || return 0
  if grep -RnE --include='*.kt' --include='*.java' --include='*.swift' "$pattern" "${dirs[@]}" 2>/dev/null; then
    echo "error: $label code must not log (see the matches above)" >&2
    status=1
  fi
}

scan "Android" '\bandroid\.util\.Log\b|\bLog\.[a-z]\(|\bprintln\(|\bprint\(|printStackTrace|\bLogger\(|\bTimber\.' \
  apps/mobile/android/app/src/main apps/mobile/android/core/src/main

scan "iOS" '\bprint\(|\bNSLog\b|\bos_log\b|\bLogger\(|\bdebugPrint\(|\bdump\(' \
  apps/mobile/ios/Mosaic apps/mobile/ios/MosaicAutoFill apps/mobile/ios/MosaicKit

if [ "$status" -eq 0 ]; then
  echo "ok: no logging calls in app code"
fi
exit "$status"
