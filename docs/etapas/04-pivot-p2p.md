# Stage 04 — Pivot to P2P (iroh)

## What motivated it

Two user requirements changed the architecture:

1. **Strong privacy** and, explicitly, **no central server storing messages** —
   cited reference: `pears.com` / Keet ("the chat app that knows nothing").
2. **Ephemeral messages**: "we have no interest in storing messages… it expires in a day".

## Decisions (ADR)

- **P2P with iroh** (Rust stack: QUIC/TLS1.3, NAT traversal, relays as encrypted fallback).
- **No application server.** What runs is a **local node** per user, which serves the UI on localhost.
- **No history.** Messages TTL = **24h** in a local buffer; no `iroh-docs`/CRDT, no database.
- **Capability-based access:** the invite is a **self-contained ticket** with the workspace secret.
- **Media** over a WebRTC mesh (DTLS-SRTP E2E), signaled by the P2P overlay.
- Discarded: SQLite/SQLx, argon2/JWT, `iroh-docs`.

## Spike (Agent S) — API de-risking

The throwaway crate `crates/iroh-spike` proved, running **offline on localhost**:

- gossip A→B (`broadcast`/`Event::Received`),
- blob A→B (`add_bytes` → `downloader.download` → `get_bytes`, integrity ok),
- `EndpointTicket` and `EndpointId`.

**Resolved versions:** `iroh 1.3.0`, `iroh-gossip 0.101.0`, `iroh-blobs 0.103.1`,
`iroh-tickets 1.0.0`.

**Critical gotchas (API changed vs. old tutorials):**

- `endpoint.add_node_addr` no longer exists; the address book is `MemoryLookup` +
  `.address_lookup(lookup).` and `lookup.add_endpoint_info(addr)`.
- `Gossip::builder().spawn(ep)` alone does not accept connections — it must be registered in the `Router`
  (`.accept(iroh_gossip::ALPN, gossip.clone())`).
- `GossipReceiver` requires `futures_util::StreamExt` for `.next()`.
- Presets: `N0` (public DNS+relay, needs internet), `Minimal` (offline), `Empty` (fails).
- Always wrap `joined()`/`next()` in a timeout.

## Consequence

The prototype from Stages 02/03 (central server) was **replaced**; it remained only as a reference for
the UI and the Axum structure. The contract was rewritten as **v3** (`docs/CONTRACT.md`).
