#!/bin/sh
# Build a BRINEWAKE release: the Mac app (Developer ID signed, notarized and
# stapled), the Windows zip, the game-only archives the launcher installs,
# and the signed manifest. Nothing is published; see tools/publish_release.sh.
#
#   tools/release.sh VERSION NOTES_FILE [--no-notarize] [--no-windows]
#
# VERSION must match [workspace.package] version in Cargo.toml. NOTES_FILE
# has one release note per line. Needs:
#   - a Developer ID Application certificate in the login keychain (the
#     first one found, or BRINEWAKE_MAC_IDENTITY; another keychain with
#     BRINEWAKE_KEYCHAIN);
#   - notarytool credentials stored as "brinewake-notary"
#     (xcrun notarytool store-credentials brinewake-notary);
#   - the signing key at ~/.config/brinewake/release-signing.key;
#   - for Windows: rustup target x86_64-pc-windows-gnu and MinGW;
#   - cargo-about, for the third-party license notices in every download.
#
# BRINEWAKE_REPO (default halfprice06/brinewake) is the GitHub repository
# whose releases host the files.
#
# Leaves everything in dist/release/VERSION/.
set -eu
cd "$(dirname "$0")/.."

version=${1:?usage: tools/release.sh VERSION NOTES_FILE [--no-notarize] [--no-windows]}
notes=${2:?usage: tools/release.sh VERSION NOTES_FILE [--no-notarize] [--no-windows]}
shift 2
notarize=1
windows=1
for flag in "$@"; do
  case $flag in
    --no-notarize) notarize=0 ;;
    --no-windows) windows=0 ;;
    *) echo "unknown flag $flag" >&2; exit 1 ;;
  esac
done

cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
if [ "$cargo_version" != "$version" ]; then
  echo "Cargo.toml says $cargo_version; set [workspace.package] version = \"$version\" first" >&2
  exit 1
fi
[ -f "$notes" ] || { echo "no notes file $notes" >&2; exit 1; }

identity=${BRINEWAKE_MAC_IDENTITY:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)}
[ -n "$identity" ] || { echo "no Developer ID Application certificate; set BRINEWAKE_MAC_IDENTITY" >&2; exit 1; }
key="$HOME/.config/brinewake/release-signing.key"
repo=${BRINEWAKE_REPO:-halfprice06/brinewake}
base="https://github.com/$repo/releases/download/v$version"
out="dist/release/$version"
work="target/release-work/$version"
rm -rf "$out" "$work"
mkdir -p "$out" "$work"

# Release builds without debugger symbols, in their own target folder so the
# everyday build is left alone.
export CARGO_TARGET_DIR=target/dist
export CARGO_PROFILE_RELEASE_DEBUG=0
export CARGO_PROFILE_RELEASE_STRIP=true
# No local paths inside the shipped binaries.
export RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=."

# The licenses of everything compiled in, for every download.
command -v cargo-about >/dev/null || { echo "install cargo-about (cargo install cargo-about)" >&2; exit 1; }
notices="$work/THIRD-PARTY-LICENSES.html"
cargo about generate --workspace tools/package/about.hbs -o "$notices"

art="art/exports/game-assets.png art/exports/game-assets.json art/exports/ui-icons.png art/exports/ui-icons.json"

plist() { # plist NAME EXECUTABLE IDENTIFIER
  cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>$1</string>
<key>CFBundleDisplayName</key><string>$1</string>
<key>CFBundleExecutable</key><string>$2</string>
<key>CFBundleIdentifier</key><string>$3</string>
<key>CFBundleIconFile</key><string>BRINEWAKE</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
}

# The keychain that holds the identity (the login keychain unless
# BRINEWAKE_KEYCHAIN says otherwise), so another keychain earlier in the
# search list is never asked.
keychain=${BRINEWAKE_KEYCHAIN:-$HOME/Library/Keychains/login.keychain-db}
sign() { codesign --force --options runtime --timestamp --keychain "$keychain" --sign "$identity" "$@"; }

echo "== macOS"
cargo build --release -p bw_desktop --bin brinewake -p bw_launcher --bin brinewake-launcher --bin brinewake-release
tool="$CARGO_TARGET_DIR/release/brinewake-release"

