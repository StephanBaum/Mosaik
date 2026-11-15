#!/usr/bin/env bash
# Keeps `unsafe` confined to crates/winshim (ADR-0157 D2).
#   1. Every other manifest, nested ones included, forbids unsafe_code (workspace lints or its own).
#   2. No `expect(unsafe_code` / `allow(unsafe_code` outside the winshim `sys_*` modules.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0

# Every tracked package manifest, nested ones (crates/*/fuzz, own workspaces)
# included (vendored third-party crates excluded): it inherits the workspace lints (root-workspace members only), or sets unsafe_code = "forbid" itself.
while IFS= read -r manifest; do
  [ "$manifest" = "crates/winshim/Cargo.toml" ] && continue
  grep -q '^\[package\]' "$manifest" || continue
  # `workspace = true` only counts for a member of the root workspace; a nested
  # manifest that is its own workspace (has [workspace]) must set the lint itself.
  if ! grep -q '^\[workspace\]' "$manifest" && awk '/^\[lints\]/{in_lints=1; next} /^\[/{in_lints=0} in_lints && /^workspace[[:space:]]*=[[:space:]]*true/{found=1} END{exit !found}' "$manifest"; then
    continue
  fi
  if awk '/^\[lints\.rust\]/{in_lints=1; next} /^\[/{in_lints=0} in_lints && /^unsafe_code[[:space:]]*=[[:space:]]*"forbid"/{found=1} END{exit !found}' "$manifest"; then
    continue
  fi
  echo "missing [lints] workspace = true or unsafe_code = \"forbid\": $manifest"
  fail=1
done < <(git ls-files -- 'Cargo.toml' '*/Cargo.toml' ':!vendor')

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
