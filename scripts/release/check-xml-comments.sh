#!/usr/bin/env bash
# XML forbids "--" inside a comment and a comment ending in "-". WiX (the MSI) and the Android
# resource merger both refuse such a file, and the ordinary CI builds neither the MSI nor the
# release APK, so this check catches it before a release run does.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
bad=$(git ls-files -z '*.wxs' '*.wxi' '*.xml' '*.plist' '*.storyboard' '*.xib' '*.svg' |
  xargs -0 -r perl -0777 -ne 'while (/<!--(.*?)-->/sg) { my $c = $1; if ($c =~ /--/ || $c =~ /-$/) { print "$ARGV\n"; last } }')
if [ -n "$bad" ]; then
  printf '::error::an XML comment contains "--" or ends in "-": %s\n' $bad
  exit 1
fi
echo "ok: no XML comment contains a double hyphen"
