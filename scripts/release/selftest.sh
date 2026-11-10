#!/usr/bin/env bash
# Self-test of the release scripts with fixture files and ephemeral keys (M9 T9).
#
#   selftest.sh
#
# checksums.sh -> sign.sh -> verify.sh on a fixture directory, in dry-run and release mode,
# then the failure cases: a tampered file, a tampered SHA256SUMS, the wrong key, a missing
# secret, and mixed dry-run and release signatures. Needs minisign; runs in CI (the release
# workflow and PRs that touch scripts/release). The keys are made here and thrown away.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
command -v minisign >/dev/null || { echo "error: minisign is not installed" >&2; exit 1; }
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export VERSION=9.9.9 GIT_SHA=0123456789abcdef0123456789abcdef01234567

passed=0
ok() { passed=$((passed + 1)); echo "ok: $1"; }
must_fail() {
  local what="$1"
  shift
  if "$@" > /dev/null 2>&1; then
    echo "FAIL: $what unexpectedly succeeded" >&2
    exit 1
  fi
  ok "$what fails"
}

fixture() {
  local d="$1"
  mkdir -p "$d"
  printf 'one\n' > "$d/mosaic-cli-9.9.9-x86_64-unknown-linux-gnu.tar.gz"
  printf 'two\n' > "$d/mosaic-extension-9.9.9-chromium.zip"
  printf 'ghcr.io/example/mosaic-server@sha256:%064d\n' 0 > "$d/mosaic-server-image.ref"
  bash "$here/checksums.sh" "$d" > /dev/null
}

# Dry run: sign, then verify with the ephemeral public key.
d="$work/dry"
fixture "$d"
bash "$here/sign.sh" "$d" --dry-run > /dev/null
[ -f "$d/SHA256SUMS.DRY-RUN.minisig" ] && [ ! -e "$d/SHA256SUMS.minisig" ] || { echo "FAIL: dry-run file names" >&2; exit 1; }
bash "$here/verify.sh" "$d" --pubkey-file "$d/DRY-RUN-NOT-A-RELEASE-minisign.pub" --version 9.9.9 --dry-run > /dev/null
ok "dry run signs and verifies"
must_fail "a dry-run signature as a release" bash "$here/verify.sh" "$d" --pubkey-file "$d/DRY-RUN-NOT-A-RELEASE-minisign.pub"
printf 'tampered\n' >> "$d/mosaic-extension-9.9.9-chromium.zip"
must_fail "a tampered artifact" bash "$here/verify.sh" "$d" --pubkey-file "$d/DRY-RUN-NOT-A-RELEASE-minisign.pub" --dry-run
d2="$work/dry2"
fixture "$d2"
bash "$here/sign.sh" "$d2" --dry-run > /dev/null
printf '%064d  extra-file\n' 0 >> "$d2/SHA256SUMS"
must_fail "a tampered SHA256SUMS" bash "$here/verify.sh" "$d2" --pubkey-file "$d2/DRY-RUN-NOT-A-RELEASE-minisign.pub" --dry-run
d3="$work/dry3"
fixture "$d3"
bash "$here/sign.sh" "$d3" --dry-run > /dev/null
must_fail "the wrong key" bash "$here/verify.sh" "$d3" --pubkey-file "$d2/DRY-RUN-NOT-A-RELEASE-minisign.pub" --dry-run

# Release mode with a throw-away key (unencrypted here; CI keys are encrypted).
minisign -G -W -f -p "$work/rel.pub" -s "$work/rel.key" > /dev/null
r="$work/rel"
fixture "$r"
must_fail "a release without secrets" env -u MINISIGN_SECRET_KEY -u MINISIGN_PASSWORD bash "$here/sign.sh" "$r" --release
[ ! -e "$r/SHA256SUMS.minisig" ] || { echo "FAIL: a failed release left a signature" >&2; exit 1; }
MINISIGN_SECRET_KEY="$(cat "$work/rel.key")" MINISIGN_PASSWORD=unused bash "$here/sign.sh" "$r" --release > /dev/null
bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub" --version 9.9.9 > /dev/null
ok "release signs and verifies"
must_fail "the wrong version" bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub" --version 9.9.8
must_fail "a release under a dry-run key" bash "$here/verify.sh" "$r" --pubkey-file "$d3/DRY-RUN-NOT-A-RELEASE-minisign.pub"
printf 'x\n' >> "$r/mosaic-cli-9.9.9-x86_64-unknown-linux-gnu.tar.gz"
must_fail "a tampered release artifact" bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub"
must_fail "a dry run next to a release signature" bash "$here/sign.sh" "$r" --dry-run

# checksums.sh: sorted, excludes manifests and signatures, and the unsigned manifest name.
c="$work/sums"
mkdir -p "$c"
printf 'b\n' > "$c/b.bin"
printf 'a\n' > "$c/a.bin"
printf 'x\n' > "$c/x.minisig"
bash "$here/checksums.sh" "$c" > /dev/null
bash "$here/checksums.sh" "$c" --out SHA256SUMS.unsigned > /dev/null
[ "$(awk '{print $2}' "$c/SHA256SUMS" | tr '\n' ' ')" = "a.bin b.bin " ] || { echo "FAIL: checksums order or content" >&2; exit 1; }
ok "checksums are sorted and skip signatures and manifests"

echo "selftest passed ($passed checks)"
