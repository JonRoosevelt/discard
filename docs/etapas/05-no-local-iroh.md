# Etapa 05 — Nó local iroh (Agent N1)

## Escopo

Reescrever `crates/server` como **nó local**: iroh (endpoint/gossip/blobs) + Axum em `127.0.0.1`
servindo a SPA e a API local + WebSocket. Fonte da verdade: `crates/shared` v3.

## Arquivos

`crates/server/src/`: `main.rs`, `config.rs`, `error.rs`, `identity.rs`, `ticket.rs`, `iroh_net.rs`,
`retention.rs`, `store.rs`, `hub.rs`, `state.rs`, `ws.rs`, `media.rs`, `auth.rs`,
`routes/{mod,identity,workspaces,channels,messages,uploads,files,sounds}.rs`.
Removidos: `db.rs`, `models.rs`, `auth.rs` (antigo), `upload.rs`, `migrations/` e as deps
`sqlx`/`argon2`/`jsonwebtoken`. Novas: `iroh 1.3`, `iroh-gossip 0.101`, `iroh-blobs 0.103`,
`iroh-tickets 1.0`, `base64`, `hex`, `blake3`.

## Destaques

- **Identidade = chave do endpoint.** Segredo ed25519 em `data/identity.key` (0600); `EndpointId`
  é a identidade pública. `display_name` em `data/profile.json`. Backup exportável/importável.
- **Sessão local:** `session_token` aleatório no boot; a UI é aberta em `?k=<token>`.
- **Ticket de workspace** (`ticket.rs`): base64(JSON) `{id,name,owner_id,owner_name,secret,channels,
  endpoint_ticket}`. Topic de canal = `BLAKE3(secret||channel_id)[0..32]`.
- **Retenção 24h** (`retention.rs`): buffer por canal, dedupe por id, refcount de blobs por tag;
  GC a cada 5 min remove mensagens e blobs expirados.
- **Mensagem:** enviada → broadcast por gossip + buffer + push para a UI; recebida → dedupe, buffer,
  download dos blobs anexados, push para a UI.
- **Descoberta:** `presets::N0` (internet) ou `presets::Minimal` (`CADENCE_OFFLINE=1`), sempre com
  `MemoryLookup` alimentado pelos endereços dos tickets.

## Verificação (dois nós, `CADENCE_OFFLINE=1`)

- A cria workspace → **B faz join pelo ticket e passa a listar o workspace** (descoberta por ticket).
- A e B assinam `#geral`; A envia → **B recebe por gossip** e o `GET .../messages` de B traz a mensagem.
- A sobe um anexo → A envia → **B baixa o blob** e `GET /files/{hash}` de B é idêntico.
- Segurança: sem/with token inválido → 401; SPA com deep link → 200; `/api/sounds` → lista.

## Desvios

- `Id = String` em todo o sistema (`"{ts_ms}-{author_short}"` para mensagens).
- `/files/{hash}` não exige token (browsers não mandam `Authorization` em `<img>`).
- Import de identidade reconstrói o endpoint iroh em runtime (UI precisa re-assinar).
- Canais criados após o ticket não chegam a quem já entrou (ticket é snapshot) — item conhecido da v3.
