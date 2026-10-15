# Etapa 09 — Integração e verificação final (tech lead)

## Builds

- `cargo build` — limpo (workspace: `shared`, `server`, `iroh-spike`).
- `cd web && npm run check` — 0 erros, 0 warnings. `npm run build` — ok.

## Smoke test E2E (dois nós)

Provado no nó local (`CADENCE_OFFLINE=1`, portas e dirs distintos):

1. dois nós criam identidade;
2. A cria workspace → B faz **join por ticket** e passa a listar o workspace;
3. mensagem enviada em A aparece em B (gossip) e no buffer 24h;
4. anexo replicado: B baixa o blob e `/files/{hash}` confere;
5. voz: ambos entram no canal, presença cruzada, `signal` relayado, `sound_play` propagado;
6. `/api/sounds` (7 builtin) e upload de som.

## Validação visual (UI real servida pelo nó)

No navegador via `http://127.0.0.1:8787/?k=<token>`:

- tela de **identidade local** (criar/importar/exportar backup), com o texto "Mensagens desaparecem em 24h";
- shell com rail, sidebar (Canais de texto / Canais de voz), chat com selo **24H** no header;
- criar workspace → canais `#geral` e `🔊 Geral`;
- menu criar/entrar com ticket, modal de convite;
- entrar no canal de voz → painel com tile, controles e **soundboard com os 7 efeitos**; falha de mic
  tratada de forma discreta.

## Correções pós-validação

- **Layout responsivo (Shell):** o painel de voz reservava 320px **sempre** (mesmo fora de chamada),
  esmagando o chat em janelas estreitas (72+240+320 = 632px). Agora o painel só existe quando o usuário
  está num canal de voz (`class:incall`) e há breakpoints: `≤1100px` painel 280px; `≤880px` painel
  escondido; `≤520px` sidebar escondida. O chat usa `minmax(0, 1fr)` e nunca colapsa.

## Como rodar

```bash
# nó local (abre a UI com o token)
cargo run -p server
# online (descoberta real): sem CADENCE_OFFLINE
# offline/local: CADENCE_OFFLINE=1
```

Variáveis: `CADENCE_ADDR` (default `127.0.0.1:8787`), `CADENCE_DATA_DIR` (default `data`),
`CADENCE_OFFLINE`.

Desenvolvimento da UI:

```bash
cd web && npm install && npm run dev   # 5173, proxy para o nó em 8787
```

## Limitações conhecidas / próximos passos

- Canais criados **depois** do convite não chegam a quem já entrou (o ticket é um snapshot). Solução:
  gossip de `workspace_update` assinado pelo owner.
- Sem persistência de mensagens por design (TTL 24h); entrar num canal não traz backfill.
- `presets::N0` usa DNS/relay públicos da n0; para privacidade máxima, **self-host** de relay e/ou
  lookup por Mainline DHT (previsto).
- Cifra de conteúdo no cliente (AES-GCM/WC) sobre o documento do gossip é opcional e ainda não ligada.
- Empacotamento (binário + `web/dist` embutido) e ícone/serviço de usuário não foram feitos.

## Decisão sobre "servidor"

Não há servidor de aplicação. Só relay (fallback, cifrado, self-hostável) e address lookup. O binário
`cadence-server` é o **nó local** que cada usuário roda — não é um servidor central.
