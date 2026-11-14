#!/usr/bin/env bash
# Guards the Android dependency tree (docs/MOBILE.md, ADR-0095).
#
#   check-android-deps.sh [--write]
#
# Resolves `:app:dependencies --configuration releaseRuntimeClasspath`, reduces it to group:artifact and
# fails if an artifact is not in apps/mobile/android/dependency-allowlist.txt, or if one matches
# firebase|gms|crashlytics|analytics|sentry|mlkit (no Firebase, Play services, analytics or crash reporting).
# With --write the allowlist is rewritten from the resolved tree instead (review the diff before committing).
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-android-deps.sh [--write]

  --write   rewrite apps/mobile/android/dependency-allowlist.txt from the resolved classpath

Runs ./gradlew in apps/mobile/android (JDK 21; the generated bindings are not needed for this task).
Extra Gradle arguments can be passed in GRADLE_ARGS.
USAGE
}

write=0
case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  --write) write=1 ;;
  "") ;;
  *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
android="$root/apps/mobile/android"
allowlist="$android/dependency-allowlist.txt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cd "$android"
# shellcheck disable=SC2086
./gradlew --console=plain -q ${GRADLE_ARGS:-} :app:dependencies --configuration releaseRuntimeClasspath > "$tmp/tree.txt"

# Tree lines look like "+--- group:artifact:1.0 -> 1.1 (*)" or "\--- group:artifact -> 1.1".
# Skip project dependencies and constraints "(c)"; keep group:artifact only.
sed -n 's/^[| +\\-]*--- //p' "$tmp/tree.txt" \
  | grep -v '^project ' \
  | grep -v ' (c)$' \
  | sed -E 's/^([A-Za-z0-9_.-]+):([A-Za-z0-9_.-]+).*/\1:\2/' \
  | sort -u > "$tmp/resolved.txt"

if [ ! -s "$tmp/resolved.txt" ]; then
  echo "error: no dependencies were parsed from the Gradle output" >&2
  head -n 20 "$tmp/tree.txt" >&2
  exit 1
fi

if [ "$write" -eq 1 ]; then
  {
    sed -n '/^#/p' "$allowlist"
    cat "$tmp/resolved.txt"
  } > "$tmp/new.txt"
  mv "$tmp/new.txt" "$allowlist"
  echo "ok: wrote $(wc -l < "$tmp/resolved.txt") entries to $allowlist"
  exit 0
fi

status=0
if banned="$(grep -Ei 'firebase|gms|crashlytics|analytics|sentry|mlkit' "$tmp/resolved.txt")"; then
  echo "error: forbidden dependencies (telemetry, Play services or ML Kit):" >&2
  echo "$banned" >&2
  status=1
fi

sed -e '/^#/d' -e '/^[[:space:]]*$/d' "$allowlist" | sort -u > "$tmp/allowed.txt"
if unlisted="$(comm -23 "$tmp/resolved.txt" "$tmp/allowed.txt")" && [ -n "$unlisted" ]; then
  echo "error: dependencies that are not in dependency-allowlist.txt:" >&2
  echo "$unlisted" >&2
  status=1
fi
if stale="$(comm -13 "$tmp/resolved.txt" "$tmp/allowed.txt")" && [ -n "$stale" ]; then
  echo "note: allowlist entries that are no longer resolved (remove them):" >&2
  echo "$stale" >&2
fi

if [ "$status" -eq 0 ]; then
  echo "ok: $(wc -l < "$tmp/resolved.txt") dependencies, all allowed"
fi
exit "$status"
