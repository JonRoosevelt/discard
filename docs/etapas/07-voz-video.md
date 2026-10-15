# Stage 07 — Voice/video (N-voice + F2)

## Backend (Agent N-voice)

`crates/server/src/voice.rs` + `sounds.rs`; `ws.rs` gained the real handlers for
`VoiceJoin/Update/Leave`, `Signal`, `SoundPlay`; `hub.rs` gained `broadcast_all` (voice events go to
all local UIs).

- **Presence via gossip + heartbeat/TTL:** the node re-broadcasts `presence_join` every 10s; remote
  members expire after 30s without a signal (generating `VoiceLeft`). On join it also does `presence_sync` and the
  peers respond `presence_state` (immediate snapshot).
- **Signaling:** `signal {to, data}` travels as an envelope on the voice channel gossip; the node whose
  `public_id == to` delivers `ServerEvent::Signal {from, data}` to its UI. Pure relay (does not interpret SDP).
- **Soundboard:** `sound_play` → envelope on gossip; everyone in the channel receives `ServerEvent::SoundPlay`.

**Test (2 nodes):** A and B join the voice channel and each sees the other; `signal` from A reaches B;
`sound_play` reaches the other; `GET /api/sounds` → 7 builtin. **PASS.**

## Frontend (Agent F2)

`web/src/lib/rtc.ts` (`VoiceSession`), `components/VideoTile.svelte`, `components/VoicePanel.svelte`.

- WebRTC mesh: **the newcomer initiates** offers to the present members; the others only respond. Glare
  covered by `polite/impolite` (`selfId > peerId`). ICE buffered before the remote description.
- `ICE_SERVERS` **empty by default** (privacy decision: no third-party STUN; direct ICE).
- No unnecessary renegotiation: `sendrecv` transceivers created at the start; camera/screen via
  `sender.replaceTrack`.
- `VoicePanel` derives from `app.voice` + `app.identity.public_id` whether **I** am in a channel; tile grid,
  control bar (mic, deafen, camera, screen, leave) and renders `<Soundboard />`.

## Verification

- `npm run check` 0/0 and `npm run build` ok.
- In the real preview (local node + `?k=`), joining the voice channel opened the panel with a local tile, controls and
  soundboard; the microphone failure (headless preview) was handled with a discreet message.
