# Etapa 10 — Distribuição (binário único) e empacotamento

## Objetivo

Transformar o nó local num **app de arquivo único** (a UI embutida no binário), que **abre o
navegador sozinho**, e gerar instaladores para **Linux (omarchy/Arch)** e **macOS**.
Ver `docs/DIST.md` para o guia do usuário.

## Núcleo (tech lead)

- **UI embutida:** feature opcional `embed-ui` (`crates/server/Cargo.toml`) usando `rust-embed`
  (com `interpolate-folder-path`) apontando para `web/dist`. `crates/server/src/assets.rs` serve os
  assets com `Content-Type` e `Cache-Control` (imutável para `assets/*`), com fallback SPA para
  `index.html`. Sem a feature, o dev continua servindo `web/dist` do disco via `ServeDir`.
- **Auto-abrir navegador:** no boot, `xdg-open` (Linux) / `open` (macOS), desligável com
  `CADENCE_NO_OPEN=1`. O URL usa `127.0.0.1` mesmo quando o bind é `0.0.0.0`.
- **Binário renomeado** para `cadence` (era `cadence-server`).
- **Config nova:** `CADENCE_NO_OPEN`, `CADENCE_TOKEN` (token fixo em dev), `CADENCE_WEB_DIST`.
- **Ícone:** `assets/icon.svg` + `assets/icon.png` (1024²).

## Linux (`scripts/build.sh`, `scripts/package-linux.sh`)

- `build.sh`: builda a UI e o release com `embed-ui`.
- `package-linux.sh`: gera `dist/cadence-0.1.0-linux-<arch>.tar.gz` com o binário único, o ícone e um
  `install.sh` que instala em `~/.local/bin/cadence`, registra ícone/atalho no XDG e cria o launcher
  `cadence-app` com `CADENCE_DATA_DIR=~/.local/share/cadence`.

**Verificado nesta máquina:** o binário roda **sem** `web/dist` ao lado (index/deep-link/som = 200);
`install.sh` num HOME falso instalou tudo e o launcher subiu com os dados no XDG.

## macOS (`scripts/package-macos.sh` + `.github/workflows/release.yml`)

- **Não há cross-compile:** o `.app`/`.dmg` **precisa ser construído no macOS** (num Mac ou no CI
  `macos-latest`). O script monta `Cadence.app` (launcher com `CADENCE_DATA_DIR=~/Library/Application
  Support/Cadence`, `Info.plist` com permissões de mic/câmera, `icon.icns`), assina **ad-hoc** e cria o
  `.dmg`.
- **Gatekeeper:** app não notarizado exige remover a quarentena no primeiro uso:
  `xattr -dr com.apple.quarantine /Applications/Cadence.app` (ou botão direito > Abrir).
- **CI:** `release.yml` gera `.dmg` (macos) e `.tar.gz` (linux). Alternativa: buildar no Mac acessível
  por Tailscale.

## Limitações

- Assinatura ad-hoc serve para uso local; distribuição limpa exige Developer ID + notarização.
- AppImage ainda não é produzido (o tarball + `install.sh` cobrem a instalação).
