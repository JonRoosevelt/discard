# Stage 09 — Integration and final verification (tech lead)

## Builds

- `cargo build` — clean (workspace: `shared`, `server`, `iroh-spike`).
- `cd web && npm run check` — 0 errors, 0 warnings. `npm run build` — ok.

## E2E smoke test (two nodes)

Proven on the local node (`DISCARD_OFFLINE=1`, distinct ports and dirs):

1. two nodes create an identity;
2. A creates a workspace → B does a **join by ticket** and starts listing the workspace;
3. a message sent on A appears on B (gossip) and in the 24h buffer;
4. attachment replicated: B downloads the blob and `/files/{hash}` matches;
5. voice: both join the channel, cross presence, `signal` relayed, `sound_play` propagated;
6. `/api/sounds` (7 builtin) and sound upload.

## Visual validation (real UI served by the node)

In the browser via `http://127.0.0.1:8787/?k=<token>`:

- **local identity** screen (create/import/export backup), with the text "Mensagens desaparecem em 24h";
- shell with rail, sidebar (Text Channels / Voice Channels), chat with a **24H** badge in the header;
- create workspace → channels `#geral` and `🔊 Geral`;
- create/join menu with ticket, invite modal;
- join the voice channel → panel with tile, controls and **soundboard with the 7 effects**; mic failure
  handled discreetly.

## Post-validation fixes

- **Responsive layout (Shell):** the voice panel reserved 320px **always** (even outside a call),
  squeezing the chat in narrow windows (72+240+320 = 632px). Now the panel only exists when the user
  is in a voice channel (`class:incall`) and there are breakpoints: `≤1100px` panel 280px; `≤880px` panel
  hidden; `≤520px` sidebar hidden. The chat uses `minmax(0, 1fr)` and never collapses.

## How to run

```bash
# local node (opens the UI with the token)
cargo run -p server
# online (real discovery): without DISCARD_OFFLINE
# offline/local: DISCARD_OFFLINE=1
```

Variables: `DISCARD_ADDR` (default `127.0.0.1:8787`), `DISCARD_DATA_DIR` (default `data`),
`DISCARD_OFFLINE`.

UI development:

```bash
cd web && npm install && npm run dev   # 5173, proxy to the node on 8787
```

## Known limitations / next steps

- Channels created **after** the invite do not reach those who already joined (the ticket is a snapshot). Solution:
  gossip of `workspace_update` signed by the owner.
- No persistence of messages by design (TTL 24h); joining a channel does not bring backfill.
- `presets::N0` uses n0's public DNS/relay; for maximum privacy, **self-host** a relay and/or
  lookup via Mainline DHT (planned).
- Content encryption in the client (AES-GCM/WC) over the gossip document is optional and not yet enabled.
- Packaging (binary + embedded `web/dist`) and icon/user service were not done.

## Decision on "server"

There is no application server. Only relay (fallback, encrypted, self-hostable) and address lookup. The binary
`discard-server` is the **local node** that each user runs — it is not a central server.
