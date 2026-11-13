#!/usr/bin/env bash
# Red-team tests of the release scripts (RT-release, whole-app red team 2026-11-13).
# Complements selftest.sh: the attacks that held are regression tests and run every time; the
# open findings are skipped unless asked for, so CI stays green until they are fixed.
#
#   redteam-selftest.sh                                 the regression tests
#   RT_RELEASE_RUN_KNOWN_FAILING=1 redteam-selftest.sh  also the open findings (expected to FAIL today)
#
# Needs minisign; the pnpm case needs pnpm (it installs a `file:` package offline, no network).
# Keys are ephemeral and thrown away.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
command -v minisign >/dev/null || { echo "error: minisign is not installed" >&2; exit 1; }
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export VERSION=9.9.9 GIT_SHA=0123456789abcdef0123456789abcdef01234567
run_known="${RT_RELEASE_RUN_KNOWN_FAILING:-0}"

passed=0
failed=0
ok() { passed=$((passed + 1)); echo "ok: $1"; }
bad() { failed=$((failed + 1)); echo "FAIL: $1" >&2; }
must_fail() {
  local what="$1"
  shift
  if "$@" > /dev/null 2>&1; then bad "$what unexpectedly succeeded"; else ok "$what fails"; fi
}
must_pass() {
  local what="$1"
  shift
  if "$@" > /dev/null 2>&1; then ok "$what"; else bad "$what"; fi
}
# A known-failing case: runs only on request; a pass there means the finding is fixed.
known() {
  local id="$1" what="$2"
  shift 2
  if [ "$run_known" != 1 ]; then
    echo "skip: $id ($what; open finding, set RT_RELEASE_RUN_KNOWN_FAILING=1 to run)"
    return 0
  fi
  if "$@"; then ok "$id fixed: $what"; else bad "$id still open: $what"; fi
}

fixture() {
  local d="$1" ref="${2:-ghcr.io/example/mosaic-server@sha256:$(printf '%064d' 0)}"
  mkdir -p "$d"
  printf 'one\n' > "$d/mosaic-cli-9.9.9-x86_64-unknown-linux-gnu.tar.gz"
  printf 'two\n' > "$d/mosaic-extension-9.9.9-chromium.zip"
  printf '%s\n' "$ref" > "$d/mosaic-server-image.ref"
  bash "$here/checksums.sh" "$d" > /dev/null
}
minisign -G -W -f -p "$work/rel.pub" -s "$work/rel.key" > /dev/null
release_sign() { MINISIGN_SECRET_KEY="$(cat "$work/rel.key")" MINISIGN_PASSWORD=unused bash "$here/sign.sh" "$1" --release > /dev/null; }

echo "# attacks that held (regression tests)"

# A tag that is not the tree's version, a tag with shell metacharacters, a tag without v.
must_fail "check-versions: a tag that differs from the tree" bash "$here/check-versions.sh" --tag v9.9.8
must_fail "check-versions: a tag with shell metacharacters" bash "$here/check-versions.sh" --tag 'v9.9.8;echo pwned'
must_fail "check-versions: a tag without the v" bash "$here/check-versions.sh" --tag 9.9.8
must_fail "check-versions: no tag and no --allow-untagged" env -u GITHUB_REF_TYPE -u GITHUB_REF_NAME bash "$here/check-versions.sh"

# An artifact name with white space cannot enter SHA256SUMS (sha256sum -c would mis-parse it).
mkdir -p "$work/ws" && printf 'x\n' > "$work/ws/evil name.zip"
must_fail "checksums: a file name with white space" bash "$here/checksums.sh" "$work/ws"

# The image reference must be pinned by digest, a tag is refused after a valid signature.
d="$work/tagref"
fixture "$d" "ghcr.io/example/mosaic-server:9.9.9"
release_sign "$d"
must_fail "verify: an image reference by tag instead of digest" bash "$here/verify.sh" "$d" --pubkey-file "$work/rel.pub" --version 9.9.9

# The version check is on the whole version: 9.9 does not match a 9.9.9 release.
r="$work/rel"
fixture "$r"
release_sign "$r"
must_pass "verify: the signed release verifies" bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub" --version 9.9.9
must_fail "verify: a version prefix (9.9 for 9.9.9)" bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub" --version 9.9
must_fail "verify: another version (9.9.10)" bash "$here/verify.sh" "$r" --pubkey-file "$work/rel.pub" --version 9.9.10

# sign.sh leaves no key material next to the release files, and the trusted comment is exact.
if find "$r" -maxdepth 1 \( -name '*.key' -o -name 'release.key' \) | grep -q .; then
  bad "sign: key material left in the release directory"
else
  ok "sign: no key material in the release directory"
fi
comment="$(sed -n 's/^trusted comment: //p' "$r/SHA256SUMS.minisig")"
[ "$comment" = "mosaic 9.9.9 $GIT_SHA" ] && ok "sign: trusted comment is 'mosaic <version> <sha>'" || bad "sign: trusted comment is '$comment'"

# Editing the trusted comment of a valid signature (a downgrade by comment) breaks the signature.
e="$work/edited"
cp -R "$r" "$e"
sed -i.bak 's/^trusted comment: mosaic 9.9.9 /trusted comment: mosaic 9.9.8 /' "$e/SHA256SUMS.minisig" && rm -f "$e/SHA256SUMS.minisig.bak"
must_fail "verify: an edited trusted comment" bash "$here/verify.sh" "$e" --pubkey-file "$work/rel.pub" --version 9.9.8

