#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="0.1.0"
ARCH="$(uname -m)"
NAME="cadence-${VERSION}-linux-${ARCH}"
DIST="dist"

bash scripts/build.sh

rm -rf "${DIST}/${NAME}" "${DIST}/${NAME}.tar.gz"
mkdir -p "${DIST}/${NAME}"

install -m 0755 target/release/cadence "${DIST}/${NAME}/cadence"
install -m 0644 assets/icon.png "${DIST}/${NAME}/cadence.png"

cat > "${DIST}/${NAME}/install.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bindir="$HOME/.local/bin"
appdir="$HOME/.local/share/applications"
icondir="$HOME/.local/share/icons/hicolor/512x512/apps"
datadir="$HOME/.local/share/cadence"

mkdir -p "$bindir" "$appdir" "$icondir" "$datadir"
install -m 0755 "$here/cadence" "$bindir/cadence"
install -m 0644 "$here/cadence.png" "$icondir/cadence.png"

cat > "$bindir/cadence-app" <<LAUNCH
#!/usr/bin/env bash
export CADENCE_DATA_DIR="\${CADENCE_DATA_DIR:-$datadir}"
exec "$bindir/cadence" "\$@"
LAUNCH
chmod +x "$bindir/cadence-app"

cat > "$appdir/cadence.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=Cadence
Comment=Chat P2P, local-first e efêmero
Exec=$bindir/cadence-app
Icon=cadence
Terminal=false
Categories=Network;Chat;
DESK

echo "Instalado: $bindir/cadence"
echo "Dados em:  $datadir"
echo "Execute:   cadence          (garanta ~/.local/bin no PATH)"
echo "Ou abra:   Cadence          (no menu de aplicativos)"
INSTALL

chmod +x "${DIST}/${NAME}/install.sh"
tar -C "${DIST}" -czf "${DIST}/${NAME}.tar.gz" "${NAME}"

echo "OK -> ${DIST}/${NAME}.tar.gz"
