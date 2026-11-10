#!/usr/bin/env bash
# Signs <dir>/SHA256SUMS with minisign (M9 T9, ADR-0081, CRYPTO_SPEC section 17).
#
#   VERSION=0.1.0 GIT_SHA=<sha> sign.sh <dir> --release
#   VERSION=0.1.0 GIT_SHA=<sha> sign.sh <dir> --dry-run
#
# --release  needs MINISIGN_SECRET_KEY (the content of the encrypted key file) and
#            MINISIGN_PASSWORD in the environment and FAILS when either is missing: it never
#            falls back to a dry run. Writes SHA256SUMS.minisig with the trusted comment
#            "mosaic <version> <git sha>".
# --dry-run  makes an ephemeral key, signs with it, writes SHA256SUMS.DRY-RUN.minisig (trusted
#            comment starting "DRY RUN") and DRY-RUN-NOT-A-RELEASE-minisign.pub, and throws
#            the secret half away. It never writes SHA256SUMS.minisig.
#
# The secrets are only read from the environment, never printed, never put on a command line
# and never traced (no `set -x` in this script, and the caller must not add one).
set -euo pipefail

dir="${1:?usage: sign.sh <dir> --release|--dry-run}"
mode="${2:?usage: sign.sh <dir> --release|--dry-run}"
version="${VERSION:?VERSION is required}"
git_sha="${GIT_SHA:?GIT_SHA is required}"
[ -f "$dir/SHA256SUMS" ] || { echo "error: $dir/SHA256SUMS is missing (run checksums.sh first)" >&2; exit 1; }
command -v minisign >/dev/null || { echo "error: minisign is not installed" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
umask 077

case "$mode" in
  --release)
    if [ -z "${MINISIGN_SECRET_KEY:-}" ] || [ -z "${MINISIGN_PASSWORD:-}" ]; then
      echo "error: MINISIGN_SECRET_KEY and MINISIGN_PASSWORD are required for a release signature" >&2
      exit 1
    fi
    if [ -e "$dir/SHA256SUMS.DRY-RUN.minisig" ]; then
      echo "error: a dry-run signature is present in a release directory" >&2
      exit 1
    fi
    printf '%s\n' "$MINISIGN_SECRET_KEY" > "$work/release.key"
    printf '%s\n' "$MINISIGN_PASSWORD" |
      minisign -S -s "$work/release.key" -m "$dir/SHA256SUMS" -x "$dir/SHA256SUMS.minisig" \
        -t "mosaic $version $git_sha" > /dev/null
    echo "signed $dir/SHA256SUMS.minisig"
    ;;
  --dry-run)
    if [ -e "$dir/SHA256SUMS.minisig" ]; then
      echo "error: a release signature is present in a dry-run directory" >&2
      exit 1
    fi
    minisign -G -W -f -p "$work/dry-run.pub" -s "$work/dry-run.key" > /dev/null
    minisign -S -s "$work/dry-run.key" -m "$dir/SHA256SUMS" -x "$dir/SHA256SUMS.DRY-RUN.minisig" \
      -t "DRY RUN, not a release: mosaic $version $git_sha" > /dev/null < /dev/null
    cp "$work/dry-run.pub" "$dir/DRY-RUN-NOT-A-RELEASE-minisign.pub"
    echo "dry-run signed $dir/SHA256SUMS.DRY-RUN.minisig (ephemeral key, not a release)"
    ;;
  *)
    echo "error: unknown mode $mode" >&2
    exit 2
    ;;
esac
