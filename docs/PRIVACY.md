# Cadence — Privacidade e modelo de ameaça

Requisito de primeira classe: **não existe servidor central guardando mensagens**. Todo conteúdo
vive nos dispositivos dos participantes e é replicado **peer-to-peer**. Este documento define o que
prometemos, o que não prometemos, e por quê.

## Princípios

1. **Local-first.** O dispositivo do usuário é a fonte da verdade. Não há banco de dados central.
2. **Sem servidor de conteúdo.** Nenhum operador vê mensagens, arquivos, áudio ou vídeo.
3. **Criptografia ponta a ponta por padrão.** Não é opcional nem "enterprise-only".
4. **Sem terceiros.** Nenhuma telemetria, analytics, CDN, fonte ou chamada a serviço externo.
   Fontes são as do sistema; todo asset é local.
5. **Minimização de metadados.** Coletamos o mínimo e não logamos conteúdo.
6. **Verificável.** Só dependências que podem ser auditadas; sem binários opacos.

## Retenção de dados

**Não armazenamos mensagens.** Elas são efêmeras: vivem no máximo **24 horas**, só no buffer local de
cada nó (memória + cache local), e são descartadas automaticamente. Não há histórico durável, não há
sincronização de histórico entre dispositivos, e nada é enviado para armazenamento. Anexos e áudios do
soundboard seguem a mesma janela de 24h e são coletados (GC) depois. Só a **configuração do workspace**
(nome, canais) persiste localmente, porque não é conteúdo de conversa.

## Modelo de ameaça

| Adversário | O que quer | Como nos defendemos | O que resta |
|---|---|---|---|
| Operador de um servidor central | Ler mensagens, perfil de uso | Não existe servidor de conteúdo | — |
| MITM na rede | Ler/escrever tráfego | QUIC + TLS 1.3, chaves verificadas por identidade criptográfica | — |
| Peer de um workspace | Ler canais sem acesso | Acesso ao conteúdo é uma **capability** (chave do documento); sem a chave, o dado é opaco | Um membro legítimo vê o que lhe foi concedido |
| Operador do relay (fallback) | Correlacionar, ler conteúdo | Relay só encaminha **pacotes já cifrados**; nunca vê conteúdo | Vê EndpointIDs, IPs e timing (ver Metadados) |
| Observador de rede / ISP | Metadados | QUIC cifrado; DHT/discovery configurável | Volume, timing, IPs de relay |
| Peer malicioso | Expor IP, floodar | Conexões diretas são escolha do usuário; rate-limit e bloqueio local | IP é visível a quem se conecta direto (ver abaixo) |

## Arquitetura de privacidade

### Transporte — `iroh`
Cada nó é um `iroh::Endpoint` identificado por uma **chave criptográfica** (ed25519), não por IP.
Conectar é "discar por chave". Toda conexão é **QUIC + TLS 1.3**, autenticada e cifrada ponta a ponta.
Relays públicos existem só como **fallback** para furar NAT e encaminham pacotes **já cifrados**;
nenhum dado é armazenado na nuvem.

### Identidade
A identidade do usuário é um **par de chaves gerado no dispositivo**. A chave privada **nunca sai do
dispositivo** (IndexedDB no cliente de navegador; arquivo local com permissão `0600` no nó). Não há
e-mail, telefone ou conta em servidor. O `display_name` é local e não autenticado.
Backup = **exportação manual** de um arquivo de recuperação (a chave não é derivada da senha).

### Confidencialidade do conteúdo
- **Mensagens e estado:** vivem num **iroh Document** (KV multiwriter por CRDT). A identidade do
  documento é a chave pública de um par que **controla a escrita**. Compartilhar o `DocTicket` é o ato
  de conceder acesso. Sem o ticket, o documento é inalcançável.
- **Arquivos:** vão para o **blobs store** content-addressed (BLAKE3). O hash só significa algo para
  quem já tem acesso ao documento que o referencia.
- **Mídia:** em mesh WebRTC, o áudio/vídeo é cifrado ponta a ponta por **DTLS-SRTP** entre os pares.
  O nó local não é intermediário da mídia.
- **Defesa em profundidade (opcional):** o corpo de cada mensagem pode ser cifrado com uma chave
  simétrica do workspace (AES-GCM, via WebCrypto) antes de entrar no documento, de modo que nem o
  documento nem os blobs contenham texto claro.

### Cifra em repouso
- Nó local: store de documentos e blobs em disco; a chave de identidade protegida por permissões e,
  opcionalmente, passphrase local.
- Navegador: chave privada e material sensível em IndexedDB; nada em `localStorage` além do token de
  sessão do nó local (que é efêmero e local).

## Metadados — o que ainda vaza (honestidade)

1. **IP entre pares diretos.** Conexões diretas (escolha do produto, menor latência) revelam o IP a
   quem você conecta. Quem quiser esconder o IP pode exigir relay; está previsto como opção por
   workspace. **Não é o padrão.**
2. **Descoberta.** Por padrão o `iroh` usa DNS/relay da n0 para localizar peers. Para privacidade
   máxima: **self-host de relay** e/ou **lookup via Mainline DHT** (`BEP 44`), ambos configuráveis.
3. **Timing e volume.** Um relay de fallback vê quando e quanto você troca, ainda que não veja o quê.
4. **Presença.** Estar online e em um canal é visível para os membros daquele workspace.

## O que NÃO prometemos

- Anonimato de rede (o IP é exposto a peers diretos e a relays).
- Resistência a um adversário que controle **todos** os seus peers e o seu dispositivo.
- Que membros de um canal não vazem conteúdo (eles têm acesso legítimo por definição).
- Deniability criptográfica (as entradas de documento são **assinadas** pelo autor).

## Decisões que sustentam a privacidade

- **Sem servidor central** — não há alvo a confiscar; não há "nós vemos suas mensagens".
- **Capability-based access** — acesso é uma chave, não um registro de permissão num servidor.
- **Zero terceiros** — nada de Google Fonts/Analytics/STUN do Google.
- **Chaves só no dispositivo** — comprometer o servidor não compromete o usuário.

## Checklist de verificação

- [ ] Nenhuma requisição de rede a domínios de terceiros (auditar `web/` e deps).
- [ ] Chave privada nunca serializada para o nó/rede.
- [ ] Sem log de conteúdo (mensagens, corpo de arquivos, SDP/ICE) em nível `info`.
- [ ] Relays e lookup self-hostáveis por configuração.
- [ ] Opção por workspace de "forçar relay / esconder IP".
- [ ] Exportação/importação de backup de chave.
