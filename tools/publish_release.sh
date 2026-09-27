#!/bin/sh
# Publish a release built by tools/release.sh as a GitHub release on the
# game's repository (BRINEWAKE_REPO, default halfprice06/brinewake). Every
# launcher picks it up on its next start, through danprice.ai/brinewake or
# straight from GitHub.
#
#   tools/publish_release.sh VERSION
set -eu
cd "$(dirname "$0")/.."
version=${1:?usage: tools/publish_release.sh VERSION}
repo=${BRINEWAKE_REPO:-halfprice06/brinewake}
out="dist/release/$version"
[ -f "$out/latest.json.sig" ] || { echo "no signed release in $out; run tools/release.sh first" >&2; exit 1; }
"target/dist/release/brinewake-release" verify "$out/latest.json"
# The release page links the code signing policy.
body="$out/RELEASE-PAGE.md"
{ sed 's/^/- /' "$out/NOTES.txt"; printf '\nDownload BRINEWAKE-%s-macos.dmg or BRINEWAKE-%s-windows.zip. [Code signing policy](https://github.com/%s/blob/main/docs/CODE-SIGNING.md)\n' "$version" "$version" "$repo"; } > "$body"
gh release create "v$version" --repo "$repo" --title "BRINEWAKE $version" --notes-file "$body" \
  "$out/latest.json" "$out/latest.json.sig" \
  "$out"/BRINEWAKE-"$version"-macos.dmg "$out"/BRINEWAKE-"$version"-windows.zip \
  "$out"/brinewake-game-"$version"-*.zip
echo "Published v$version to https://github.com/$repo/releases/tag/v$version"
