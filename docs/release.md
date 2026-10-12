# Processo de release

> Estado: **nenhuma release publicada.** Assinatura de código e atualização automática **não estão configuradas**.

## Gerar o instalador (Windows)

```bash
corepack pnpm install --frozen-lockfile
powershell -ExecutionPolicy Bypass -File scripts/release.ps1
```

O script compila a CLI `nexus` em release, copia-a como sidecar (`src-tauri/binaries/nexus-<alvo>.exe`, fora do git) e roda `tauri build --config src-tauri/tauri.release.conf.json`, que adiciona `bundle.externalBin`. A CLI fica fora da configuração base de propósito: o `tauri-build` exige o sidecar já na compilação, o que quebraria `cargo test`. No fim, o script imprime caminho, tamanho e SHA-256 do instalador.

Saída: `target/release/bundle/nsis/NEXUS_<versão>_x64-setup.exe`.

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

No CI, o job `bundle` (disparo manual com `bundle: true`) gera o mesmo instalador **não assinado** como artefato.

## Checklist de release (a cumprir)

1. Atualizar versão em `Cargo.toml` (workspace), `apps/desktop/package.json` e `tauri.conf.json`.
2. Bateria completa verde (ver [testing.md](testing.md)), incluindo E2E no Windows.
3. Revisar o [modelo de ameaças](threat-model.md) e `cargo audit` / `pnpm audit` (no CI, job `audit`).
4. Confirmar a licença do ícone (seção Marca).
5. Gerar o instalador em ambiente limpo (CI) a partir de uma tag.
6. **Assinar** o instalador e o executável — _pendente_: requer certificado de assinatura de código (OV/EV ou Azure Trusted Signing) e configuração de `bundle.windows.signCommand`/`certificateThumbprint`.
7. Publicar notas de versão com mudanças de schema (migrations) e limitações conhecidas.

## Atualização segura (pendente)

O plugin de atualização do Tauri exige um par de chaves de assinatura de updates e um endpoint de distribuição. Nada disso existe ainda; o app **não** verifica atualizações.

## Reprodutibilidade

- `Cargo.lock` e `pnpm-lock.yaml` versionados; instalações com `--locked` / `--frozen-lockfile`.
- Toolchain Rust fixada (`rust-toolchain.toml`), Node ≥ 22.12, pnpm fixado em `packageManager`.
- O pnpm aplica idade mínima de publicação para pacotes (proteção de cadeia de suprimentos); não adicionamos exceções.
