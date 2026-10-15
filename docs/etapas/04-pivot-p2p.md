# Etapa 04 — Pivot para P2P (iroh)

## O que motivou

Dois requisitos do usuário mudaram a arquitetura:

1. **Privacidade forte** e, explicitamente, **nenhum servidor central guardando mensagens** —
   referência citada: `pears.com` / Keet ("the chat app that knows nothing").
2. **Mensagens efêmeras**: "não temos interesse em guardar mensagens… expira com um dia".

## Decisões (ADR)

- **P2P com iroh** (stack Rust: QUIC/TLS1.3, NAT traversal, relays como fallback cifrado).
- **Sem servidor de aplicação.** O que roda é um **nó local** por usuário, que serve a UI em localhost.
- **Sem histórico.** Mensagens TTL = **24h** num buffer local; sem `iroh-docs`/CRDT, sem banco.
- **Acesso por capability:** o convite é um **ticket autocontido** com o segredo do workspace.
- **Mídia** por WebRTC mesh (DTLS-SRTP E2E), sinalizada pelo overlay P2P.
- Descartados: SQLite/SQLx, argon2/JWT, `iroh-docs`.

## Spike (Agent S) — de-riscagem da API

Crate descartável `crates/iroh-spike` provou, rodando **offline em localhost**:

- gossip A→B (`broadcast`/`Event::Received`),
- blob A→B (`add_bytes` → `downloader.download` → `get_bytes`, integridade ok),
- `EndpointTicket` e `EndpointId`.

**Versões resolvidas:** `iroh 1.3.0`, `iroh-gossip 0.101.0`, `iroh-blobs 0.103.1`,
`iroh-tickets 1.0.0`.

**Gotchas críticos (API mudou vs. tutoriais antigos):**

- Não existe mais `endpoint.add_node_addr`; o address book é `MemoryLookup` +
  `.address_lookup(lookup).` e `lookup.add_endpoint_info(addr)`.
- `Gossip::builder().spawn(ep)` sozinho não aceita conexões — precisa registrar no `Router`
  (`.accept(iroh_gossip::ALPN, gossip.clone())`).
- `GossipReceiver` exige `futures_util::StreamExt` para `.next()`.
- Presets: `N0` (DNS+relay públicos, precisa internet), `Minimal` (offline), `Empty` (falha).
- Sempre envolver `joined()`/`next()` em timeout.

## Consequência

O protótipo das Etapas 02/03 (servidor central) foi **substituído**; ficou apenas como referência de
UI e de estrutura Axum. O contrato foi reescrito como **v3** (`docs/CONTRACT.md`).