# The game: an app of its own, as installed in the player's data folder.
game="$work/mac-game"
app="$game/BRINEWAKE.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/art/exports"
cp "$CARGO_TARGET_DIR/release/brinewake" "$app/Contents/MacOS/brinewake"
# shellcheck disable=SC2086
cp $art "$app/Contents/Resources/art/exports/"
cp tools/package/BRINEWAKE.icns "$app/Contents/Resources/"
cp LICENSE "$notices" "$app/Contents/Resources/"
plist BRINEWAKE brinewake ai.danprice.brinewake.game > "$app/Contents/Info.plist"
printf '%s\n' "$version" > "$game/VERSION"
sign "$app"

# The launcher app players download, with the game inside it.
launcher="$work/mac/BRINEWAKE.app"
mkdir -p "$launcher/Contents/MacOS" "$launcher/Contents/Resources"
cp "$CARGO_TARGET_DIR/release/brinewake-launcher" "$launcher/Contents/MacOS/brinewake-launcher"
cp tools/package/BRINEWAKE.icns "$launcher/Contents/Resources/"
cp LICENSE "$notices" "$launcher/Contents/Resources/"
cp -R "$game" "$launcher/Contents/Resources/game"
plist BRINEWAKE brinewake-launcher ai.danprice.brinewake > "$launcher/Contents/Info.plist"
sign "$launcher"
codesign --verify --strict --deep "$launcher"

if [ "$notarize" = 1 ]; then
  echo "== notarizing (a few minutes)"
  ditto -c -k --keepParent "$launcher" "$work/notarize.zip"
  xcrun notarytool submit "$work/notarize.zip" --keychain-profile brinewake-notary --wait
  xcrun stapler staple "$launcher"
  spctl --assess --type execute --verbose "$launcher"
else
  echo "== not notarized: Gatekeeper will ask players to allow it"
fi
ditto -c -k --keepParent "$launcher" "$out/BRINEWAKE-$version-macos.zip"
"$tool" pack "$game" "$out/brinewake-game-$version-macos-arm64.zip"

platforms="macos-arm64|macOS (Apple Silicon)|$out/brinewake-game-$version-macos-arm64.zip|$out/BRINEWAKE-$version-macos.zip"

if [ "$windows" = 1 ]; then
  echo "== Windows"
  target=x86_64-pc-windows-gnu
  cargo build --release --target "$target" -p bw_desktop --bin brinewake -p bw_launcher --bin brinewake-launcher
  wgame="$work/win-game"
  mkdir -p "$wgame/art/exports"
  cp "$CARGO_TARGET_DIR/$target/release/brinewake.exe" "$wgame/brinewake.exe"
  # shellcheck disable=SC2086
  cp $art "$wgame/art/exports/"
  printf '%s\r\n' "$version" > "$wgame/VERSION"
  cp LICENSE "$notices" "$wgame/"
  wdl="$work/win/BRINEWAKE"
  mkdir -p "$wdl"
  cp "$CARGO_TARGET_DIR/$target/release/brinewake-launcher.exe" "$wdl/BRINEWAKE.exe"
  cp -R "$wgame" "$wdl/game"
  # Notepad wants CRLF line endings.
  sed 's/$/\r/' tools/package/HOW-TO-PLAY.txt > "$wdl/HOW TO PLAY.txt"
  cp LICENSE "$notices" "$wdl/"
  "$tool" pack "$work/win" "$out/BRINEWAKE-$version-windows.zip"
  "$tool" pack "$wgame" "$out/brinewake-game-$version-windows-x64.zip"
  platforms="$platforms
windows-x64|Windows (64-bit)|$out/brinewake-game-$version-windows-x64.zip|$out/BRINEWAKE-$version-windows.zip"
fi

echo "== manifest"
set --
old_ifs=$IFS
IFS='
'
for line in $platforms; do
  IFS='|'
  # shellcheck disable=SC2086
  set -- "$@" $line
  IFS='
'
done
IFS=$old_ifs
"$tool" manifest "$out/latest.json" "$version" "$(date +%Y-%m-%d)" "$base" "$notes" "$@"
"$tool" sign "$key" "$out/latest.json"
"$tool" verify "$out/latest.json"
cp "$notes" "$out/NOTES.txt"
ls -l "$out"
