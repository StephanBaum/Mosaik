#!/usr/bin/env bash
# Writes the SHA-256 manifest of a release directory (M9 T9, CRYPTO_SPEC section 17).
#
#   checksums.sh <dir> [--out NAME]
#
# Hashes every regular file in <dir> (not in subdirectories) except the manifest
# itself (SHA256SUMS and the --out name) and signatures (*.minisig), and writes
# <dir>/<NAME> (default SHA256SUMS) in `sha256sum -c` format, names sorted bytewise.
# The server image is listed through the file mosaic-server-image.ref, which holds
# ghcr.io/<namespace>/mosaic-server@sha256:<digest>; keeping the reference in a hashed file
# keeps SHA256SUMS a plain list that every checksum tool accepts.
set -euo pipefail

dir="${1:?usage: checksums.sh <dir> [--out NAME]}"
shift
name=SHA256SUMS
if [ "${1:-}" = "--out" ]; then
  name="${2:?--out needs a name}"
  shift 2
fi
[ $# -eq 0 ] || { echo "unknown argument: $1" >&2; exit 2; }
[ -d "$dir" ] || { echo "error: $dir is not a directory" >&2; exit 1; }

if command -v sha256sum >/dev/null; then hash_cmd=(sha256sum --text); else hash_cmd=(shasum -a 256 --text); fi  # text mode: two spaces on every platform

cd "$dir"
files="$(find . -maxdepth 1 -type f ! -name SHA256SUMS ! -name "$name" ! -name '*.minisig' | sed 's|^\./||' | LC_ALL=C sort)"
[ -n "$files" ] || { echo "error: no files to hash in $dir" >&2; exit 1; }
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
while IFS= read -r f; do
  case "$f" in
    *[[:space:]]*) echo "error: file name with white space: $f" >&2; exit 1 ;;
  esac
  # A public key is never listed. Only the dry run's own key may sit here (sign.sh writes it after
  # this script, so normally it is absent); any other *.pub is refused, not skipped (RT-release-5).
  case "$f" in
    DRY-RUN-NOT-A-RELEASE-minisign.pub) continue ;;
    *.pub) echo "error: unexpected public key file in the release directory: $f" >&2; exit 1 ;;
  esac
  "${hash_cmd[@]}" -- "$f" >> "$tmp"
done <<< "$files"
mv "$tmp" "$name"
trap - EXIT
echo "wrote $dir/$name ($(wc -l < "$name" | tr -d ' ') files)"
