# Etapa 00 — Visão geral (P2P, efêmero, local-first)

> Revisado duas vezes: primeiro para P2P (sem servidor de conteúdo), depois para **mensagens efêmeras
> (24h)**. Ver `PRIVACY.md` e `CONTRACT.md` (v3).

## Objetivo

Clone de Discord **enxuto, P2P e efêmero**:

- **Workspaces** com canais de **texto** e **voz**.
- **Chat** em tempo real por gossip; **mensagens expiram em 24h** (sem histórico durável).
- **Upload de arquivos** por blobs content-addressed (TTL 24h).
- **Áudio/vídeo em tempo real** por WebRTC mesh, sinalizado pelo overlay P2P.
- **Efeitos sonoros** (soundboard) propagados por gossip.
- **Privacidade forte**: sem servidor de conteúdo, cifra E2E, zero terceiros.

## Arquitetura

Cada usuário roda um **nó local** em Rust; a **UI no navegador** é servida por esse nó e fala só com
ele (localhost). Os nós se falam por **iroh** (QUIC + TLS 1.3, NAT traversal, relays só como fallback).

```
[ Navegador A ] ──localhost──> [ Nó A ] <══ iroh ══> [ Nó B ] ──localhost──> [ Navegador B ]
```

| Necessidade | Protocolo iroh | Persistência |
|---|---|---|
| Tempo real (chat, presença, sinalização, soundboard) | **iroh-gossip** | nenhuma (efêmero, 24h em buffer local) |
| Arquivos / áudios | **iroh-blobs** (BLAKE3) | blob com TTL 24h + GC |
| Conexão, identidade, cifra, NAT | **iroh endpoint** | chave local |

> Descartado em relação à v1/v2: **SQLite/SQLx**, **autenticação por senha** e **iroh-docs/CRDT**.
> Não há histórico para replicar — só tempo real e um buffer de 24h.

## Infraestrutura necessária

Nãohá servidor de aplicação. Só o **relay** (fallback NAT, encaminha cifrado) e o **address lookup**
(DNS do iroh ou Mainline DHT) — ambos opcionais e self-hostáveis. Nada guarda conteúdo.

## Decisões (ADR resumido)

1. **Efêmero por design.** Mensagens TTL 24h, sem persistência nem sincronização de histórico.
2. **iroh como camada P2P.** Discagem por chave, QUIC E2E, relays como fallback cifrado.
3. **Capability-based access.** Acesso a um workspace é um **ticket autocontido** (contém o segredo e a
   config). Sem ticket, sem acesso.
4. **Mesh WebRTC para mídia** (até ~6 pessoas); ICE direto por decisão de produto.
5. **Nó local em Rust** (não é servidor central): agente iroh + serve a UI + mantém o buffer 24h.
6. **Frontend fino**, dono da criptografia de ponta (chave privada nunca vai ao nó).

## Estrutura do repositório

```
Discard/
├── Cargo.toml                 # workspace Rust
├── crates/
│   ├── shared/                # tipos do protocolo UI <-> nó (Id = String)
│   ├── server/                # bin `cadence-server` = NÓ LOCAL (Axum + iroh)
│   └── iroh-spike/            # spike descartável para validar a API do iroh
├── web/                       # UI Svelte 5 + Vite
├── docs/                      # documentação
├── data/ , uploads/           # estado local do nó (runtime)
```

## Roadmap (ondas)

| Onda | Escopo | Agentes |
|---|---|---|
| 1 | UI shell + chat + API local (protótipo) | A, F1 — **feito, será adaptado** |
| 2 | Spike iroh (gossip + blobs) | S |
| 3 | Nó local iroh (identidade, gossip, blobs, TTL 24h) | N1 |
| 4 | Frontend: identidade, tickets de convite, ids string | N2 |
| 5 | Voz/vídeo e soundboard sobre gossip | N-voice, F2, F3 |
| 6 | Integração, empacotamento, E2E | tech lead |

## Rodar (dev, alvo)

```bash
cargo run -p server          # nó local; abre http://127.0.0.1:8787/?k=<token>
cd web && npm install && npm run dev   # UI em 5173 durante o desenvolvimento
```
