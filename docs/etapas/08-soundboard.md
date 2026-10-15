# Etapa 08 — Soundboard (F3 + tech lead)

## Assets (tech lead)

Sete efeitos sintetizados com `ffmpeg` em `web/public/sounds/` (servidos em `/sounds/<arquivo>`):

| id | emoji | arquivo |
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

- Grid compacto de botões (emoji + nome), volume com headroom (`new Audio(url)`).
- Ao clicar: toca localmente e propaga `ws.send({op:'sound_play', channel_id, sound_id})` — onde
  `channel_id` é o canal de voz em que **eu** estou; botões desabilitados com dica fora de chamada.
- Handler próprio de `ws.on` toca `sound_play` recebido (só do meu canal) e ignora o eco (`by == meu id`).
- **Upload de som custom:** `file`/`name`/`emoji` → `POST /api/sounds` (`uploadSound` em `api.ts`) →
  `app.reloadSounds()`. Sons custom **não** expiram em 24h (são assets), fora do GC.

## Backend (N-voice)

`crates/server/src/sounds.rs` (`SoundStore`, `data/sounds.json`) + `routes/sounds.rs`: builtin +
uploads; upload persiste blob e metadata; `sounds.json` sobrevive a restart e é excluído do retention.

## Verificação

- `npm run check` 0/0, `npm run build` ok.
- 7 builtin listados; upload → 8; persistiu após restart (teste do N-voice).
