# Stage 03 — Wave 1: frontend shell + chat (Agent F1)

## Scope

A complete Svelte 5 application for **chat**, with a minimalist, dense, dark-first UX (reference: Discord),
consuming the backend's REST/WS contract.

## Files

`web/src/`:
- `lib/protocol.ts` — TS types mirroring `shared` (`SignalData`/`ClientEvent`/`ServerEvent` unions).
- `lib/api.ts` — `token` (localStorage), `api<T>`, `Auth`, `uploadFile`, REST helpers.
- `lib/ws.ts` — WebSocket client with exponential reconnection (0.5s→30s) and offline queue.
- `lib/state.svelte.ts` — global store with runes (`app`, `net`, `typing`).
- `lib/format.ts` — `formatTime`, `formatDay`, `formatBytes`, `initials`, `avatarColor`, `isImage`, `parseDate`.
- `components/`: `Login`, `Shell`, `WorkspaceRail`, `ChannelSidebar`, `ChatView`, `MessageList`,
  `Composer`, `MembersPanel`, `VoicePanel` (stub), `Soundboard` (stub).
- `app.css` (dark-first tokens), `App.svelte`, `index.html`.
- Removed: `lib/Counter.svelte`, `assets/`.

## Layout

Workspace rail (72px) · channel sidebar (240px) · chat (flex) · voice panel (320px).
Voice channels show the connected members. Composer with auto-grow, Enter sends / Shift+Enter breaks,
attachments via button and drag&drop, "typing" throttle.

## Deviations / notes

1. **`app.members`** is derived (self + authors seen in messages + members in voice), since there is no
   members endpoint. See the open item in Stage 01.
2. Extra exports (`net`, `typing`, `onReconnect`, `onStatus`) — additive, they do not break the contract shape.
3. `selectChannel` only does `subscribe` on text channels.
4. `appendMessage` deduplicates by `id` (REST + WS can deliver the same message).

## Verification

- `npm run build` → built in ~631ms, 0 warnings.
- `npm run check` (svelte-check + tsc) → 0 errors, 0 warnings.
