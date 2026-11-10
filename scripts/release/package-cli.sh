#!/usr/bin/env bash
# Packs the CLI and the native host of one Unix target into a deterministic tar.gz (M9 T9).
#
#   package-cli.sh <target-triple> <bindir> <outdir>
#
# <bindir> holds `mosaic` and `mosaic-native-host`. Writes
# <outdir>/mosaic-cli-<version>-<target>.tar.gz: sorted entries, owner 0:0, mtime
# SOURCE_DATE_EPOCH (default: the last commit), gzip without a name or time stamp. Needs GNU
# tar (gtar on macOS). Windows archives are made in the workflow (they are not reproducible).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
target="${1:?usage: package-cli.sh <target> <bindir> <outdir>}"
bindir="${2:?bindir}"
out="${3:?outdir}"

tar_bin=tar
if ! tar --version 2>/dev/null | grep -q GNU; then tar_bin=gtar; fi
command -v "$tar_bin" >/dev/null || { echo "error: GNU tar is not installed" >&2; exit 1; }
sde="${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct)}"
version="$(awk '/^\[workspace\.package\]/{s=1;next} /^\[/{s=0} s&&/^version[[:space:]]*=/{split($0,a,"\"");print a[2];exit}' "$root/Cargo.toml")"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
name="mosaic-cli-$version-$target"
"$tar_bin" --sort=name --mtime="@$sde" --owner=0 --group=0 --numeric-owner --mode='u=rwx,go=rx' \
  -C "$bindir" -cf - mosaic mosaic-native-host | gzip -n -9 > "$out/$name.tar.gz"
echo "wrote $out/$name.tar.gz"
