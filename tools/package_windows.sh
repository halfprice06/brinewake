#!/bin/sh
# A Windows download friends unzip and run: brinewake.exe beside the art it
# reads, and a note on joining online. Cross-built with MinGW from Linux or
# macOS; no installer, nothing published.
#
#   rustup target add x86_64-pc-windows-gnu
#   apt-get install gcc-mingw-w64-x86-64-win32   (or: brew install mingw-w64)
#   tools/package_windows.sh
#
# Leaves dist/BRINEWAKE-windows/ and dist/BRINEWAKE-windows.zip.
set -eu
cd "$(dirname "$0")/.."
target=x86_64-pc-windows-gnu
if [ "${1-}" != "--reuse-build" ]; then
  # Friends need no debugger symbols: a smaller download.
  CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_STRIP=true \
    cargo build --release --target "$target" -p bw_desktop --bin brinewake
fi
name=BRINEWAKE-windows
out="dist/$name"
rm -rf "$out" "dist/$name.zip"
mkdir -p "$out/art/exports"
cp "target/$target/release/brinewake.exe" "$out/BRINEWAKE.exe"
cp art/exports/game-assets.png art/exports/game-assets.json \
  art/exports/ui-icons.png art/exports/ui-icons.json "$out/art/exports/"
# Notepad wants CRLF line endings.
sed 's/$/\r/' tools/package/HOW-TO-PLAY.txt > "$out/HOW TO PLAY.txt"
(cd dist && zip -qr "$name.zip" "$name")
printf 'Built dist/%s.zip\n' "$name"
