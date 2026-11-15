#!/usr/bin/env bash
# Red-team tests of the release scripts (RT-release, whole-app red team 2026-11-13).
# Complements selftest.sh. Every case runs every time: the attacks that held, and the
# findings that are now fixed (RT-release-1 to 5), which stay as regression tests. CI runs it.
#
#   redteam-selftest.sh
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
# A case for a fixed finding: a pass means the fix still holds.
known() {
  local id="$1" what="$2"
  shift 2
  if "$@"; then ok "$id fixed: $what"; else bad "$id regressed: $what"; fi
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

echo "# fixed findings (regression tests)"

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
# The exemptions are exact names: an unlisted SHA256SUMS.exe or setup.exe.minisig, or a name with a
# newline made of two listed names, must not pass either (found when verifying the RT-release fixes).
rt5_name() {
  local d="$work/rt5-$1"
  fixture "$d"
  release_sign "$d"
  printf 'unlisted\n' > "$d/$2"
  ! bash "$here/verify.sh" "$d" --pubkey-file "$work/rel.pub" --version 9.9.9 > /dev/null 2>&1
}
known RT-release-5 "verify.sh exempts SHA256SUMS* by prefix" rt5_name a SHA256SUMS.exe
known RT-release-5 "verify.sh exempts any *.minisig" rt5_name b mosaic-setup.exe.minisig
known RT-release-5 "verify.sh reads a newline name as two listed names" rt5_name c \
  $'mosaic-cli-9.9.9-x86_64-unknown-linux-gnu.tar.gz\nmosaic-extension-9.9.9-chromium.zip'

# RT-release-3: THREAT_MODEL 4.9 says dependency lifecycle scripts are off (`onlyBuiltDependencies`).
# The repository pins pnpm 9.15.4 (package.json packageManager), which reads the key from the root
# package.json `pnpm` field and ignores pnpm-workspace.yaml (pnpm 10 reads the latter); both are set.
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

# RT2-CHAIN-2: check-features.sh also reads the desktop's Tauri configs and its feature lists.
chain2() {
  local base="$work/chain2" case_
  mkdir -p "$base/scripts/release" "$base/apps/desktop/src-tauri" "$base/crates/cli"     "$base/crates/server" "$base/crates/native-messaging" "$base/deploy" "$base/.github/workflows"
  cp "$here/check-features.sh" "$base/scripts/release/"
  for c in cli server native-messaging; do printf '[features]
' > "$base/crates/$c/Cargo.toml"; done
  : > "$base/deploy/Dockerfile"
  : > "$base/.github/workflows/release.yml"
  printf '[features]
custom-protocol = ["tauri/custom-protocol"]
preview = ["mosaic-native-messaging/preview"]
'     > "$base/apps/desktop/src-tauri/Cargo.toml"
  printf '{"build": {"beforeBuildCommand": "pnpm build"}}
' > "$base/apps/desktop/src-tauri/tauri.conf.json"
  printf '{}
' > "$base/apps/desktop/src-tauri/tauri.release.conf.json"
  bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1 || return 1
  for case_ in features before custom; do
    cp "$base/apps/desktop/src-tauri/Cargo.toml" "$base/Cargo.keep"
    cp "$base/apps/desktop/src-tauri/tauri.conf.json" "$base/tauri.keep"
    case "$case_" in
      features) printf '{"build": {"features": ["preview"]}}
' > "$base/apps/desktop/src-tauri/tauri.conf.json" ;;
      before) printf '{"build": {"beforeBuildCommand": "pnpm build && pnpm build:preview"}}
' > "$base/apps/desktop/src-tauri/tauri.conf.json" ;;
      custom) sed -i 's|^custom-protocol = \[|custom-protocol = ["preview", |' "$base/apps/desktop/src-tauri/Cargo.toml" ;;
    esac
    if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
    cp "$base/Cargo.keep" "$base/apps/desktop/src-tauri/Cargo.toml"
    cp "$base/tauri.keep" "$base/apps/desktop/src-tauri/tauri.conf.json"
  done
}
known RT2-CHAIN-2 "the release guard catches preview in tauri.conf.json and in the desktop feature lists" chain2

# FIX-1: the [features] header spellings Cargo accepts, a missing header, and a --cfg feature set
# from a .cargo config all fail the guard.
chain2_headers() {
  local base="$work/chain2h" hdr
  mkdir -p "$base/scripts/release" "$base/apps/desktop/src-tauri" "$base/crates/cli" "$base/crates/server" "$base/crates/native-messaging" "$base/deploy" "$base/.github/workflows"
  cp "$here/check-features.sh" "$base/scripts/release/"
  for c in cli server native-messaging; do printf '[features]
' > "$base/crates/$c/Cargo.toml"; done
  : > "$base/deploy/Dockerfile"
  : > "$base/.github/workflows/release.yml"
  printf '{}
' > "$base/apps/desktop/src-tauri/tauri.conf.json"
  printf '{}
' > "$base/apps/desktop/src-tauri/tauri.release.conf.json"
  write_desktop() { printf '%s
custom-protocol = ["tauri/custom-protocol"]
preview = []
' "$1" > "$base/apps/desktop/src-tauri/Cargo.toml"; }
  write_desktop '[features]'
  bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1 || return 1
  # The other spellings with a clean list still pass (the header is found), and with preview in
  # custom-protocol they fail.
  for hdr in '[ features ]' '["features"]' '[features] # the list' '[  "features"  ]'; do
    write_desktop "$hdr"
    bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1 || return 1
    printf '%s
custom-protocol = ["tauri/custom-protocol", "preview"]
preview = []
' "$hdr" > "$base/apps/desktop/src-tauri/Cargo.toml"
    if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
    # A default feature list under the same header is seen as well.
    printf '%s
default = ["x"]
' "$hdr" > "$base/crates/cli/Cargo.toml"
    write_desktop "$hdr"
    if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
    printf '[features]
' > "$base/crates/cli/Cargo.toml"
  done
  # No header at all in a manifest that is expected to have one.
  write_desktop '# nothing'
  printf 'x = 1
' > "$base/apps/desktop/src-tauri/Cargo.toml"
  if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
  write_desktop '[features]'
  # A feature cfg from the cargo config, in the usual and in the multi-line spelling.
  mkdir -p "$base/.cargo"
  printf '[build]
rustflags = ["--cfg", "feature=\\"preview\\""]
' > "$base/.cargo/config.toml"
  if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
  printf '%s\n' '[build]' 'rustflags = [' '  "--cfg",' "  'feature=\"preview\"'," ']' > "$base/.cargo/config.toml"
  if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
  rm -f "$base/.cargo/config.toml"
  # A build script printing the cfg.
  printf 'fn main() { println!("cargo:rustc-cfg=feature=\\"preview\\""); }
' > "$base/crates/native-messaging/build.rs"
  if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
  rm -f "$base/crates/native-messaging/build.rs"
  # Encoded rustflags in the release workflow.
  printf 'env:
  CARGO_ENCODED_RUSTFLAGS: x
' > "$base/.github/workflows/release.yml"
  if bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1; then return 1; fi
  : > "$base/.github/workflows/release.yml"
  bash "$base/scripts/release/check-features.sh" > /dev/null 2>&1
}
known RT2-CHAIN-2b "header spellings, a missing header and feature cfgs from outside the manifests are caught" chain2_headers

echo
echo "redteam-selftest: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
