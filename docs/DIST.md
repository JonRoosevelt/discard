# Discard — Distribuição e instalação

O Discard é um app **local-first**: cada usuário roda o seu próprio nó (`discard`) na máquina e a
UI fala com ele em `http://127.0.0.1:<porta>`. Não há servidor central para instalar.

## Build

Na raiz do repositório:

```sh
cd web && npm ci && npm run build && cd ..
cargo build --release --features embed-ui
```

Isso gera **`target/release/discard`**. Com a feature `embed-ui`, o conteúdo de `web/dist` é
**embutido no binário**: o resultado é um **arquivo único**, sem a pasta `web/dist` ao lado.

Ao subir, o nó imprime:

```
Discard pronto: http://127.0.0.1:<porta>/?k=<token>
```

e **abre o navegador** automaticamente. Defina `DISCARD_NO_OPEN=1` para não abrir.

## Variáveis de ambiente

| Variável | Padrão | Função |
|---|---|---|
| `DISCARD_ADDR` | `127.0.0.1:8787` | endereço/porta do nó local |
| `DISCARD_DATA_DIR` | `data` (relativo ao cwd) | identidade, workspaces, blobs e sons |
| `DISCARD_OFFLINE` | desligado | `1` desabilita rede (iroh) |
| `DISCARD_NO_OPEN` | desligado | `1` não abre o navegador |
| `DISCARD_TOKEN` | gerado no boot | token de sessão da UI (`?k=` / `Authorization`) |

## Linux (omarchy / Arch)

O binário é único e autossuficiente. Uma instalação simples:

```sh
install -Dm755 target/release/discard ~/.local/bin/discard
```

Recomenda-se apontar os dados para um diretório estável (fora do cwd):

```sh
export DISCARD_DATA_DIR="$HOME/.local/share/discard"
mkdir -p "$DISCARD_DATA_DIR"
```

Atalho de desktop (`~/.local/share/applications/discard.desktop`):

```ini
[Desktop Entry]
Type=Application
Name=Discard
Exec=env DISCARD_DATA_DIR=%h/.local/share/discard %h/.local/bin/discard
Terminal=false
Categories=Network;InstantMessaging;
```

`scripts/package-linux.sh` (dono: core) gera os pacotes portáveis em `dist/` (`.tar.gz` e/ou
AppImage). Descompacte e rode o `discard`, ou execute o AppImage com permissão de execução.

## macOS

O empacotamento **precisa rodar no macOS** (não há cross-compile). O script
`scripts/package-macos.sh`:

1. garante `web/dist` e o binário release com `embed-ui`;
2. monta `dist/macos/Discard.app`:
   - `Contents/MacOS/Discard` — script launcher que define
     `DISCARD_DATA_DIR="$HOME/Library/Application Support/Discard"`, cria o diretório e faz `exec`
     no binário real (`Contents/MacOS/discard-bin`);
   - `Contents/MacOS/discard-bin` — o binário release;
   - `Contents/Info.plist` — `app.discard.desktop`, versão `0.1.0`, com as descrições de uso de
     microfone e câmera;
   - `Contents/Resources/icon.icns` — gerado de `assets/icon.png` com `sips` + `iconutil`.
3. aplica assinatura **ad-hoc** (`codesign --sign -`, best-effort);
4. gera `dist/Discard-0.1.0-macos.dmg`.

### Instalar

- **`.app`**: arraste `Discard.app` para `/Applications` (ou rode direto de `dist/macos`).
- **`.dmg`**: dê duplo clique em `dist/Discard-0.1.0-macos.dmg` e arraste o app para `Applications`.

Os dados ficam em `~/Library/Application Support/Discard` (definido pelo launcher), e **não** no
diretório do bundle.

### Gatekeeper (importante)

O app é apenas assinado em **ad-hoc**, não é notarizado pela Apple. No primeiro uso o macOS pode
bloquear com "não é possível verificar o desenvolvedor". Para abrir:

```sh
xattr -dr com.apple.quarantine /Applications/Discard.app
```

ou clique com o botão direito no app > **Abrir** > **Abrir**. Também é possível ir em
**Ajustes do Sistema > Privacidade e Segurança** e usar **Abrir Assim Mesmo**.

### Quarentena e microfone

- Arquivos baixados (DMG) recebem o atributo `com.apple.quarantine`; o comando acima o remove.
- Na primeira chamada de voz/vídeo, o macOS pede acesso a **microfone** e **câmera**. As descrições
  ficam no `Info.plist`. Se você negar, reabilite em
  **Ajustes do Sistema > Privacidade e Segurança > Microfone / Câmera**.

### Distribuir de verdade

Assinatura ad-hoc só serve para uso local. Para distribuir sem avisos de segurança é preciso:

1. certificado **Developer ID Application**;
2. `codesign --sign "Developer ID Application: ..." --options runtime Discard.app` (hardened runtime);
3. `xcrun notarytool submit` e `xcrun stapler staple Discard.app`.

## CI (GitHub Actions)

`.github/workflows/release.yml` roda em `workflow_dispatch` e em tags `v*`:

- **job `macos`** (`macos-latest`): build da UI, toolchain Rust stable, `scripts/package-macos.sh`
  e upload do artefato `Discard-*.dmg`;
- **job `linux`** (`ubuntu-latest`): build da UI, toolchain Rust stable, `scripts/package-linux.sh`
  e upload de `dist/*` (`.tar.gz`/AppImage).

Os artefatos ficam disponíveis na execução do workflow (aba **Actions**, seção **Artifacts**).
Baixe, instale conforme a plataforma acima e, no macOS, remova a quarentena antes de abrir.
