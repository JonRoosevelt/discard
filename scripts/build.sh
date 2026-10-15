#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Build the UI (web/dist)"
(
  cd web
  if [ ! -d node_modules ]; then npm install; fi
  npm run build
)

echo "==> Release binary with the UI embedded"
cargo build --release --features embed-ui

echo "OK -> target/release/discard"
