# Discard — Interface Contract v3 (P2P, efêmero, local-first)

> **v3.** Sem servidor central **e** sem armazenamento de mensagens: **mensagens expiram em 24h**.
> Sem CRDT, sem banco de histórico. Cada usuário roda um **nó local** em Rust que fala **iroh**.
> Ver `PRIVACY.md` e `etapas/00-visao-geral.md`.

## Infraestrutura necessária (o que sobra)

Nenhum servidor de aplicação. Só infraestrutura **opcional e substituível**, que não vê conteúdo:

| Peça | Para quê | Padrão | Self-host |
|---|---|---|---|
| Relay iroh | Furar NAT / fallback; encaminha pacotes **já cifrados** | relays públicos da n0 | sim (`presets::N0` → relay próprio) |
| Address lookup | Descobrir o endereço de um peer pela chave | DNS do iroh (`dns.iroh.link`) | sim (Mainline DHT, BEP 44) |

## Topologia

```
[ Navegador A ] --HTTP/WS localhost--> [ Nó A ] <══ iroh (QUIC/TLS1.3) ══> [ Nó B ] <--localhost--> [ Navegador B ]
                                     gossip (tempo real) + blobs (arquivos)
```

## Retenção (regra dura)

- **Mensagens:** efêmeras. TTL = **24h**. Vivem num **buffer circular local** do nó; nada é enviado
  para armazenamento. Nunca são replicadas para persistir. Ao expirar, somem.
- **Anexos/sons:** blobs com TTL de 24h e **GC**; removidos quando a mensagem que os referencia expira.
- **Config do workspace** (nome, canais, membros): essa sim persiste **localmente** em cada usuário,
  porque é o que dá sentido ao workspace. Não é conteúdo de conversa.
- Backfill opcional ao entrar: um peer pode enviar as últimas mensagens **ainda vivas** (<= 24h).

## Conceitos

| Conceito | Implementação |
|---|---|
| Identidade | par de chaves ed25519 local = chave do endpoint iroh (sem e-mail/senha) |
| Workspace | topic secreto (gossip) + config assinada pelo owner. **Sem documento/CRDT** |
| Convite / acesso | **ticket autocontido** (base64): `{id, name, secret, channels, owner}` — é uma capability |
| Canal | id uuid; topic de texto = `BLAKE3(secret || channel_id)` |
| Workspace (tempo real) | topic = `BLAKE3("discard-realtime-v1" || secret)`; carrega presença de voz, sinalização e soundboard |
| Mensagem | efêmera (24h), transportada por gossip **no topic do canal**; id `"{ts_ms}-{author_short}"` |
| Anexo | blob (BLAKE3), TTL 24h; `url = /files/<hash>` |
| Tempo real (texto) | gossip topic por canal |
| Presença de voz / sinalização / soundboard | gossip no **topic do workspace** (visível a todos os membros, qualquer canal) |

## Identidade e sessão local

- Chave privada em `data/identity.key` (`0600`), opcionalmente com passphrase. Nunca vai à rede.
- Sessão local: no boot o nó gera `session_token` e abre `http://127.0.0.1:<porta>/?k=<token>`.
  `/api` e `/ws` exigem `Authorization: Bearer <session_token>` (proteção contra outros sites).

## REST local (`/api`)

| Método | Rota | Body | Retorno |
|---|---|---|---|
| GET | `/api/health` | – | `{"ok":true}` |
| GET | `/api/identity` | – | `Identity` ou 404 |
| POST | `/api/identity` | `{display_name}` | `Identity` |
| GET | `/api/identity/backup` | – | download do backup |
| POST | `/api/identity/import` | `{backup, passphrase?}` | `Identity` |
| GET | `/api/workspaces` | – | `Workspace[]` |
| POST | `/api/workspaces` | `{name}` | `Workspace` (**traz o `ticket`**) |
| POST | `/api/workspaces/join` | `{ticket}` | `Workspace` |
| GET | `/api/workspaces/{id}` | – | `Workspace` |
| GET | `/api/workspaces/{id}/ticket` | – | `{ticket}` |
| POST | `/api/workspaces/{id}/channels` | `{name, kind}` | `Channel` |
| GET | `/api/channels/{id}/messages` | – | `Message[]` (só o que ainda está vivo) |
| POST | `/api/channels/{id}/messages` | `{content, attachment_ids}` | `Message` |
| POST | `/api/upload` | multipart `file` | `Attachment` |
| GET | `/files/{hash}` | – | bytes do blob |
| GET | `/api/sounds` | – | `Sound[]` |
| POST | `/api/sounds` | multipart `file`,`name`,`emoji` | `Sound` |

