#!/usr/bin/env bash
# Guards the iOS dependency tree (docs/MOBILE.md, spec 3.16): no third-party Swift packages, no CocoaPods.
#
#   check-ios-deps.sh
#
# Fails if apps/mobile/ios/project.yml declares a package that is not a local path (a url, github,
# version or branch key), if a Podfile, Cartfile, Package.swift, Package.resolved or Pods directory is
# present, or if a generated .xcodeproj references a remote Swift package. The allowlist is empty: the
# only Swift code is the repository's own (MosaicKit and the generated UniFFI bindings).
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-ios-deps.sh

Run from anywhere; it needs only grep and find. Exit code 0: clean; 1: a third-party dependency; 2: usage.
USAGE
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
ios=apps/mobile/ios
status=0

spec="$ios/project.yml"
if [ ! -f "$spec" ]; then
  echo "error: $spec is missing" >&2
  exit 1
fi
if grep -nE '^[[:space:]]*(url|github|branch|revision|from|exactVersion|minorVersion|majorVersion|minVersion|maxVersion|version):' "$spec"; then
  echo "error: $spec declares a remote Swift package (only local path packages are allowed)" >&2
  status=1
fi
if grep -nE '^[[:space:]]*(package|carthage|sdk):[[:space:]]*[A-Za-z]' "$spec" | grep -vE 'sdk:[[:space:]]*(AuthenticationServices|Security|LocalAuthentication)'; then
  echo "error: $spec declares a package, Carthage or SDK dependency outside the allowlist" >&2
  status=1
fi

found="$(find "$ios" \( -name Podfile -o -name Podfile.lock -o -name Cartfile -o -name Package.swift -o -name Package.resolved -o -name Pods \) -not -path '*/build/*' 2>/dev/null || true)"
if [ -n "$found" ]; then
  echo "$found"
  echo "error: CocoaPods, Carthage or SwiftPM files are not allowed in $ios" >&2
  status=1
fi

for project in "$ios"/*.xcodeproj/project.pbxproj; do
  [ -f "$project" ] || continue
  if grep -nE 'XCRemoteSwiftPackageReference|XCSwiftPackageProductDependency' "$project"; then
    echo "error: $project references a Swift package" >&2
    status=1
  fi
done

if [ "$status" -eq 0 ]; then
  echo "ok: no third-party Swift packages, CocoaPods or Carthage in $ios"
fi
exit "$status"
