# Stage 00 — Overview (P2P, ephemeral, local-first)

> Revised twice: first for P2P (no content server), then for **ephemeral
> messages (24h)**. See `PRIVACY.md` and `CONTRACT.md` (v3).

## Goal

A **lean, P2P and ephemeral** Discord clone:

- **Workspaces** with **text** and **voice** channels.
- **Chat** in real time over gossip; **messages expire in 24h** (no durable history).
- **File upload** via content-addressed blobs (TTL 24h).
- **Real-time audio/video** over a WebRTC mesh, signaled by the P2P overlay.
- **Sound effects** (soundboard) propagated over gossip.
- **Strong privacy**: no content server, E2E encryption, zero third parties.

## Architecture

Each user runs a **local node** in Rust; the **browser UI** is served by that node and talks only to
it (localhost). The nodes talk to each other over **iroh** (QUIC + TLS 1.3, NAT traversal, relays only as fallback).

```
[ Browser A ] ──localhost──> [ Node A ] <══ iroh ══> [ Node B ] ──localhost──> [ Browser B ]
```

| Need | iroh protocol | Persistence |
|---|---|---|
| Real time (chat, presence, signaling, soundboard) | **iroh-gossip** | none (ephemeral, 24h in local buffer) |
| Files / audios | **iroh-blobs** (BLAKE3) | blob with 24h TTL + GC |
| Connection, identity, encryption, NAT | **iroh endpoint** | local key |

> Discarded relative to v1/v2: **SQLite/SQLx**, **password authentication** and **iroh-docs/CRDT**.
> There is no history to replicate — just real time and a 24h buffer.

## Required infrastructure

There is no application server. Only the **relay** (NAT fallback, forwards encrypted traffic) and the
**address lookup** (iroh DNS or Mainline DHT) — both optional and self-hostable. Nothing stores content.

## Decisions (summarized ADR)

1. **Ephemeral by design.** Messages TTL 24h, no persistence or history synchronization.
2. **iroh as the P2P layer.** Dialing by key, E2E QUIC, relays as encrypted fallback.
3. **Capability-based access.** Access to a workspace is a **self-contained ticket** (contains the secret and the
   config). No ticket, no access.
4. **WebRTC mesh for media** (up to ~6 people); direct ICE by product decision.
5. **Local node in Rust** (not a central server): iroh agent + serves the UI + keeps the 24h buffer.
6. **Thin frontend**, owner of the end-to-end encryption (the private key never goes to the node).

## Repository structure

```
Discard/
├── Cargo.toml                 # Rust workspace
├── crates/
│   ├── shared/                # UI <-> node protocol types (Id = String)
│   ├── server/                # bin `discard-server` = LOCAL NODE (Axum + iroh)
│   └── iroh-spike/            # throwaway spike to validate the iroh API
├── web/                       # UI Svelte 5 + Vite
├── docs/                      # documentation
├── data/ , uploads/           # local node state (runtime)
```

## Roadmap (waves)

| Wave | Scope | Agents |
|---|---|---|
| 1 | UI shell + chat + local API (prototype) | A, F1 — **done, will be adapted** |
| 2 | iroh spike (gossip + blobs) | S |
| 3 | Local iroh node (identity, gossip, blobs, TTL 24h) | N1 |
| 4 | Frontend: identity, invite tickets, string ids | N2 |
| 5 | Voice/video and soundboard over gossip | N-voice, F2, F3 |
| 6 | Integration, packaging, E2E | tech lead |

## Run (dev, target)

```bash
cargo run -p server          # local node; opens http://127.0.0.1:8787/?k=<token>
cd web && npm install && npm run dev   # UI on 5173 during development
```
