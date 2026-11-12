#!/usr/bin/env bash
# Signs the macOS CLI binaries, builds the installer package and notarizes it (M9 T9, ADR-0082).
#
#   macos-sign-cli.sh <bindir> <outdir> <version> <arch>
#
# <bindir> holds `mosaic` and `mosaic-native-host`. Signs both with Developer ID, the hardened
# runtime and a secure timestamp, then packs `mosaic` into mosaic-<version>-<arch>.pkg
# (installs /usr/local/bin/mosaic, root-owned), signs the package with the Developer ID
# Installer identity, notarizes it with notarytool --wait and staples the ticket. The signed
# binaries stay in <bindir>.
#
# Environment (all required; the .p12 holds the application and the installer identities):
#   APPLE_CERTIFICATE (base64 .p12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY,
#   APPLE_INSTALLER_IDENTITY, APPLE_API_ISSUER, APPLE_API_KEY (key ID), APPLE_API_KEY_P8
#   (content of the .p8 file).
# The secrets are never printed and this script never turns on tracing.
set -euo pipefail

bindir="$(cd "${1:?usage: macos-sign-cli.sh <bindir> <outdir> <version> <arch>}" && pwd)"
mkdir -p "${2:?outdir}"
outdir="$(cd "$2" && pwd)"
version="${3:?version}"
arch="${4:?arch}"
for v in APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY APPLE_INSTALLER_IDENTITY \
  APPLE_API_ISSUER APPLE_API_KEY APPLE_API_KEY_P8; do
  [ -n "${!v:-}" ] || { echo "error: $v is not set" >&2; exit 1; }
done

work="$(mktemp -d)"
keychain="$work/mosaic-release.keychain-db"
cleanup() {
  security delete-keychain "$keychain" > /dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
umask 077

kc_password="$(openssl rand -hex 24)"
printf '%s' "$APPLE_CERTIFICATE" | base64 --decode > "$work/cert.p12"
security create-keychain -p "$kc_password" "$keychain"
security set-keychain-settings -lut 3600 "$keychain"
security unlock-keychain -p "$kc_password" "$keychain"
security import "$work/cert.p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign -T /usr/bin/productsign > /dev/null
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$kc_password" "$keychain" > /dev/null
security list-keychains -d user -s "$keychain" login.keychain-db
printf '%s\n' "$APPLE_API_KEY_P8" > "$work/AuthKey.p8"

for b in mosaic mosaic-native-host; do
  codesign --force --options runtime --timestamp --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$bindir/$b"
  codesign --verify --strict --verbose=2 "$bindir/$b"
done

root="$work/pkgroot"
mkdir -p "$root/usr/local/bin"
install -m 0755 "$bindir/mosaic" "$root/usr/local/bin/mosaic"
pkgbuild --root "$root" --identifier io.github.stephanbaum.mosaic.cli --version "$version" \
  --install-location / "$work/unsigned.pkg" > /dev/null
pkg="$outdir/mosaic-$version-$arch.pkg"
productsign --keychain "$keychain" --sign "$APPLE_INSTALLER_IDENTITY" "$work/unsigned.pkg" "$pkg"

xcrun notarytool submit "$pkg" --key "$work/AuthKey.p8" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" --wait
xcrun stapler staple "$pkg"
xcrun stapler validate "$pkg"
echo "wrote $pkg"
