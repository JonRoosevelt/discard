# Etapa 07 — Voz/vídeo (N-voice + F2)

## Backend (Agent N-voice)

`crates/server/src/voice.rs` + `sounds.rs`; `ws.rs` ganhou os handlers reais de
`VoiceJoin/Update/Leave`, `Signal`, `SoundPlay`; `hub.rs` ganhou `broadcast_all` (eventos de voz vão a
todas as UIs locais).

- **Presença por gossip + heartbeat/TTL:** o nó re-broadcasta `presence_join` a cada 10s; membros
  remotos expiram após 30s sem sinal (gerando `VoiceLeft`). No join também faz `presence_sync` e os
  peers respondem `presence_state` (snapshot imediato).
- **Sinalização:** `signal {to, data}` viaja como envelope no gossip do canal de voz; o nó cujo
  `public_id == to` entrega `ServerEvent::Signal {from, data}` à sua UI. Relay puro (não interpreta SDP).
- **Soundboard:** `sound_play` → envelope no gossip; todos no canal recebem `ServerEvent::SoundPlay`.

**Teste (2 nós):** A e B entram no canal de voz e cada um enxerga o outro; `signal` de A chega em B;
`sound_play` chega no outro; `GET /api/sounds` → 7 builtin. **PASS.**

## Frontend (Agent F2)

`web/src/lib/rtc.ts` (`VoiceSession`), `components/VideoTile.svelte`, `components/VoicePanel.svelte`.

- Mesh WebRTC: **o novato inicia** offers para os membros presentes; os demais só respondem. Glare
  coberto por `polite/impolite` (`selfId > peerId`). ICE bufferizado antes do remote description.
- `ICE_SERVERS` **vazio por padrão** (decisão de privacidade: sem STUN de terceiros; ICE direto).
- Sem renegociação desnecessária: transceivers `sendrecv` criados no início; câmera/tela por
  `sender.replaceTrack`.
- `VoicePanel` deriva de `app.voice` + `app.identity.public_id` se **eu** estou em canal; grid de tiles,
  barra de controles (mic, deafen, câmera, tela, sair) e renderiza `<Soundboard />`.

## Verificação

- `npm run check` 0/0 e `npm run build` ok.
- No preview real (nó local + `?k=`), entrar no canal de voz abriu o painel com tile local, controles e
  soundboard; a falha de microfone (preview headless) foi tratada com mensagem discreta.
