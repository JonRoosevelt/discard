#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Build da UI (web/dist)"
(
  cd web
  if [ ! -d node_modules ]; then npm install; fi
  npm run build
)

echo "==> Binário release com a UI embutida"
cargo build --release --features embed-ui

echo "OK -> target/release/discard"
