#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="0.1.0"
ARCH="$(uname -m)"
NAME="discard-${VERSION}-linux-${ARCH}"
DIST="dist"

bash scripts/build.sh

rm -rf "${DIST}/${NAME}" "${DIST}/${NAME}.tar.gz"
mkdir -p "${DIST}/${NAME}"

install -m 0755 target/release/discard "${DIST}/${NAME}/discard"
install -m 0644 assets/icon.png "${DIST}/${NAME}/discard.png"

cat > "${DIST}/${NAME}/install.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bindir="$HOME/.local/bin"
appdir="$HOME/.local/share/applications"
icondir="$HOME/.local/share/icons/hicolor/512x512/apps"
datadir="$HOME/.local/share/discard"

mkdir -p "$bindir" "$appdir" "$icondir" "$datadir"
install -m 0755 "$here/discard" "$bindir/discard"
install -m 0644 "$here/discard.png" "$icondir/discard.png"

cat > "$bindir/discard-app" <<LAUNCH
#!/usr/bin/env bash
export DISCARD_DATA_DIR="\${DISCARD_DATA_DIR:-$datadir}"
exec "$bindir/discard" "\$@"
LAUNCH
chmod +x "$bindir/discard-app"

cat > "$appdir/discard.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=Discard
Comment=P2P chat, local-first and ephemeral
Exec=$bindir/discard-app
Icon=discard
Terminal=false
Categories=Network;Chat;
DESK

echo "Installed: $bindir/discard"
echo "Data in:   $datadir"
echo "Run:       discard        (make sure ~/.local/bin is on PATH)"
echo "Or open:   Discard        (from your application menu)"
INSTALL

chmod +x "${DIST}/${NAME}/install.sh"
tar -C "${DIST}" -czf "${DIST}/${NAME}.tar.gz" "${NAME}"

echo "OK -> ${DIST}/${NAME}.tar.gz"
