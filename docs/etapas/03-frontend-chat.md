# Etapa 03 — Wave 1: frontend shell + chat (Agent F1)

## Escopo

Aplicação Svelte 5 completa para **chat**, com UX minimalista, densa e dark-first (referência: Discord),
consumindo o contrato REST/WS do backend.

## Arquivos

`web/src/`:
- `lib/protocol.ts` — tipos TS espelhando `shared` (unions `SignalData`/`ClientEvent`/`ServerEvent`).
- `lib/api.ts` — `token` (localStorage), `api<T>`, `Auth`, `uploadFile`, helpers REST.
- `lib/ws.ts` — cliente WebSocket com reconexão exponencial (0,5s→30s) e fila offline.
- `lib/state.svelte.ts` — store global com runes (`app`, `net`, `typing`).
- `lib/format.ts` — `formatTime`, `formatDay`, `formatBytes`, `initials`, `avatarColor`, `isImage`, `parseDate`.
- `components/`: `Login`, `Shell`, `WorkspaceRail`, `ChannelSidebar`, `ChatView`, `MessageList`,
  `Composer`, `MembersPanel`, `VoicePanel` (stub), `Soundboard` (stub).
- `app.css` (tokens dark-first), `App.svelte`, `index.html`.
- Removidos: `lib/Counter.svelte`, `assets/`.

## Layout

Rail de workspaces (72px) · sidebar de canais (240px) · chat (flex) · painel de voz (320px).
Canais de voz mostram os membros conectados. Composer com auto-grow, Enter envia / Shift+Enter quebra,
anexos por botão e drag&drop, throttle de "typing".

## Desvios / observações

1. **`app.members`** é derivado (self + autores vistos em mensagens + membros em voz), pois não há
   endpoint de membros. Ver pendência na Etapa 01.
2. Exports extras (`net`, `typing`, `onReconnect`, `onStatus`) — aditivos, não quebram o shape do contrato.
3. `selectChannel` só faz `subscribe` em canais de texto.
4. `appendMessage` deduplica por `id` (REST + WS podem entregar a mesma mensagem).

## Verificação

- `npm run build` → built em ~631ms, 0 warnings.
- `npm run check` (svelte-check + tsc) → 0 errors, 0 warnings.
