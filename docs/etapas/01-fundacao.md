# Etapa 01 — Fundação

## Objetivo

Criar o esqueleto do monorepo e **congelar o contrato de interface** antes de paralelizar o trabalho
entre subagents. Tudo aqui foi feito pelo tech lead.

## Ambiente

- Arch Linux, 4 vCPU, 11 GB RAM.
- Rust **não estava instalado** — instalado via rustup (stable **1.99.0**).
- Node **26.8**, npm 11.19, Postgres 18, SQLite 3.53, Docker, ffmpeg disponíveis.
- Comandos Rust exigem `export PATH="$HOME/.cargo/bin:$PATH"`.

## Entregas

1. **Workspace Rust** (`Cargo.toml`) com `edition 2021`, perfil `release` com `lto = "thin"`,
   e dependências centralizadas em `[workspace.dependencies]`.

2. **`crates/shared`** — tipos de protocolo compartilhados (a interface mais importante do projeto):
   `UserPublic`, `Workspace`, `Channel`, `ChannelKind`, `Attachment`, `Message`, `Sound`,
   `VoiceMember`, `VoicePresence`, requests de REST, `SignalData` (offer/answer/ice),
   `ClientEvent` e `ServerEvent` (taggeados com `op`). Ambos os lados derivam deste arquivo.

3. **`crates/server`** — bin `discard-server`. Dependências adicionadas com `cargo add`
   (versões resolvidas pelo crates.io, à época): `axum 0.8`, `tower-http 0.7`, `sqlx 0.9`,
   `argon2 0.6`, `jsonwebtoken 11`, `dashmap 6`, `futures-util`, `mime_guess`, `rand 0.10`.

4. **Schema do banco** (`crates/server/migrations/0001_init.sql`): `users`, `workspaces`,
   `workspace_members`, `channels`, `attachments`, `messages`, `message_attachments`, `sounds`.
   Timestamps em RFC3339 (`strftime('%Y-%m-%dT%H:%M:%fZ','now')`) para decodificar direto em
   `DateTime<Utc>`.

5. **Frontend** — scaffold `npm create vite@latest web -- --template svelte-ts`
   (Svelte 5.57, Vite 8, TS 6). `vite.config.ts` com proxy de `/api`, `/files` e `/ws` (incl. `ws: true`)
   para `127.0.0.1:8787`, parametrizável por `DISCARD_BACKEND`.

6. **`docs/CONTRACT.md`** — a fonte de verdade: tabela REST, semântica do WebSocket, algoritmo do mesh
   WebRTC, estrutura de arquivos com **dono por agente** e contratos do frontend (`api.ts`, `ws.ts`,
   `state.svelte.ts`).

## Decisões tomadas com o usuário

- Mídia: **mesh P2P** para o MVP.
- Frontend: **Rust no backend + frontend leve** no navegador.
- Banco: **SQLite + SQLx**.

## Verificação

- `cargo build` do workspace: OK (baseline antes de qualquer lógica).

## Pendências abertas

- `app.members` no frontend é derivado (self + autores vistos + membros em voz) porque não há endpoint
  de membros de workspace no contrato. Candidato a `GET /api/workspaces/:id/members`.
