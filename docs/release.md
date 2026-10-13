# Processo de release

> Estado: **v0.1.0** com tag e rascunho no GitHub (só Windows). A partir da v0.2, pacotes para Windows, Linux e macOS. Assinatura de código e atualização automática **não estão configuradas**.

## Gerar os pacotes

```bash
corepack pnpm install --frozen-lockfile
# Windows (PowerShell 5.1)
powershell -ExecutionPolicy Bypass -File scripts/release.ps1
# Linux / macOS (PowerShell 7)
pwsh scripts/release.ps1
# Outro alvo, ex.: Intel num Mac com Apple Silicon
pwsh scripts/release.ps1 -Target x86_64-apple-darwin
```

O script compila a CLI `nexus` em release, copia-a como sidecar (`src-tauri/binaries/nexus-<alvo>[.exe]`, fora do git), confere a CLI com `scripts/smoke-cli.mjs` (`--version` e handshake MCP real, quando o alvo roda na máquina) e roda `tauri build --config src-tauri/tauri.release.conf.json`, que adiciona `bundle.externalBin`. A CLI fica fora da configuração base de propósito: o `tauri-build` exige o sidecar já na compilação, o que quebraria `cargo test`. No fim, o script imprime caminho, tamanho e SHA-256 de cada pacote.

| Sistema              | Formatos              | Saída (`target/[<alvo>/]release/bundle/`) | CLI instalada em                  |
| -------------------- | --------------------- | ----------------------------------------- | --------------------------------- |
| Windows x64          | NSIS                  | `nsis/NEXUS_<versão>_x64-setup.exe`       | ao lado de `nexus-desktop.exe`    |
| Linux x64            | `.deb`, `.AppImage`   | `deb/`, `appimage/`                       | `/usr/bin/nexus` (no `.deb`)      |
| macOS (arm64 e x64)  | `.app`, `.dmg`        | `macos/`, `dmg/`                          | `NEXUS.app/Contents/MacOS/nexus`  |

### Abrir sem assinatura

- **Windows**: o SmartScreen mostra "O Windows protegeu o computador" → "Mais informações" → "Executar assim mesmo".
- **macOS**: o app tem só assinatura ad-hoc (`signingIdentity: "-"`, exigida no Apple Silicon). Baixado da internet, o Gatekeeper o bloqueia ("está danificado" ou "não pode ser aberto"). Depois de arrastar para Aplicativos: `xattr -dr com.apple.quarantine /Applications/NEXUS.app`, ou clique com o botão direito → Abrir.
- **Linux**: `.deb` com `sudo apt install ./nexus_<versão>_amd64.deb`; `.AppImage` com `chmod +x` e executar (requer `libfuse2` em distribuições recentes).

A interface no Linux e no macOS é validada pelo build e pela fumaça da CLI/MCP no CI; o teste manual da janela depende de quem tiver o sistema.

Características do instalador (configuradas em `apps/desktop/src-tauri/tauri.conf.json`):

- instalação por usuário (`currentUser`), sem privilégios de administrador;
- idiomas pt-BR e inglês;
- WebView2: baixado via bootstrapper silencioso se não existir;
- `nexus.exe` instalado ao lado de `nexus-desktop.exe` (a tela Integrações o encontra ali) e removido na desinstalação;
- ícone da marca gerado de `apps/desktop/assets/brand/nexus-icon-1024.png` (`pnpm tauri icon assets/brand/nexus-icon-1024.png -o src-tauri/icons`; as pastas `android/` e `ios/` geradas são descartadas).

Verificação mais recente (Fase 7, release, 6,79 MiB): instalação silenciosa (`/S /D=<pasta>`) com `nexus-desktop.exe`, `nexus.exe` (respondendo `nexus 0.1.0`) e `uninstall.exe`; desinstalação silenciosa sem resíduos. A suíte E2E roda contra o build de depuração (o build de release não abre a porta CDP); a Fase 3 registrou E2E contra um instalador de depuração instalado.

