# Stage 05 — Local iroh node (Agent N1)

## Scope

Rewrite `crates/server` as a **local node**: iroh (endpoint/gossip/blobs) + Axum on `127.0.0.1`
serving the SPA and the local API + WebSocket. Source of truth: `crates/shared` v3.

## Files

`crates/server/src/`: `main.rs`, `config.rs`, `error.rs`, `identity.rs`, `ticket.rs`, `iroh_net.rs`,
`retention.rs`, `store.rs`, `hub.rs`, `state.rs`, `ws.rs`, `media.rs`, `auth.rs`,
`routes/{mod,identity,workspaces,channels,messages,uploads,files,sounds}.rs`.
Removed: `db.rs`, `models.rs`, `auth.rs` (old), `upload.rs`, `migrations/` and the deps
`sqlx`/`argon2`/`jsonwebtoken`. New: `iroh 1.3`, `iroh-gossip 0.101`, `iroh-blobs 0.103`,
`iroh-tickets 1.0`, `base64`, `hex`, `blake3`.

## Highlights

- **Identity = endpoint key.** ed25519 secret in `data/identity.key` (0600); `EndpointId`
  is the public identity. `display_name` in `data/profile.json`. Exportable/importable backup.
- **Local session:** random `session_token` on boot; the UI is opened at `?k=<token>`.
- **Workspace ticket** (`ticket.rs`): base64(JSON) `{id,name,owner_id,owner_name,secret,channels,
  endpoint_ticket}`. Channel topic = `BLAKE3(secret||channel_id)[0..32]`.
- **24h retention** (`retention.rs`): per-channel buffer, dedupe by id, blob refcount by tag;
  GC every 5 min removes expired messages and blobs.
- **Message:** sent → broadcast via gossip + buffer + push to the UI; received → dedupe, buffer,
  download of attached blobs, push to the UI.
- **Discovery:** `presets::N0` (internet) or `presets::Minimal` (`DISCARD_OFFLINE=1`), always with
  `MemoryLookup` fed by the addresses from the tickets.

## Verification (two nodes, `DISCARD_OFFLINE=1`)

- A creates a workspace → **B joins by ticket and starts listing the workspace** (discovery by ticket).
- A and B subscribe to `#geral`; A sends → **B receives via gossip** and B's `GET .../messages` returns the message.
- A uploads an attachment → A sends → **B downloads the blob** and B's `GET /files/{hash}` is identical.
- Security: without/invalid token → 401; SPA with deep link → 200; `/api/sounds` → list.

## Deviations

- `Id = String` throughout the system (`"{ts_ms}-{author_short}"` for messages).
- `/files/{hash}` does not require a token (browsers do not send `Authorization` on `<img>`).
- Identity import rebuilds the iroh endpoint at runtime (the UI needs to re-subscribe).
- Channels created after the ticket do not reach those who already joined (the ticket is a snapshot) — a known v3 item.
