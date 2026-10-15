# Stage 06 — P2P frontend + files (Agent N2)

## Scope

Adapt the prototype's Svelte 5 UI for: local session (`?k=`), **identity** in place of login,
**invite tickets**, `Id = string`, and the notice of ephemeral messages (24h).

## Files

`web/src/lib/`: `protocol.ts` (v3 types), `api.ts` (token from `?k=` → `sessionStorage`, `Identity`,
workspaces/ticket/join, `uploadFile`, `listSounds`, `uploadSound`), `identity.ts` (helpers),
`ws.ts` (reconnection, session token), `state.svelte.ts` (`app` with `identity`, `booted/entered`,
`reloadSounds`), `format.ts`.
`web/src/components/`: `Login.svelte` (→ identity), **`InviteDialog.svelte` (new)**,
`WorkspaceRail.svelte` (create/join menu), `ChannelSidebar.svelte` (invite button), `Shell.svelte`,
`ChatView.svelte`, `MessageList.svelte`, `Composer.svelte`, `MembersPanel.svelte`; `app.css` (modal).

## Uploads / files

- `POST /api/upload` (multipart) → `Attachment { id: hash, url: "/files/{hash}", ... }`; the blob stays in the
  iroh store with a 24h TTL and is replicated on demand to the peers that receive the message.
- Composer with attachment via button and drag&drop; images with preview, other files as a card.

## Verification

- `npm run check` → 0 errors, 0 warnings.
- `npm run build` → ok (~93 kB JS, ~25 kB CSS gzip-friendly).

## Deviations

- Reactive identity state in `app.identity` (pure `.ts` has no runes; helpers mutate).
- `downloadBackup()` in addition to `backupUrl()` (the `/api` route requires Bearer, a direct link does not send it).
- The entry flow requires clicking "Entrar" on the identity screen.
