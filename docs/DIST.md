# Discard — Distribution and installation

Discard is a **local-first** app: each user runs their own node (`discard`) on the machine and the
UI talks to it at `http://127.0.0.1:<port>`. There is no central server to install.

## Build

At the repository root:

```sh
cd web && npm ci && npm run build && cd ..
cargo build --release --features embed-ui
```

This produces **`target/release/discard`**. With the `embed-ui` feature, the contents of `web/dist` are
**embedded in the binary**: the result is a **single file**, without the `web/dist` folder alongside.

On startup, the node prints:

```
Discard ready: http://127.0.0.1:<port>/?k=<token>
```

and **opens the browser** automatically. Set `DISCARD_NO_OPEN=1` to not open it.

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `DISCARD_ADDR` | `127.0.0.1:8787` | local node address/port |
| `DISCARD_DATA_DIR` | `data` (relative to cwd) | identity, workspaces, blobs and sounds |
| `DISCARD_OFFLINE` | off | `1` disables networking (iroh) |
| `DISCARD_NO_OPEN` | off | `1` does not open the browser |
| `DISCARD_TOKEN` | generated on boot | UI session token (`?k=` / `Authorization`) |

## Linux (omarchy / Arch)

The binary is single and self-contained. A simple installation:

```sh
install -Dm755 target/release/discard ~/.local/bin/discard
```

It is recommended to point the data to a stable directory (outside the cwd):

```sh
export DISCARD_DATA_DIR="$HOME/.local/share/discard"
mkdir -p "$DISCARD_DATA_DIR"
```

Desktop shortcut (`~/.local/share/applications/discard.desktop`):

```ini
[Desktop Entry]
Type=Application
Name=Discard
Exec=env DISCARD_DATA_DIR=%h/.local/share/discard %h/.local/bin/discard
Terminal=false
Categories=Network;InstantMessaging;
```

`scripts/package-linux.sh` (owner: core) produces the portable packages in `dist/` (`.tar.gz` and/or
AppImage). Unpack and run the `discard`, or execute the AppImage with execute permission.

## macOS

Packaging **must run on macOS** (there is no cross-compile). The script
`scripts/package-macos.sh`:

1. ensures `web/dist` and the release binary with `embed-ui`;
2. assembles `dist/macos/Discard.app`:
   - `Contents/MacOS/Discard` — launcher script that sets
     `DISCARD_DATA_DIR="$HOME/Library/Application Support/Discard"`, creates the directory and `exec`s
     the real binary (`Contents/MacOS/discard-bin`);
   - `Contents/MacOS/discard-bin` — the release binary;
   - `Contents/Info.plist` — `app.discard.desktop`, version `0.1.0`, with the microphone and camera
     usage descriptions;
   - `Contents/Resources/icon.icns` — generated from `assets/icon.png` with `sips` + `iconutil`.
3. applies an **ad-hoc** signature (`codesign --sign -`, best-effort);
4. generates `dist/Discard-0.1.0-macos.dmg`.

### Install

- **`.app`**: drag `Discard.app` to `/Applications` (or run it directly from `dist/macos`).
- **`.dmg`**: double-click `dist/Discard-0.1.0-macos.dmg` and drag the app to `Applications`.

The data goes in `~/Library/Application Support/Discard` (set by the launcher), and **not** in the
bundle directory.

### Gatekeeper (important)

The app is only **ad-hoc** signed, it is not notarized by Apple. On first use macOS may
block it with "cannot verify the developer". To open it:

```sh
xattr -dr com.apple.quarantine /Applications/Discard.app
```

or right-click the app > **Open** > **Open**. You can also go to
**System Settings > Privacy & Security** and use **Open Anyway**.

### Quarantine and microphone

- Downloaded files (DMG) receive the `com.apple.quarantine` attribute; the command above removes it.
- On the first voice/video call, macOS asks for access to the **microphone** and **camera**. The descriptions
  are in `Info.plist`. If you deny it, re-enable it in
  **System Settings > Privacy & Security > Microphone / Camera**.

### Distributing for real

Ad-hoc signing only works for local use. To distribute without security warnings you need:

1. a **Developer ID Application** certificate;
2. `codesign --sign "Developer ID Application: ..." --options runtime Discard.app` (hardened runtime);
3. `xcrun notarytool submit` and `xcrun stapler staple Discard.app`.

## CI (GitHub Actions)

`.github/workflows/release.yml` runs on `workflow_dispatch` and on `v*` tags:

- **job `macos`** (`macos-latest`): UI build, Rust stable toolchain, `scripts/package-macos.sh`
  and upload of the `Discard-*.dmg` artifact;
- **job `linux`** (`ubuntu-latest`): UI build, Rust stable toolchain, `scripts/package-linux.sh`
  and upload of `dist/*` (`.tar.gz`/AppImage).

The artifacts are available in the workflow run (the **Actions** tab, **Artifacts** section).
Download, install according to the platform above and, on macOS, remove the quarantine before opening.
