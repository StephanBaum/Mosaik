#!/usr/bin/env bash
# Verifies a release directory the way docs/VERIFY.md tells users to (M9 T9, CRYPTO_SPEC 17).
#
#   verify.sh <dir> --pubkey-file FILE [--version X.Y.Z] [--dry-run]
#   verify.sh <dir> --pubkey BASE64KEY [--version X.Y.Z]
#
# 1. minisign -Vm SHA256SUMS with the public key (release: SHA256SUMS.minisig;
#    --dry-run: SHA256SUMS.DRY-RUN.minisig, which must start its trusted comment with
#    "DRY RUN" and must not sit next to a release signature).
# 2. The trusted comment names the expected version (when --version is given).
# 3. Every file in SHA256SUMS has the listed SHA-256 (--strict), and every file in <dir> is listed
#    (except SHA256SUMS, the signature checked in step 1 and, in a dry run, the ephemeral public key).
# 4. When present, mosaic-server-image.ref holds a ghcr.io reference pinned by digest.
# Exits non-zero on the first failure.
set -euo pipefail

dir="${1:?usage: verify.sh <dir> --pubkey-file FILE|--pubkey KEY [--version X] [--dry-run]}"
shift
pub_args=()
version=""
dry=0
while [ $# -gt 0 ]; do
  case "$1" in
    --pubkey-file)
      # Absolute before the `cd "$dir"` below: a relative path is relative to the caller (RT-release-1).
      pk="${2:?}"
      case "$pk" in /* | [A-Za-z]:[\/]*) ;; *) pk="$PWD/$pk" ;; esac
      [ -f "$pk" ] || { echo "error: public key file $pk not found" >&2; exit 1; }
      pub_args=(-p "$pk")
      shift 2
      ;;
    --pubkey) pub_args=(-P "${2:?}"); shift 2 ;;
    --version) version="${2:?}"; shift 2 ;;
    --dry-run) dry=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[ "${#pub_args[@]}" -gt 0 ] || { echo "error: a public key is required" >&2; exit 2; }
command -v minisign >/dev/null || { echo "error: minisign is not installed" >&2; exit 1; }

cd "$dir"
if [ "$dry" -eq 1 ]; then
  sig=SHA256SUMS.DRY-RUN.minisig
  [ ! -e SHA256SUMS.minisig ] || { echo "error: a dry run must not have SHA256SUMS.minisig" >&2; exit 1; }
else
  sig=SHA256SUMS.minisig
  [ ! -e SHA256SUMS.DRY-RUN.minisig ] || { echo "error: a release must not hold a dry-run signature" >&2; exit 1; }
fi
[ -f "$sig" ] || { echo "error: $sig is missing" >&2; exit 1; }

out="$(minisign -V -m SHA256SUMS -x "$sig" "${pub_args[@]}" 2>&1)" || { echo "$out" >&2; echo "error: the signature does not verify" >&2; exit 1; }
echo "$out"
comment="$(printf '%s\n' "$out" | sed -n 's/^Trusted comment: //p')"
if [ "$dry" -eq 1 ]; then
  case "$comment" in "DRY RUN"*) ;; *) echo "error: the dry-run comment does not start with DRY RUN" >&2; exit 1 ;; esac
else
  case "$comment" in "DRY RUN"*) echo "error: a release carries a dry-run comment" >&2; exit 1 ;; esac
fi
if [ -n "$version" ]; then
  case "$comment" in
    *"mosaic $version "*) ;;
    *) echo "error: the trusted comment ('$comment') does not name version $version" >&2; exit 1 ;;
  esac
fi

# --strict: an improper line in SHA256SUMS is an error, not a warning (RT-release-2).
if command -v sha256sum >/dev/null; then sha256sum --strict -c SHA256SUMS; else shasum -a 256 --strict -c SHA256SUMS; fi

# Every regular file must be listed (RT-release-5): sha256sum -c only checks listed files, so an
# unlisted file dropped next to a signed SHA256SUMS would ship unsigned. Exempt are SHA256SUMS, the
# signature checked above and, in a dry run only, the ephemeral public key.
listed="$(mktemp)"
trap 'rm -f "$listed"' EXIT
sed -E 's/^[0-9a-fA-F]+ [ *]//' SHA256SUMS > "$listed"
unlisted=0
# NUL-separated, and a name with a control character is refused: a name with a newline would
# otherwise read as two listed names. Only the exact manifest and signature names are exempt
# (not SHA256SUMS* or *.minisig, which would let SHA256SUMS.exe or setup.exe.minisig through).
while IFS= read -r -d '' f; do
  f="${f#./}"
  case "$f" in *[[:cntrl:]]*) echo "error: a file name in the release directory holds a control character" >&2; unlisted=1; continue ;; esac
  case "$f" in SHA256SUMS | "$sig") continue ;; esac
  if [ "$dry" -eq 1 ] && [ "$f" = DRY-RUN-NOT-A-RELEASE-minisign.pub ]; then continue; fi
  if ! grep -Fxq -- "$f" "$listed"; then echo "error: $f is in the release directory but not in SHA256SUMS" >&2; unlisted=1; fi
done < <(find . -maxdepth 1 ! -type d -print0 | LC_ALL=C sort -z)
[ "$unlisted" -eq 0 ] || exit 1

if [ -f mosaic-server-image.ref ]; then
  grep -Eq '^ghcr\.io/[a-z0-9._/-]+@sha256:[0-9a-f]{64}$' mosaic-server-image.ref \
    || { echo "error: mosaic-server-image.ref is not a digest-pinned ghcr.io reference" >&2; exit 1; }
fi
echo "verified: $dir"
