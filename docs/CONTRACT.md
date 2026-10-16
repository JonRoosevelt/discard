# Discard — Interface Contract v3 (P2P, ephemeral, local-first)

> **v3.** No central server **and** no message storage: **messages expire in 24h**.
> No CRDT, no history database. Each user runs a **local node** in Rust that speaks **iroh**.
> See `PRIVACY.md` and `etapas/00-visao-geral.md`.

## Required infrastructure (what remains)

No application server. Only **optional and replaceable** infrastructure, which does not see content:

| Piece | Purpose | Default | Self-host |
|---|---|---|---|
| iroh relay | NAT hole punching / fallback; forwards **already encrypted** packets | n0 public relays | yes (`presets::N0` → own relay) |
| Address lookup | Discover a peer's address by key | iroh DNS (`dns.iroh.link`) | yes (Mainline DHT, BEP 44) |

## Topology

```
[ Browser A ] --HTTP/WS localhost--> [ Node A ] <══ iroh (QUIC/TLS1.3) ══> [ Node B ] <--localhost--> [ Browser B ]
                                     gossip (real time) + blobs (files)
```

## Retention (hard rule)

- **Messages:** ephemeral. TTL = **24h**. They live in a **local ring buffer** of the node; nothing is sent
  for storage. They are never replicated in order to persist. When they expire, they vanish.
- **Attachments/sounds:** blobs with a 24h TTL and **GC**; removed when the message that references them expires.
- **Workspace config** (name, channels, members): this one does persist **locally** on each user,
  because it is what gives the workspace meaning. It is not conversation content.
- Optional backfill on join: a peer may send the latest messages **still alive** (<= 24h).

## Concepts

| Concept | Implementation |
|---|---|
| Identity | local ed25519 keypair = iroh endpoint key (no email/password) |
| Workspace | secret topic (gossip) + config signed by the owner. **No document/CRDT** |
| Invite / access | **self-contained ticket** (base64): `{id, name, secret, channels, owner}` — it is a capability |
| Channel | uuid id; text topic = `BLAKE3(secret || channel_id)` |
| Workspace (real time) | topic = `BLAKE3("discard-realtime-v1" || secret)`; carries voice presence, signaling and soundboard |
| Message | ephemeral (24h), transported via gossip **on the channel topic**; id `"{ts_ms}-{author_short}"` |
| Attachment | blob (BLAKE3), TTL 24h; `url = /files/<hash>` |
| Real time (text) | gossip topic per channel |
| Voice presence / signaling / soundboard | gossip on the **workspace topic** (visible to all members, any channel) |

## Identity and local session

- Private key in `data/identity.key` (`0600`), optionally with a passphrase. It never goes to the network.
- Local session: on boot the node generates a `session_token` and opens `http://127.0.0.1:<port>/?k=<token>`.
  `/api` and `/ws` require `Authorization: Bearer <session_token>` (protection against other sites).

## Local REST (`/api`)

| Method | Route | Body | Return |
|---|---|---|---|
| GET | `/api/health` | – | `{"ok":true}` |
| GET | `/api/identity` | – | `Identity` or 404 |
| POST | `/api/identity` | `{display_name}` | `Identity` |
| GET | `/api/identity/backup` | – | backup download |
| POST | `/api/identity/import` | `{backup, passphrase?}` | `Identity` |
| GET | `/api/workspaces` | – | `Workspace[]` |
| POST | `/api/workspaces` | `{name}` | `Workspace` (**includes the `ticket`**) |
| POST | `/api/workspaces/join` | `{ticket}` | `Workspace` |
| GET | `/api/workspaces/{id}` | – | `Workspace` |
| GET | `/api/workspaces/{id}/ticket` | – | `{ticket}` |
| POST | `/api/workspaces/{id}/channels` | `{name, kind}` | `Channel` |
| GET | `/api/channels/{id}/messages` | – | `Message[]` (only what is still alive) |
| POST | `/api/channels/{id}/messages` | `{content, attachment_ids}` | `Message` |
| POST | `/api/upload` | multipart `file` | `Attachment` |
| GET | `/files/{hash}` | – | blob bytes |
| GET | `/api/sounds` | – | `Sound[]` |
| POST | `/api/sounds` | multipart `file`,`name`,`emoji` | `Sound` |

