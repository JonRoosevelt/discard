# Discard — Documentação

Clone minimalista de Discord: **workspaces**, canais de **texto e voz**, **upload de arquivos**,
**áudio/vídeo em tempo real**, **efeitos sonoros** e **privacidade forte** — tudo **P2P**
(sem servidor central) e **efêmero** (mensagens expiram em 24h).

| Documento | Conteúdo |
|---|---|
| [CONTRACT.md](./CONTRACT.md) | Fonte de verdade das interfaces (REST local, WS, tipos, topics, donos de arquivo) |
| [PRIVACY.md](./PRIVACY.md) | Modelo de ameaça, retenção (24h), cifra de conteúdo |
| [DIST.md](./DIST.md) | Distribuição e instalação (binário único, Linux e macOS) |
| [etapas/00-visao-geral.md](./etapas/00-visao-geral.md) | Visão geral, stack, decisões e roadmap |
| [etapas/01-fundacao.md](./etapas/01-fundacao.md) | Workspace Rust, protocolo compartilhado, scaffolds |
| [etapas/02-backend-chat.md](./etapas/02-backend-chat.md) | Protótipo: backend de chat (substituído no pivot) |
| [etapas/03-frontend-chat.md](./etapas/03-frontend-chat.md) | Protótipo: UI de chat (substituída no pivot) |
| [etapas/04-pivot-p2p.md](./etapas/04-pivot-p2p.md) | Pivot para P2P (iroh) + spike de de-riscagem |
| [etapas/05-no-local-iroh.md](./etapas/05-no-local-iroh.md) | Nó local iroh (identidade, gossip, blobs, 24h) |
| [etapas/06-frontend-p2p.md](./etapas/06-frontend-p2p.md) | Frontend P2P: identidade, tickets, arquivos |
| [etapas/07-voz-video.md](./etapas/07-voz-video.md) | Voz/vídeo: presença/sinalização + mesh WebRTC |
| [etapas/08-soundboard.md](./etapas/08-soundboard.md) | Soundboard + efeitos builtin |
| [etapas/09-integracao.md](./etapas/09-integracao.md) | Integração, E2E, como rodar, limitações |
| [etapas/10-distribuicao.md](./etapas/10-distribuicao.md) | Binário único (UI embutida) e empacotamento |
| [etapas/11-presenca-e-cifra.md](./etapas/11-presenca-e-cifra.md) | Presença por workspace + cifra de conteúdo |

Ordem de leitura: **CONTRACT → PRIVACY → etapas/00 → 04 → … → 11**.
