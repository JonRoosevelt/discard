# Discard

Clone de Discord **minimalista, P2P e efêmero**.

- **Sem servidor central.** Cada usuário roda um **nó local** em Rust; os nós falam **iroh** (QUIC).
- **Efêmero.** Mensagens e anexos **expiram em 24h** — nada de histórico durável.
- **Workspaces** com canais de **texto** e **voz**; convite por **ticket** (capability).
- **Voz/vídeo** por **WebRTC mesh** (E2E por DTLS-SRTP); sinalização pelo overlay P2P.
- **Soundboard** com efeitos propagados por gossip.
- **Privacidade forte**: cifra E2E por padrão, zero terceiros, chaves só no dispositivo.
  Ver [`docs/PRIVACY.md`](docs/PRIVACY.md).

## Arquitetura

```
[ Navegador A ] --localhost--> [ Nó A ] <== iroh (gossip + blobs) ==> [ Nó B ] --localhost--> [ Navegador B ]
```

- **Nó local** (`crates/server`, bin `discard-server`): identidade (ed25519), endpoint/gossip/blobs do
  iroh, buffer local de 24h, e a API local (Axum) que serve a UI.
- **UI** (`web/`, Svelte 5 + Vite): fala só com o nó local; mídia por WebRTC.
- **Infra opcional** (não vê conteúdo): relay do iroh (fallback cifrado) e address lookup — ambos
  self-hostáveis.

## Rodar

```bash
# nó local (abre a UI com o token em http://127.0.0.1:8787/?k=...)
cargo run -p server

# UI em modo dev (opcional)
cd web && npm install && npm run dev
```

Variáveis: `DISCARD_ADDR` (`127.0.0.1:8787`), `DISCARD_DATA_DIR` (`data`), `DISCARD_OFFLINE`
(`1` = sem relay/DNS, para testes locais).

## Stack

Rust (Axum + iroh + iroh-gossip + iroh-blobs) · Svelte 5 (runes) + Vite + TypeScript · WebRTC · WebCrypto.

## Documentação

Toda a documentação e o histórico de decisões por etapa estão em [`docs/`](docs/README.md).
