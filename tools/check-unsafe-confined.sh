#!/usr/bin/env bash
# Keeps `unsafe` confined to crates/winshim (ADR-0157 D2).
#   1. Every other workspace member inherits the workspace lints (unsafe_code = "forbid").
#   2. No `expect(unsafe_code` / `allow(unsafe_code` outside the winshim `sys_*` modules.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0

for manifest in crates/*/Cargo.toml apps/desktop/src-tauri/Cargo.toml; do
  [ "$manifest" = "crates/winshim/Cargo.toml" ] && continue
  if ! awk '/^\[lints\]/{in_lints=1; next} /^\[/{in_lints=0} in_lints && /^workspace[[:space:]]*=[[:space:]]*true/{found=1} END{exit !found}' "$manifest"; then
    echo "missing [lints] workspace = true (forbid unsafe_code): $manifest"
    fail=1
  fi
done

# Overrides of the lint anywhere but the shim's sys_* modules. This script and
# the docs are not code.
matches=$(git grep -n -E '(expect|allow|warn)\(unsafe_code' -- '*.rs' ':!crates/winshim/src/sys_*.rs' || true)
if [ -n "$matches" ]; then
  echo "unsafe_code lint override outside crates/winshim/src/sys_*.rs:"
  echo "$matches"
  fail=1
fi

# The shim's own crate-level lint must stay deny, never allow.
if ! grep -q '^unsafe_code = "deny"' crates/winshim/Cargo.toml; then
  echo 'crates/winshim/Cargo.toml must set unsafe_code = "deny"'
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "unsafe is confined to crates/winshim"
