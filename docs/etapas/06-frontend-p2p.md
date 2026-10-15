# Etapa 06 — Frontend P2P + arquivos (Agent N2)

## Escopo

Adaptar a UI Svelte 5 do protótipo para: sessão local (`?k=`), **identidade** no lugar de login,
**tickets de convite**, `Id = string`, e o aviso de mensagens efêmeras (24h).

## Arquivos

`web/src/lib/`: `protocol.ts` (tipos v3), `api.ts` (token de `?k=` → `sessionStorage`, `Identity`,
workspaces/ticket/join, `uploadFile`, `listSounds`, `uploadSound`), `identity.ts` (helpers),
`ws.ts` (reconexão, token da sessão), `state.svelte.ts` (`app` com `identity`, `booted/entered`,
`reloadSounds`), `format.ts`.
`web/src/components/`: `Login.svelte` (→ identidade), **`InviteDialog.svelte` (novo)**,
`WorkspaceRail.svelte` (menu criar/entrar), `ChannelSidebar.svelte` (botão convite), `Shell.svelte`,
`ChatView.svelte`, `MessageList.svelte`, `Composer.svelte`, `MembersPanel.svelte`; `app.css` (modal).

## Uploads / arquivos

- `POST /api/upload` (multipart) → `Attachment { id: hash, url: "/files/{hash}", ... }`; o blob fica no
  store do iroh com TTL 24h e é replicado sob demanda aos peers que recebem a mensagem.
- Composer com anexo por botão e drag&drop; imagens com preview, outros arquivos como card.

## Verificação

- `npm run check` → 0 erros, 0 warnings.
- `npm run build` → ok (~93 kB JS, ~25 kB CSS gzip-friendly).

## Desvios

- Estado reativo da identidade em `app.identity` (`.ts` puro não tem runes; helpers mutam).
- `downloadBackup()` além de `backupUrl()` (rota `/api` exige Bearer, link direto não manda).
- Fluxo de entrada exige clicar "Entrar" na tela de identidade.
