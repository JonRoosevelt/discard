# Stage 11 — Per-workspace presence + content encryption

Two product pending items tackled in parallel after the rename to **Discard**.

## Per-workspace voice presence (Agent V)

**Problem:** presence traveled on the voice channel topic; only those who were in the call saw who was
in it — the sidebar did not show the members of channels you had not joined.

**Solution:** a **real-time topic per workspace**, derived from the secret:
`BLAKE3("discard-realtime-v1" || secret)`. The node subscribes to this topic for each of the user's workspaces
(in `Ready` and in the 10s heartbeat), with bootstrap on the known peers (ticket owner + discovered ones).
Presence, **signaling** and **soundboard** now travel on this topic; **messages remain on the
per-channel topics**. Filters preserved (`signal` by recipient, `sound_play` by the channel you are in).

**Verification (2 nodes):** B joins the voice channel, A stays out — A **sees** B in the sidebar
(`voice_joined`/`voice_presence`); `signal` A→B and `sound_play` arrive; text continues on the channel topic.

## Content encryption in the client (Agent C)

**Goal:** the node/DB stores only **ciphertext**; peers with the workspace secret decrypt it.

- **Backend:** `Workspace` gained `secret: Option<String>` (populated from `workspaces.rs` from the
  store; it is local and token-gated). `store.rs` keeps `secret: None` and the route fills it in.
- **Frontend (`lib/crypto.ts`, WebCrypto):**
  - `key = HKDF-SHA256(secret, info="discard-content-v1") → AES-256-GCM`.
  - Envelope: `content = "E1:" + base64(nonce[12] || ciphertext+tag)`.
  - Without the `E1:` prefix ⇒ plaintext (fallback/compat). Null `secret` ⇒ sends unencrypted.
  - `state.svelte.ts` encrypts on send, decrypts for display (cache per message), key per workspace.
- **Attachments not encrypted** (stretch not done): served directly by `/files/<hash>`. Documented.

**Threat model scope:** the encryption protects in transit/at rest and against peers without the key. The
**local node** (yours) knows the key — it is not a central server. See `PRIVACY.md`.

## Dev config

`DISCARD_TOKEN` allows a **fixed token** (stable URL in `?k=`); `DISCARD_NO_OPEN=1` avoids opening the
browser. `DISCARD_WEB_DIST` points the UI to another directory.

## Status

`cargo build` and `npm run check`/`build` green with both changes together; CI (`release.yml`) generating
`Discard-*.dmg` and `discard-*.tar.gz`.
