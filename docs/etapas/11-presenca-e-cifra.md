# Etapa 11 — Presença por workspace + cifra de conteúdo

Duas pendências de produto atacadas em paralelo após o rename para **Discard**.

## Presença de voz por workspace (Agent V)

**Problema:** a presença viajava no topic do canal de voz; só quem estava na chamada via quem estava
nela — a sidebar não mostrava os membros de canais que você não entrou.

**Solução:** um **topic de tempo real por workspace**, derivado do segredo:
`BLAKE3("discard-realtime-v1" || secret)`. O nó assina esse topic para cada workspace do usuário
(no `Ready` e no heartbeat de 10s), com bootstrap nos peers conhecidos (owner do ticket + descobertos).
Presença, **sinalização** e **soundboard** passam a trafegar nesse topic; **mensagens continuam nos
topics por canal**. Filtros preservados (`signal` por destinatário, `sound_play` pelo canal em que se está).

**Verificação (2 nós):** B entra no canal de voz, A fica fora — A **vê** B na sidebar
(`voice_joined`/`voice_presence`); `signal` A→B e `sound_play` chegam; texto segue no topic do canal.

## Cifra de conteúdo no cliente (Agent C)

**Objetivo:** o nó/DB só guarda **ciphertext**; peers com o segredo do workspace decifram.

- **Backend:** `Workspace` ganhou `secret: Option<String>` (populado de `workspaces.rs` a partir do
  store; é local e token-gated). `store.rs` mantém `secret: None` e a rota preenche.
- **Frontend (`lib/crypto.ts`, WebCrypto):**
  - `key = HKDF-SHA256(secret, info="discard-content-v1") → AES-256-GCM`.
  - Envelope: `content = "E1:" + base64(nonce[12] || ciphertext+tag)`.
  - Sem prefixo `E1:` ⇒ texto claro (fallback/compat). `secret` nulo ⇒ envia sem cifrar.
  - `state.svelte.ts` cifra no envio, decifra para exibir (cache por mensagem), chave por workspace.
- **Anexos não cifrados** (stretch não feito): servidos direto por `/files/<hash>`. Documentado.

**Escopo do modelo de ameaça:** a cifra protege em trânsito/em repouso e contra peers sem a chave. O
**nó local** (seu) conhece a chave — não é um servidor central. Ver `PRIVACY.md`.

## Config de dev

`DISCARD_TOKEN` permite **token fixo** (URL estável no `?k=`); `DISCARD_NO_OPEN=1` evita abrir o
navegador. `DISCARD_WEB_DIST` aponta a UI para outro diretório.

## Estado

`cargo build` e `npm run check`/`build` verdes com as duas mudanças juntas; CI (`release.yml`) gerando
`Discard-*.dmg` e `discard-*.tar.gz`.
