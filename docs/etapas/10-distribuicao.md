# Stage 10 — Distribution (single binary) and packaging

## Goal

Turn the local node into a **single-file app** (the UI embedded in the binary), which **opens the
browser by itself**, and generate installers for **Linux (omarchy/Arch)** and **macOS**.
See `docs/DIST.md` for the user guide.

## Core (tech lead)

- **Embedded UI:** optional `embed-ui` feature (`crates/server/Cargo.toml`) using `rust-embed`
  (with `interpolate-folder-path`) pointing to `web/dist`. `crates/server/src/assets.rs` serves the
  assets with `Content-Type` and `Cache-Control` (immutable for `assets/*`), with an SPA fallback to
  `index.html`. Without the feature, the dev keeps serving `web/dist` from disk via `ServeDir`.
- **Auto-open browser:** on boot, `xdg-open` (Linux) / `open` (macOS), disableable with
  `DISCARD_NO_OPEN=1`. The URL uses `127.0.0.1` even when the bind is `0.0.0.0`.
- **Binary renamed** to `discard` (was `discard-server`).
- **New config:** `DISCARD_NO_OPEN`, `DISCARD_TOKEN` (fixed token in dev), `DISCARD_WEB_DIST`.
- **Icon:** `assets/icon.svg` + `assets/icon.png` (1024²).

## Linux (`scripts/build.sh`, `scripts/package-linux.sh`)

- `build.sh`: builds the UI and the release with `embed-ui`.
- `package-linux.sh`: produces `dist/discard-0.1.0-linux-<arch>.tar.gz` with the single binary, the icon and an
  `install.sh` that installs to `~/.local/bin/discard`, registers the icon/shortcut in XDG and creates the launcher
  `discard-app` with `DISCARD_DATA_DIR=~/.local/share/discard`.

**Verified on this machine:** the binary runs **without** `web/dist` alongside it (index/deep-link/sound = 200);
`install.sh` on a fake HOME installed everything and the launcher started with the data in XDG.

## macOS (`scripts/package-macos.sh` + `.github/workflows/release.yml`)

- **There is no cross-compile:** the `.app`/`.dmg` **must be built on macOS** (on a Mac or in CI
  `macos-latest`). The script assembles `Discard.app` (launcher with `DISCARD_DATA_DIR=~/Library/Application
  Support/Discard`, `Info.plist` with mic/camera permissions, `icon.icns`), signs **ad-hoc** and creates the
  `.dmg`.
- **Gatekeeper:** a non-notarized app requires removing the quarantine on first use:
  `xattr -dr com.apple.quarantine /Applications/Discard.app` (or right-click > Open).
- **CI:** `release.yml` generates `.dmg` (macos) and `.tar.gz` (linux). Alternative: build on a Mac reachable
  via Tailscale.

## Limitations

- Ad-hoc signing works for local use; clean distribution requires Developer ID + notarization.
- AppImage is not yet produced (the tarball + `install.sh` cover installation).
