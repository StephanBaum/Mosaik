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
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends minisign
actual="$(minisign -v 2>&1 | head -n 1)"
echo "minisign: $actual"
case "$actual" in
  *"minisign $expected"*) ;;
  *) echo "error: expected minisign $expected, got: $actual" >&2; exit 1 ;;
esac
