#!/usr/bin/env bash
# Installs minisign from the Ubuntu package and checks its version (ADR-0081).
#
#   install-minisign.sh
#
# The version is logged and must equal MINISIGN_EXPECTED_VERSION (default 0.11, the version of
# Ubuntu 24.04; jobs that sign run on ubuntu-24.04, not ubuntu-latest). Change the expected
# version deliberately, together with the runner image.
set -euo pipefail

expected="${MINISIGN_EXPECTED_VERSION:-0.11}"
# The exact Debian package version (M9-F9): Ubuntu 24.04 (noble) ships minisign 0.11-1. A newer
# package in the archive makes apt fail here instead of changing the tool that signs the release.
expected_deb="${MINISIGN_EXPECTED_DEB_VERSION:-0.11-1}"
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends "minisign=${expected_deb}"
actual="$(minisign -v 2>&1 | head -n 1)"
echo "minisign: $actual"
# Exact match on the version word, not a substring ("minisign 0.110" must not pass for 0.11).
if [ "$actual" != "minisign $expected" ]; then
  echo "error: expected \"minisign $expected\", got: $actual" >&2
  exit 1
fi