Erros: `{"error":"..."}` com status adequado.

## WebSocket local `/ws?token=<session_token>`

`ClientEvent` / `ServerEvent` em `crates/shared`. Ao conectar: `Ready { identity, workspaces, voice, sounds }`.

- O nó assina o **topic do workspace** de cada workspace conhecido (no boot/tick e ao conectar a UI),
  usando como bootstrap o `endpoint_ticket` do owner e os peers descobertos via `NeighborUp`.
- `subscribe` → nó entra no gossip topic (texto) do canal.
- `send_message` → nó faz `gossip.broadcast` (efêmero) no topic do canal e guarda no buffer local de 24h.
- `voice_join/update/leave`, `signal`, `sound_play` → gossip no **topic do workspace**.
- `Ready.voice` traz presenças de **todos os canais** do usuário; a sidebar filtra por workspace.
- `signal` é filtrado pelo destinatário (`to`); `sound_play` só é repassado a quem está no canal de voz.
- Novidades de peers (`gossip Event::Received`) viram `ServerEvent` para as UIs locais.
- `ping`/`pong`.

## Tipos (`crates/shared`)

**`Id = String`**. `Identity { public_id, display_name, has_backup }`, `UserPublic { id, display_name,
avatar_url }`, `Workspace { id, name, icon_url, owner_id, ticket, secret, channels }`, `Channel`,
`Attachment`, `Message`, `Sound`, `VoiceMember`, `VoicePresence`. Requests e eventos conforme acima.

`Workspace.secret` é o segredo hex do workspace (o mesmo do `ticket`). O nó local (localhost+token) o
expõe à própria UI para derivar a chave de conteúdo. **Conteúdo das mensagens:** a UI cifra o corpo
antes de enviar e o envelope vai no campo `Message.content` como `"E1:" + base64(nonce(12) ||
ciphertext)` (AES-256-GCM; chave = HKDF-SHA256(secret, info="discard-content-v1")). Sem prefixo `E1:`,
o conteúdo é tratado como texto claro (fallback/legado). O nó/banco armazena apenas o envelope.

## Mesh WebRTC (inalterado)

Novato inicia offers; demais respondem. ICE direto (sem TURN por padrão). Sinalização: UI → nó local →
gossip → nó remoto → UI.

## Estrutura de arquivos (dono = quem edita)

Backend `crates/server/src/` — **Agent N1**: `identity.rs`, `iroh_net.rs` (endpoint/gossip/blobs +
lookup), `retention.rs` (buffer 24h + GC), `hub.rs` (fanout UIs locais), `main.rs`, `config.rs`,
`error.rs`, `state.rs`, `ticket.rs`, `routes/*`.
**Agent N-voice**: `voice.rs` (presença + signaling via gossip), `routes/sounds.rs`.

Frontend `web/src/` — **Agent N2**: `lib/api.ts`, `lib/ws.ts`, `lib/protocol.ts`, `lib/state.svelte.ts`,
`lib/identity.ts`, `components/Login.svelte` (→ identidade), `components/InviteDialog.svelte`,
`App.svelte` (ids string). **Agent F2**: `lib/rtc.ts`, `components/VoicePanel.svelte`,
`components/VideoTile.svelte`. **Agent F3**: `components/Soundboard.svelte`.

## Estilo

Svelte 5 runes; dark-first minimalista; sem comentários; sem terceiros (fontes/CDN/telemetria);
`cargo build` + `npm run build` + `npm run check` verdes; **nunca commitar**.
