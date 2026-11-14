#!/usr/bin/env bash
# Asserts that no Cargo features are enabled for the shipped crates (M8 TODO 1, ADR-0082).
#
#   check-features.sh
#
# 1. The shipped crates (mosaic-cli, mosaic-server, mosaic-native-messaging, mosaic-desktop)
#    have an empty default feature set.
# 2. No dependency of a shipped crate enables `test-util` outside [dev-dependencies].
# 3. The release build commands (release.yml, deploy/Dockerfile and scripts/release/*.sh)
#    pass no --features, --all-features, --no-default-features or -F.
# 4. The `preview` feature (ADR-0153) is not enabled anywhere the release desktop build reads:
#    no feature list of apps/desktop/src-tauri/Cargo.toml but its own definition names it
#    (custom-protocol is always on in `tauri build`), and tauri.conf.json and
#    tauri.release.conf.json have no `features` key and no `preview` at all, so neither
#    build.features nor a beforeBuildCommand can pull it in (CHAIN-2).
# It reads the manifests as text and needs no cargo.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail=0
shipped=(crates/cli crates/server crates/native-messaging apps/desktop/src-tauri)

for d in "${shipped[@]}"; do
  f="$root/$d/Cargo.toml"
  if [ ! -f "$f" ]; then
    echo "error: $f is missing" >&2
    fail=1
    continue
  fi
  default="$(awk '/^\[features\]/{s=1;next} /^\[/{s=0} s&&/^default[[:space:]]*=/{print;exit}' "$f")"
  if [ -n "$default" ] && ! printf '%s' "$default" | grep -Eq '^default[[:space:]]*=[[:space:]]*\[[[:space:]]*\]'; then
    echo "error: $d has default features: $default" >&2
    fail=1
  fi
  # A line naming test-util inside a [dependencies]-like section that is not a dev section.
  bad="$(awk '/^\[/{sec=$0} /test-util/ && !/^[[:space:]]*#/ && sec ~ /dependencies/ && sec !~ /dev-dependencies/ {print}' "$f")"
  if [ -n "$bad" ]; then
    echo "error: test-util enabled outside dev-dependencies in $d:" >&2
    echo "$bad" >&2
    fail=1
  fi
  echo "checked: $d"
done

files=("$root/.github/workflows/release.yml" "$root/deploy/Dockerfile")
for s in "$root"/scripts/release/*.sh; do
  if [ "$(basename "$s")" != "check-features.sh" ]; then files+=("$s"); fi
done
for f in "${files[@]}"; do
  [ -f "$f" ] || continue
  hits="$(grep -nE -- '(--features|--all-features|--no-default-features|[[:space:]]-F[[:space:]=])' "$f" | grep -vE '^[0-9]+:[[:space:]]*#' || true)"
  if [ -n "$hits" ]; then
    echo "error: feature flags in ${f#"$root"/}:" >&2
    echo "$hits" >&2
    fail=1
  fi
done

# 4. The preview feature must not reach the desktop build through its manifest or Tauri config.
desktop="$root/apps/desktop/src-tauri"
if [ -f "$desktop/Cargo.toml" ]; then
  # Any line of [features] except the preview definition itself (and comments) naming it.
  bad="$(awk '/^\[/{s=($0=="[features]")} s && !/^[[:space:]]*#/ && !/^preview[[:space:]]*=/ && /preview/ {print}' "$desktop/Cargo.toml")"
  if [ -n "$bad" ]; then
    echo "error: the preview feature is enabled by another feature of the desktop crate:" >&2
    echo "$bad" >&2
    fail=1
  fi
fi
for c in tauri.conf.json tauri.release.conf.json; do
  [ -f "$desktop/$c" ] || continue
  hits="$(grep -niE '"features"|preview' "$desktop/$c" || true)"
  if [ -n "$hits" ]; then
    echo "error: features or the preview word in apps/desktop/src-tauri/$c:" >&2
    echo "$hits" >&2
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then exit 1; fi
echo "ok: no Cargo features are enabled for the shipped crates"
