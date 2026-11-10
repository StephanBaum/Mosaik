#!/usr/bin/env bash
# Checks that the release tag and every version in the tree agree (M9 T9).
#
#   check-versions.sh [--tag vX.Y.Z] [--allow-untagged]
#
# Sources: the workspace version (Cargo.toml), apps/desktop/src-tauri/tauri.conf.json,
# apps/extension/package.json, and the built extension manifests (dist/*/manifest.json)
# when they exist. With a tag (--tag, or GITHUB_REF_NAME for a tag ref) every source must
# equal the tag without its "v". With --allow-untagged and no tag, a mismatch between the
# sources is a warning, not a failure (development trees). The last line of stdout is
# "version=X.Y.Z" when the check passes.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tag=""
allow_untagged=0
while [ $# -gt 0 ]; do
  case "$1" in
    --tag) tag="${2:?--tag needs a value}"; shift 2 ;;
    --allow-untagged) allow_untagged=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
if [ -z "$tag" ] && [ "${GITHUB_REF_TYPE:-}" = "tag" ]; then
  tag="${GITHUB_REF_NAME:-}"
fi

semver='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
fail=0

ws="$(awk '/^\[workspace\.package\]/{s=1;next} /^\[/{s=0} s&&/^version[[:space:]]*=/{split($0,a,"\"");print a[2];exit}' "$root/Cargo.toml")"
json_version() {
  node -e 'process.stdout.write(String(JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).version??""))' "$1"
}
tauri="$(json_version "$root/apps/desktop/src-tauri/tauri.conf.json")"
ext="$(json_version "$root/apps/extension/package.json")"

names=("workspace Cargo.toml" "tauri.conf.json" "apps/extension/package.json")
vals=("$ws" "$tauri" "$ext")
for b in chromium firefox; do
  m="$root/apps/extension/dist/$b/manifest.json"
  if [ -f "$m" ]; then
    names+=("extension dist/$b manifest")
    vals+=("$(json_version "$m")")
  fi
done

for i in "${!names[@]}"; do
  echo "${names[$i]}: ${vals[$i]}"
  if ! [[ "${vals[$i]}" =~ $semver ]]; then
    echo "error: ${names[$i]} is not a semantic version" >&2
    fail=1
  fi
done

if [ -n "$tag" ]; then
  want="${tag#v}"
  case "$tag" in v*) ;; *) echo "error: tag '$tag' does not start with v" >&2; fail=1 ;; esac
  for i in "${!names[@]}"; do
    if [ "${vals[$i]}" != "$want" ]; then
      echo "error: ${names[$i]} is ${vals[$i]}, the tag is $tag" >&2
      fail=1
    fi
  done
  version="$want"
else
  if [ "$allow_untagged" -ne 1 ]; then
    echo "error: no tag given (use --tag or --allow-untagged)" >&2
    exit 2
  fi
  version="$ws"
  for i in "${!names[@]}"; do
    if [ "${vals[$i]}" != "$ws" ]; then
      echo "warning: ${names[$i]} is ${vals[$i]}, the workspace version is $ws (a release tag needs them equal)" >&2
    fi
  done
fi

[ "$fail" -eq 0 ] || exit 1
echo "version=$version"