## Marca

O ícone e o banner vieram do usuário em 2026-10-12. O ícone foi extraído de uma imagem de apresentação de 263 px e ampliado para 1024 px, por isso fica levemente suave em tamanhos grandes; uma fonte vetorial ou de alta resolução é desejável.

> **Pendência de licença:** a imagem de origem do ícone tem a assinatura de um designer ("design30club", t.me/beslanalaudinov). Antes de distribuir ou publicar o instalador, confirme que há direito de uso (licença ou autorização do autor) ou substitua o ícone.

No CI, o job `bundle` (disparo manual com `bundle: true`) gera o instalador Windows **não assinado** como artefato. Uma tag `v*` dispara `.github/workflows/release.yml`: em matriz (Windows x64, Linux x64, macOS arm64 e x64), testes, pacotes com a CLI e fumaça; depois um job junta tudo com `SHA256SUMS.txt` num **rascunho** de release (nada é publicado sem revisão).

## Checklist de release (a cumprir)

1. Atualizar versão em `Cargo.toml` (workspace), `apps/desktop/package.json` e `tauri.conf.json`.
2. Bateria completa verde (ver [testing.md](testing.md)), incluindo E2E no Windows.
3. Revisar o [modelo de ameaças](threat-model.md) e `cargo audit` / `pnpm audit` (no CI, job `audit`).
4. Confirmar a licença do ícone (seção Marca).
   Se o ícone for trocado, regenere os tamanhos com `pnpm tauri icon assets/brand/nexus-icon-1024.png -o src-tauri/icons` (apague `android/` e `ios/`).
5. Gerar o instalador em ambiente limpo (CI) a partir de uma tag.
6. **Assinar** o instalador e o executável — _pendente_: requer certificado de assinatura de código (OV/EV ou Azure Trusted Signing) e configuração de `bundle.windows.signCommand`/`certificateThumbprint`.
7. Publicar notas de versão com mudanças de schema (migrations) e limitações conhecidas.

## Atualização automática (v0.2)

- **Plugin** `tauri-plugin-updater` 2.x, usado só pelo Rust (`commands/updates.rs`); o webview não recebe permissões do plugin (teste em `tests/capabilities.rs`).
- **Endpoint**: `https://github.com/EduardoxDev/Nexus/releases/latest/download/latest.json`. Rascunhos e pre-releases não contam como "latest": a atualização só chega depois que a release é **publicada** como normal.
- **Assinatura**: par minisign gerado com `tauri signer generate`. A chave pública está em `tauri.conf.json` (`plugins.updater.pubkey`); a privada e a senha ficam só nos segredos do GitHub (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) e com quem mantém o projeto. **Perder a chave privada impede atualizar quem já instalou** (seria preciso reinstalar à mão).
- **Pacotes**: com a chave no ambiente, `scripts/release.ps1` acrescenta `tauri.updater.conf.json` (`createUpdaterArtifacts`) e o build gera `.sig` para o instalador NSIS, o `.AppImage` e o `.app.tar.gz` do macOS. O job final roda `scripts/latest-json.mjs`, que monta o `latest.json` (falha se faltar assinatura). O `.deb` não se atualiza sozinho.
- **Política** (`nexus_core::updates`): pergunta na 1ª abertura; automática só com permissão e no máximo 1×/24 h; manual sempre que a pessoa pedir; instalação só por clique, com reinício.
- **Primeiro teste real**: a v0.1.0 não tem o atualizador; o primeiro update automático possível é de uma versão ≥ 0.2.0-rc.1 para a seguinte.

## Reprodutibilidade

- `Cargo.lock` e `pnpm-lock.yaml` versionados; instalações com `--locked` / `--frozen-lockfile`.
- Toolchain Rust fixada (`rust-toolchain.toml`), Node ≥ 22.12, pnpm fixado em `packageManager`.
- O pnpm aplica idade mínima de publicação para pacotes (proteção de cadeia de suprimentos); não adicionamos exceções.
