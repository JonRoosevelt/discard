#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

APP_NAME="Discard"
APP_ID="app.discard.desktop"
VERSION="${VERSION:-0.1.0}"
APP_DIR="dist/macos/${APP_NAME}.app"
DMG="dist/${APP_NAME}-${VERSION}-macos.dmg"

die() { echo "error: $*" >&2; exit 1; }

if [ "$(uname -s)" != "Darwin" ]; then
  die "scripts/package-macos.sh only runs on macOS (uname -s = $(uname -s)); use a macOS machine or the CI macos job"
fi

[ -f assets/icon.png ] || die "assets/icon.png not found"

if [ ! -f web/dist/index.html ]; then
  echo "web/dist missing, building the UI (npm ci && npm run build)..."
  ( cd web && npm ci && npm run build )
fi

if [ ! -f target/release/discard ] && [ ! -f target/release/discard-server ]; then
  echo "release binary missing, building (cargo build --release --features embed-ui)..."
  cargo build --release --features embed-ui
fi

if [ -f target/release/discard ]; then
  BIN_SRC="target/release/discard"
elif [ -f target/release/discard-server ]; then
  BIN_SRC="target/release/discard-server"
else
  die "release binary not found; run: cargo build --release --features embed-ui"
fi

rm -rf dist/macos
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp "$BIN_SRC" "$APP_DIR/Contents/MacOS/discard-bin"
chmod +x "$APP_DIR/Contents/MacOS/discard-bin"

cat > "$APP_DIR/Contents/MacOS/Discard" <<'LAUNCHER'
#!/bin/bash
set -euo pipefail
export DISCARD_DATA_DIR="$HOME/Library/Application Support/Discard"
mkdir -p "$DISCARD_DATA_DIR"
DIR="$(cd "$(dirname "$0")" && pwd)"
exec "$DIR/discard-bin" "$@"
LAUNCHER
chmod +x "$APP_DIR/Contents/MacOS/Discard"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>${APP_ID}</string>
  <key>CFBundleExecutable</key>
  <string>${APP_NAME}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>Discard uses the microphone for voice in channel calls.</string>
  <key>NSCameraUsageDescription</key>
  <string>Discard uses the camera for video in channel calls.</string>
</dict>
</plist>
PLIST

ICONSET="$(mktemp -d)/icon.iconset"
mkdir -p "$ICONSET"
sips -z 16 16 assets/icon.png --out "$ICONSET/icon_16x16.png" >/dev/null
sips -z 32 32 assets/icon.png --out "$ICONSET/icon_16x16@2x.png" >/dev/null
sips -z 32 32 assets/icon.png --out "$ICONSET/icon_32x32.png" >/dev/null
sips -z 64 64 assets/icon.png --out "$ICONSET/icon_32x32@2x.png" >/dev/null
sips -z 128 128 assets/icon.png --out "$ICONSET/icon_128x128.png" >/dev/null
sips -z 256 256 assets/icon.png --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256 assets/icon.png --out "$ICONSET/icon_256x256.png" >/dev/null
sips -z 512 512 assets/icon.png --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512 assets/icon.png --out "$ICONSET/icon_512x512.png" >/dev/null
sips -z 1024 1024 assets/icon.png --out "$ICONSET/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/icon.icns"
rm -rf "$(dirname "$ICONSET")"

if command -v codesign >/dev/null 2>&1; then
  if codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1; then
    echo "ad-hoc signature applied"
  else
    echo "warning: ad-hoc signing failed; open with right-click > Open"
  fi
fi

echo "warning: ad-hoc signing is NOT for distribution; to publish use a Developer ID certificate"
echo "         (codesign --sign 'Developer ID Application: ...') then notarization (xcrun notarytool) and staple."

mkdir -p dist
hdiutil create -volname "$APP_NAME" -srcfolder "$APP_DIR" -ov -format UDZO "$DMG"

echo "done: $APP_DIR"
echo "done: $DMG"
