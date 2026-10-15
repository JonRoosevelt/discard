# Stage 08 — Soundboard (F3 + tech lead)

## Assets (tech lead)

Seven effects synthesized with `ffmpeg` in `web/public/sounds/` (served at `/sounds/<file>`):

| id | emoji | file |
|---|---|---|
| pop | 🔊 | pop.mp3 |
| boom | 💥 | boom.mp3 |
| airhorn | 📢 | airhorn.mp3 |
| bell | 🔔 | bell.mp3 |
| drum | 🥁 | drum.mp3 |
| applause | 👏 | applause.mp3 |
| tada | 🎉 | tada.mp3 |

## Frontend (Agent F3)

`web/src/components/Soundboard.svelte`:

- Compact grid of buttons (emoji + name), volume with headroom (`new Audio(url)`).
- On click: plays locally and propagates `ws.send({op:'sound_play', channel_id, sound_id})` — where
  `channel_id` is the voice channel **I** am in; buttons disabled with a hint outside a call.
- Its own `ws.on` handler plays the received `sound_play` (only from my channel) and ignores the echo (`by == my id`).
- **Custom sound upload:** `file`/`name`/`emoji` → `POST /api/sounds` (`uploadSound` in `api.ts`) →
  `app.reloadSounds()`. Custom sounds do **not** expire in 24h (they are assets), outside the GC.

## Backend (N-voice)

`crates/server/src/sounds.rs` (`SoundStore`, `data/sounds.json`) + `routes/sounds.rs`: builtin +
uploads; upload persists blob and metadata; `sounds.json` survives a restart and is excluded from retention.

## Verification

- `npm run check` 0/0, `npm run build` ok.
- 7 builtin listed; upload → 8; persisted after restart (N-voice test).