Errors: `{"error":"..."}` with an appropriate status.

## Local WebSocket `/ws?token=<session_token>`

`ClientEvent` / `ServerEvent` in `crates/shared`. On connect: `Ready { identity, workspaces, voice, sounds }`.

- The node subscribes to the **workspace topic** of each known workspace (on boot/tick and when the UI connects),
  using the owner's `endpoint_ticket` as bootstrap and the peers discovered via `NeighborUp`.
- `subscribe` → node joins the channel's (text) gossip topic.
- `send_message` → node does `gossip.broadcast` (ephemeral) on the channel topic and stores it in the local 24h buffer.
- `voice_join/update/leave`, `signal`, `sound_play` → gossip on the **workspace topic**.
- `Ready.voice` carries presences from **all channels** of the user; the sidebar filters by workspace.
- `signal` is filtered by recipient (`to`); `sound_play` is only forwarded to those in the voice channel.
- Peer news (`gossip Event::Received`) become `ServerEvent` for the local UIs.
- `ping`/`pong`.

## Types (`crates/shared`)

**`Id = String`**. `Identity { public_id, display_name, has_backup }`, `UserPublic { id, display_name,
avatar_url }`, `Workspace { id, name, icon_url, owner_id, ticket, secret, channels }`, `Channel`,
`Attachment`, `Message`, `Sound`, `VoiceMember`, `VoicePresence`. Requests and events as above.

`Workspace.secret` is the workspace's hex secret (the same as in the `ticket`). The local node (localhost+token)
exposes it to its own UI to derive the content key. **Message content:** the UI encrypts the body
before sending and the envelope goes in the `Message.content` field as `"E1:" + base64(nonce(12) ||
ciphertext)` (AES-256-GCM; key = HKDF-SHA256(secret, info="discard-content-v1")). Without the `E1:`
prefix, the content is treated as plaintext (fallback/legacy). The node/database stores only the envelope.

## WebRTC Mesh (unchanged)

The newcomer initiates offers; the others respond. Signaling: UI → local node → gossip → remote node → UI.

ICE fallback: each node runs a TURN server on `127.0.0.1` (TCP, per-session credentials sent in WS `ready` as
`ice_servers`). Relayed addresses are virtual (`100.64.0.0/10`, derived from the EndpointId) and the media is
carried node-to-node as QUIC datagrams on ALPN `discard/media/1`, so it follows iroh's holepunch/relay path.
ICE still prefers host/srflx pairs. `DISCARD_RELAY_ONLY=1` drops iroh's IP transports (testing the relay path).

## File structure (owner = whoever edits)

Backend `crates/server/src/` — **Agent N1**: `identity.rs`, `iroh_net.rs` (endpoint/gossip/blobs +
lookup), `retention.rs` (24h buffer + GC), `hub.rs` (fanout to local UIs), `main.rs`, `config.rs`,
`error.rs`, `state.rs`, `ticket.rs`, `routes/*`.
**Agent N-voice**: `voice.rs` (presence + signaling via gossip), `routes/sounds.rs`.

Frontend `web/src/` — **Agent N2**: `lib/api.ts`, `lib/ws.ts`, `lib/protocol.ts`, `lib/state.svelte.ts`,
`lib/identity.ts`, `components/Login.svelte` (→ identity), `components/InviteDialog.svelte`,
`App.svelte` (string ids). **Agent F2**: `lib/rtc.ts`, `components/VoicePanel.svelte`,
`components/VideoTile.svelte`. **Agent F3**: `components/Soundboard.svelte`.

## Style

Svelte 5 runes; minimalist dark-first; no comments; no third parties (fonts/CDN/telemetry);
`cargo build` + `npm run build` + `npm run check` green; **never commit**.