# A signature made with the release key but a DRY RUN comment is refused as a release.
m="$work/mixed"
fixture "$m"
minisign -S -s "$work/rel.key" -m "$m/SHA256SUMS" -x "$m/SHA256SUMS.minisig" -t "DRY RUN, not a release: mosaic 9.9.9 $GIT_SHA" > /dev/null < /dev/null
must_fail "verify: a release signature whose comment says DRY RUN" bash "$here/verify.sh" "$m" --pubkey-file "$work/rel.pub" --version 9.9.9

# A release signature over a SHA256SUMS that lists a file whose bytes changed afterwards.
s="$work/swap"
fixture "$s"
release_sign "$s"
printf 'swapped\n' > "$s/mosaic-extension-9.9.9-chromium.zip"
must_fail "verify: a swapped artifact of the right name" bash "$here/verify.sh" "$s" --pubkey-file "$work/rel.pub" --version 9.9.9

echo "# open findings"

# RT-release-1: release.yml calls verify.sh with a relative --pubkey-file (release/minisign.pub and
# release-files/DRY-RUN-NOT-A-RELEASE-minisign.pub); verify.sh does `cd "$dir"` first, so the key is
# looked up under the release directory and the job's own verification step fails.
rt1() {
  local base="$work/rt1"
  mkdir -p "$base" && (
    cd "$base"
    fixture release-files
    bash "$here/sign.sh" release-files --dry-run > /dev/null
    bash "$here/verify.sh" release-files --pubkey-file release-files/DRY-RUN-NOT-A-RELEASE-minisign.pub --version 9.9.9 --dry-run > /dev/null 2>&1
  )
}
known RT-release-1 "verify.sh with a relative --pubkey-file, as release.yml calls it" rt1

# RT-release-2: checksums.sh passes file names to sha256sum without `--`; a file named `--help`
# writes the help text into SHA256SUMS, exits 0, and ships unlisted (sha256sum -c without --strict
# only warns about the improper lines).
rt2() {
  local d="$work/rt2"
  mkdir -p "$d"
  printf 'real\n' > "$d/mosaic-cli-9.9.9-x86_64-unknown-linux-gnu.tar.gz"
  printf 'evil\n' > "$d/--help"
  if bash "$here/checksums.sh" "$d" > /dev/null 2>&1; then
    # Accepted: then the file must be listed and no foreign line may be present.
    grep -q '  --help$' "$d/SHA256SUMS" && ! grep -q 'Usage' "$d/SHA256SUMS"
  else
    true # refusing the name is also fine
  fi
}
known RT-release-2 "checksums.sh with an artifact named --help" rt2

# RT-release-5: a regular file in the release directory that SHA256SUMS does not list goes unnoticed
# (sha256sum -c only checks listed files). The release job runs verify.sh on its own output, so an
# unlisted file dropped by a build job would ship unsigned.
rt5() {
  local d="$work/rt5"
  fixture "$d"
  release_sign "$d"
  printf 'unlisted\n' > "$d/mosaic-9.9.9-extra.msi"
  ! bash "$here/verify.sh" "$d" --pubkey-file "$work/rel.pub" --version 9.9.9 > /dev/null 2>&1
}
known RT-release-5 "verify.sh accepts a release directory with an unlisted file" rt5

# RT-release-3: THREAT_MODEL 4.9 says dependency lifecycle scripts are off (`onlyBuiltDependencies`).
# The repository pins pnpm 9.15.4 (package.json packageManager) and puts the key into
# pnpm-workspace.yaml, which pnpm 9 does not read for settings: a dependency's postinstall runs.
rt3() {
  command -v pnpm > /dev/null || { echo "pnpm is not installed" >&2; return 1; }
  local base="$work/rt3" marker
  marker="$base/marker.txt"
  mkdir -p "$base/dep" "$base/proj"
  cat > "$base/dep/package.json" <<'EOF'
{ "name": "rt-dep", "version": "1.0.0", "scripts": { "postinstall": "node -e \"require('fs').writeFileSync(process.env.RT_MARKER, 'lifecycle ran')\"" } }
EOF
  # The repository's own settings: pnpm-workspace.yaml (with its packages list replaced) and the
  # root package.json `pnpm` field, if any.
  node -e '
    const fs = require("fs");
    const [root, out] = process.argv.slice(1);
    const ws = fs.readFileSync(root + "/pnpm-workspace.yaml", "utf8").replace(/^packages:[\s\S]*?(?=^\S)/m, "packages:\n  - \".\"\n");
    fs.writeFileSync(out + "/pnpm-workspace.yaml", ws);
    const pkg = JSON.parse(fs.readFileSync(root + "/package.json", "utf8"));
    const proj = { name: "rt-proj", private: true, packageManager: pkg.packageManager, dependencies: { "rt-dep": "file:../dep" } };
    if (pkg.pnpm) proj.pnpm = pkg.pnpm;
    fs.writeFileSync(out + "/package.json", JSON.stringify(proj, null, 2));
  ' "$root" "$base/proj"
  (cd "$base/proj" && RT_MARKER="$marker" pnpm install --offline > /dev/null 2>&1) || return 1
  [ ! -e "$marker" ]
}
known RT-release-3 "a dependency postinstall runs under the repository's pnpm settings" rt3

echo
echo "redteam-selftest: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
