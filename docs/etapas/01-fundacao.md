# Stage 01 — Foundation

## Goal

Create the monorepo skeleton and **freeze the interface contract** before parallelizing the work
among subagents. Everything here was done by the tech lead.

## Environment

- Arch Linux, 4 vCPU, 11 GB RAM.
- Rust **was not installed** — installed via rustup (stable **1.99.0**).
- Node **26.8**, npm 11.19, Postgres 18, SQLite 3.53, Docker, ffmpeg available.
- Rust commands require `export PATH="$HOME/.cargo/bin:$PATH"`.

## Deliverables

1. **Rust workspace** (`Cargo.toml`) with `edition 2021`, a `release` profile with `lto = "thin"`,
   and dependencies centralized in `[workspace.dependencies]`.

2. **`crates/shared`** — shared protocol types (the most important interface in the project):
   `UserPublic`, `Workspace`, `Channel`, `ChannelKind`, `Attachment`, `Message`, `Sound`,
   `VoiceMember`, `VoicePresence`, REST requests, `SignalData` (offer/answer/ice),
   `ClientEvent` and `ServerEvent` (tagged with `op`). Both sides derive from this file.

3. **`crates/server`** — bin `discard-server`. Dependencies added with `cargo add`
   (versions resolved by crates.io, at the time): `axum 0.8`, `tower-http 0.7`, `sqlx 0.9`,
   `argon2 0.6`, `jsonwebtoken 11`, `dashmap 6`, `futures-util`, `mime_guess`, `rand 0.10`.

4. **Database schema** (`crates/server/migrations/0001_init.sql`): `users`, `workspaces`,
   `workspace_members`, `channels`, `attachments`, `messages`, `message_attachments`, `sounds`.
   Timestamps in RFC3339 (`strftime('%Y-%m-%dT%H:%M:%fZ','now')`) to decode directly into
   `DateTime<Utc>`.

5. **Frontend** — scaffold `npm create vite@latest web -- --template svelte-ts`
   (Svelte 5.57, Vite 8, TS 6). `vite.config.ts` with a proxy for `/api`, `/files` and `/ws` (incl. `ws: true`)
   to `127.0.0.1:8787`, parameterizable via `DISCARD_BACKEND`.

6. **`docs/CONTRACT.md`** — the source of truth: REST table, WebSocket semantics, WebRTC mesh
   algorithm, file structure with **owner per agent** and the frontend contracts (`api.ts`, `ws.ts`,
   `state.svelte.ts`).

## Decisions made with the user

- Media: **P2P mesh** for the MVP.
- Frontend: **Rust on the backend + lightweight frontend** in the browser.
- Database: **SQLite + SQLx**.

## Verification

- `cargo build` of the workspace: OK (baseline before any logic).

## Open items

- `app.members` in the frontend is derived (self + authors seen + members in voice) because there is no
  workspace members endpoint in the contract. Candidate for `GET /api/workspaces/:id/members`.
