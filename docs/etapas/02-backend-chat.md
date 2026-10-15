# Stage 02 — Wave 1: chat backend (Agent A)

## Scope

A complete Axum server for **chat** (no voice and no real upload — those stayed as stubs for
Wave 2), with authentication, REST for workspaces/channels/messages and a WebSocket loop.

## Files created

`crates/server/src/`: `config.rs`, `error.rs`, `db.rs`, `models.rs`, `auth.rs`, `state.rs`,
`hub.rs`, `ws.rs`, `upload.rs` (stub), `main.rs`.
`crates/server/src/routes/`: `mod.rs`, `auth.rs`, `workspaces.rs`, `channels.rs`, `messages.rs`,
`uploads.rs` (stub), `sounds.rs`.

## What was completed

- **Config via env**: `DISCARD_ADDR`, `DISCARD_DB`, `DISCARD_UPLOADS`, `DISCARD_JWT_SECRET`.
- **Errors**: `AppError` → JSON `{"error": "..."}` with an appropriate HTTP status.
- **DB**: SQLite pool with `foreign_keys`, `WAL`, `create_if_missing`; automatic migrations on boot.
- **Auth**: argon2 for hashing, JWT HS256 (30 days), `AuthUser` extractor (bearer) via
  `FromRequestParts<AppState>`.
- **Hub**: connection registry, channel subscription, targeted send and broadcast; `DashMap` for
  concurrency; sender lists copied before sending so as not to hold locks.
- **WebSocket** (`/ws?token=`): validates JWT, sends `Ready`, handles `ping/pong`, `subscribe`, `send_message`,
  `typing`; 60s heartbeat. Voice/signaling/soundboard arms are **no-op** (Wave 2).
- **REST**: register (creates workspace "Meu servidor" + `geral`/`Geral`), login, me, list/create workspaces,
  create channel, list (paginated by `before`) and create message (with broadcast on the hub).

## Deviations from the contract (approved)

1. `sqlx` needed the **`macros`** feature (`sqlx::migrate!`).
2. `jsonwebtoken 11` requires a crypto provider: feature **`rust_crypto`** (pulls in p256/rsa etc.).
3. `argon2 0.6` removed `SaltString`/`OsRng`: use of `hash_password(bytes)` + `verify_password`.
4. Path params in the axum 0.8 syntax: `{id}`.
5. Pointwise `#[allow(dead_code)]` on items reserved for Wave 2 (voice hub, `store_file`).

## Verification (smoke test)

Temporary DB and port 8799. Results:

- `register`/`login`/`me`/`workspaces`/`channels`/`messages` — all 200 with correct JSON.
- Errors: duplicate user → 400; wrong password → 401; no bearer → 401; `upload` stub → 500.
- WS: handshake 101, `ready` with workspaces/channels, `ping`→`pong`, `subscribe` + `send_message`
  → broadcast `{"op":"message"}`.

`cargo build` clean (0 warnings, 0 errors).
