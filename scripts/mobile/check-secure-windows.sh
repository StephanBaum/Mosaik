#!/usr/bin/env bash
# Fails if Android app code opens a Compose dialog, menu, popup or sheet without the tapjacking filter.
#
#   check-secure-windows.sh
#
# A Compose dialog or menu is a window of its own and does not inherit the activity's
# `filterTouchesWhenObscured` (RT-android-4). Every one must go through ui/components/SecureWindows.kt, which
# sets it. This scans apps/mobile/android/app/src/main/kotlin for a bare AlertDialog(, DropdownMenu(,
# ExposedDropdownMenu(, Dialog(, Popup(, ModalBottomSheet( or BasicAlertDialog( outside that file. Tests are not
# scanned. Exit code 0: clean; 1: found; 2: usage.
set -euo pipefail

case "${1:-}" in
  -h|--help) echo "Usage: check-secure-windows.sh (run from anywhere; needs only grep)"; exit 0 ;;
  "") ;;
  *) echo "error: unknown argument: $1" >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

dir=apps/mobile/android/app/src/main/kotlin
if [ ! -d "$dir" ]; then
  echo "error: $dir not found" >&2
  exit 2
fi

# `\b` and the opening parenthesis keep SecureAlertDialog( and SecureDropdownMenu( out of the match.
pattern='(^|[^A-Za-z0-9_])(AlertDialog|BasicAlertDialog|DropdownMenu|ExposedDropdownMenu|Dialog|Popup|ModalBottomSheet)[[:space:]]*\('
if grep -RnE --include='*.kt' --exclude='SecureWindows.kt' "$pattern" "$dir"; then
  echo "error: use SecureAlertDialog / SecureDropdownMenu (ui/components/SecureWindows.kt) for every dialog and menu" >&2
  exit 1
fi
echo "ok: every dialog and menu goes through SecureWindows.kt"
