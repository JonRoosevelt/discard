# Discard — Documentation

A minimalist Discord clone: **workspaces**, **text and voice** channels, **file uploads**,
**real-time audio/video**, **sound effects** and **strong privacy** — all **P2P**
(no central server) and **ephemeral** (messages expire in 24h).

| Document | Contents |
|---|---|
| [CONTRACT.md](./CONTRACT.md) | Source of truth for the interfaces (local REST, WS, types, topics, file owners) |
| [PRIVACY.md](./PRIVACY.md) | Threat model, retention (24h), content encryption |
| [DIST.md](./DIST.md) | Distribution and installation (single binary, Linux and macOS) |
| [etapas/00-visao-geral.md](./etapas/00-visao-geral.md) | Overview, stack, decisions and roadmap |
| [etapas/01-fundacao.md](./etapas/01-fundacao.md) | Rust workspace, shared protocol, scaffolds |
| [etapas/02-backend-chat.md](./etapas/02-backend-chat.md) | Prototype: chat backend (replaced in the pivot) |
| [etapas/03-frontend-chat.md](./etapas/03-frontend-chat.md) | Prototype: chat UI (replaced in the pivot) |
| [etapas/04-pivot-p2p.md](./etapas/04-pivot-p2p.md) | Pivot to P2P (iroh) + de-risking spike |
| [etapas/05-no-local-iroh.md](./etapas/05-no-local-iroh.md) | Local iroh node (identity, gossip, blobs, 24h) |
| [etapas/06-frontend-p2p.md](./etapas/06-frontend-p2p.md) | P2P frontend: identity, tickets, files |
| [etapas/07-voz-video.md](./etapas/07-voz-video.md) | Voice/video: presence/signaling + WebRTC mesh |
| [etapas/08-soundboard.md](./etapas/08-soundboard.md) | Soundboard + builtin effects |
| [etapas/09-integracao.md](./etapas/09-integracao.md) | Integration, E2E, how to run, limitations |
| [etapas/10-distribuicao.md](./etapas/10-distribuicao.md) | Single binary (embedded UI) and packaging |
| [etapas/11-presenca-e-cifra.md](./etapas/11-presenca-e-cifra.md) | Per-workspace presence + content encryption |

Reading order: **CONTRACT → PRIVACY → etapas/00 → 04 → … → 11**.
