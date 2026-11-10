#!/usr/bin/env bash
# Packages the production extension builds as deterministic zips (M9 T9, ADR-0086).
#
#   package-extension.sh [--build] <outdir>
#
# Reads apps/extension/dist/{chromium,firefox} (build them first, or pass --build, which runs
# the production build and its dist check) and writes
# <outdir>/mosaic-extension-<version>-{chromium,firefox}.zip. Entries are sorted, every
# mtime is SOURCE_DATE_EPOCH (default: the last commit), the time zone is UTC and extra file
# attributes are dropped (zip -X), so the same input gives the same bytes.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
build=0
if [ "${1:-}" = "--build" ]; then
  build=1
  shift
fi
out="${1:?usage: package-extension.sh [--build] <outdir>}"

command -v zip >/dev/null || { echo "error: zip is not installed" >&2; exit 1; }
sde="${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct)}"
version="$(node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).version)' "$root/apps/extension/package.json")"

if [ "$build" -eq 1 ]; then
  (cd "$root" && pnpm --filter @mosaic/extension build && node apps/extension/scripts/check-dist.mjs)
fi

mkdir -p "$out"
out="$(cd "$out" && pwd)"
export TZ=UTC
for b in chromium firefox; do
  src="$root/apps/extension/dist/$b"
  [ -f "$src/manifest.json" ] || { echo "error: $src is not built (run with --build)" >&2; exit 1; }
  stage="$(mktemp -d)"
  cp -R "$src/." "$stage/"
  # Fixed mtimes: `touch -d @N` is GNU; fall back to BSD `touch -t` on macOS.
  if touch -d "@$sde" "$stage" 2>/dev/null; then
    find "$stage" -exec touch -d "@$sde" {} +
  else
    stamp="$(date -u -r "$sde" +%Y%m%d%H%M.%S)"
    find "$stage" -exec touch -t "$stamp" {} +
  fi
  zipfile="$out/mosaic-extension-$version-$b.zip"
  rm -f "$zipfile"
  (cd "$stage" && find . -type f | LC_ALL=C sort | sed 's|^\./||' | zip -X -q -9 "$zipfile" -@)
  rm -rf "$stage"
  echo "wrote $zipfile"
done
