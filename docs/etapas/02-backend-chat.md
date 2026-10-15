# Etapa 02 — Wave 1: backend de chat (Agent A)

## Escopo

Servidor Axum completo para **chat** (sem voz e sem upload real — esses ficaram como stubs para a
Wave 2), com autenticação, REST de workspaces/canais/mensagens e loop de WebSocket.

## Arquivos criados

`crates/server/src/`: `config.rs`, `error.rs`, `db.rs`, `models.rs`, `auth.rs`, `state.rs`,
`hub.rs`, `ws.rs`, `upload.rs` (stub), `main.rs`.
`crates/server/src/routes/`: `mod.rs`, `auth.rs`, `workspaces.rs`, `channels.rs`, `messages.rs`,
`uploads.rs` (stub), `sounds.rs`.

## O que ficou pronto

- **Config por env**: `DISCARD_ADDR`, `DISCARD_DB`, `DISCARD_UPLOADS`, `DISCARD_JWT_SECRET`.
- **Erros**: `AppError` → JSON `{"error": "..."}` com status HTTP adequado.
- **DB**: pool SQLite com `foreign_keys`, `WAL`, `create_if_missing`; migrações automáticas no boot.
- **Auth**: argon2 para hash, JWT HS256 (30 dias), extractor `AuthUser` (bearer) via
  `FromRequestParts<AppState>`.
- **Hub**: registro de conexões, assinatura de canais, envio direcionado e broadcast; `DashMap` para
  concorrência; listas de senders copiadas antes do envio para não segurar locks.
- **WebSocket** (`/ws?token=`): valida JWT, envia `Ready`, trata `ping/pong`, `subscribe`, `send_message`,
  `typing`; heartbeat de 60s. Arms de voz/signaling/soundboard são **no-op** (Wave 2).
- **REST**: register (cria workspace "Meu servidor" + `geral`/`Geral`), login, me, listar/criar workspaces,
  criar canal, listar (paginado por `before`) e criar mensagem (com broadcast no hub).

## Desvios do contrato (aprovados)

1. `sqlx` precisou da feature **`macros`** (`sqlx::migrate!`).
2. `jsonwebtoken 11` exige um provider de cripto: feature **`rust_crypto`** (puxa p256/rsa etc.).
3. `argon2 0.6` removeu `SaltString`/`OsRng`: uso de `hash_password(bytes)` + `verify_password`.
4. Path params na sintaxe do axum 0.8: `{id}`.
5. `#[allow(dead_code)]` pontual em itens reservados à Wave 2 (hub de voz, `store_file`).

## Verificação (smoke test)

DB temporário e porta 8799. Resultados:

- `register`/`login`/`me`/`workspaces`/`channels`/`messages` — todos 200 com JSON correto.
- Erros: usuário duplicado → 400; senha errada → 401; sem bearer → 401; `upload` stub → 500.
- WS: handshake 101, `ready` com workspaces/canais, `ping`→`pong`, `subscribe` + `send_message`
  → broadcast `{"op":"message"}`.

`cargo build` limpo (0 warnings, 0 errors).
