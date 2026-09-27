#!/bin/sh
# Local, self-contained application bundle. No publishing or installer changes.
set -eu
cd "$(dirname "$0")/.."
if [ "${1-}" != "--reuse-build" ]; then
  cargo build --release -p bw_desktop --bin brinewake
fi
bundle="dist/BRINEWAKE.app"
version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources/art/exports"
cp target/release/brinewake "$bundle/Contents/MacOS/brinewake"
cp art/exports/game-assets.png art/exports/game-assets.json \
  art/exports/ui-icons.png art/exports/ui-icons.json "$bundle/Contents/Resources/art/exports/"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>BRINEWAKE</string>
<key>CFBundleDisplayName</key><string>BRINEWAKE</string>
<key>CFBundleExecutable</key><string>brinewake</string>
<key>CFBundleIdentifier</key><string>local.brinewake.game</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$bundle"
printf 'Built %s\n' "$bundle"
