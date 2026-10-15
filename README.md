# Discard

A **minimalist, P2P and ephemeral** Discord clone.

- **No central server.** Each user runs a **local node** in Rust; nodes speak **iroh** (QUIC).
- **Ephemeral.** Messages and attachments **expire in 24h** — no durable history.
- **Workspaces** with **text** and **voice** channels; invite via **ticket** (capability).
- **Voice/video** over a **WebRTC mesh** (E2E via DTLS-SRTP); signaling over the P2P overlay.
- **Soundboard** with effects propagated via gossip.
- **Strong privacy**: E2E encryption by default, zero third parties, keys only on the device.
  See [`docs/PRIVACY.md`](docs/PRIVACY.md).

<img width="1342" height="718" alt="image" src="https://github.com/user-attachments/assets/0c91c6eb-7a24-4efa-a436-fd168499562e" />
<img width="1342" height="718" alt="image" src="https://github.com/user-attachments/assets/8f6795f2-8315-40f7-af89-44c7eac85a9e" />


## Architecture

```
[ Browser A ] --localhost--> [ Node A ] <== iroh (gossip + blobs) ==> [ Node B ] --localhost--> [ Browser B ]
```

- **Local node** (`crates/server`, bin `discard-server`): identity (ed25519), iroh
  endpoint/gossip/blobs, local 24h buffer, and the local API (Axum) that serves the UI.
- **UI** (`web/`, Svelte 5 + Vite): talks only to the local node; media over WebRTC.
- **Optional infra** (does not see content): iroh relay (encrypted fallback) and address lookup — both
  self-hostable.

## Run

```bash
# local node (opens the UI with the token at http://127.0.0.1:8787/?k=...)
cargo run -p server

# UI in dev mode (optional)
cd web && npm install && npm run dev
```

Variables: `DISCARD_ADDR` (`127.0.0.1:8787`), `DISCARD_DATA_DIR` (`data`), `DISCARD_OFFLINE`
(`1` = no relay/DNS, for local tests).

## Stack

Rust (Axum + iroh + iroh-gossip + iroh-blobs) · Svelte 5 (runes) + Vite + TypeScript · WebRTC · WebCrypto.

## Documentation

All documentation and the decision history per stage are in [`docs/`](docs/README.md).
