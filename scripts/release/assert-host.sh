#!/usr/bin/env bash
# Post-build assertion on the release native host (ADR-0153, FIX-1): the binary that is about to
# ship must not trust the dev extension ID, whatever flag or cfg reached the build.
#
#   assert-host.sh <path to mosaic-native-host[.exe]>
#
# 1. Started with the dev extension's origin as argv[1], it refuses with exit code 2 ("the caller is
#    not an allowed extension", crates/native-messaging/src/main.rs) and connects to nothing.
# 2. The system manifests it writes (`manifest --system`, text only, any host OS) do not name the
#    dev ID.
set -euo pipefail

host="${1:?usage: assert-host.sh <host binary>}"
dev_id="lccdgfpfnmegbdgooanfejdbahhokikm"
[ -f "$host" ] || { echo "error: $host is missing" >&2; exit 1; }

# Stdin is closed: a host that accepted the caller would end at once with 0, not hang.
rc=0
"$host" "chrome-extension://$dev_id/" < /dev/null > /dev/null 2>&1 || rc=$?
if [ "$rc" -ne 2 ]; then
  echo "error: the host did not refuse the dev extension ID (exit $rc, expected 2)" >&2
  exit 1
fi
echo "ok: the host refuses the dev extension ID"

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
for os in windows linux; do
  "$host" manifest --system --os "$os" --out "$out/$os" > /dev/null
  if [ -z "$(find "$out/$os" -type f)" ]; then
    echo "error: manifest --system --os $os wrote no file" >&2
    exit 1
  fi
  if grep -rqi -- "$dev_id" "$out/$os"; then
    echo "error: the $os system manifest names the dev extension ID" >&2
    exit 1
  fi
done
echo "ok: the system manifests do not name the dev extension ID"
