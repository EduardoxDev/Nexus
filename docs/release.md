# Processo de release

> Estado: **nenhuma release publicada.** Assinatura de código e atualização automática **não estão configuradas**.

## Gerar o instalador (Windows)

```bash
corepack pnpm install --frozen-lockfile
corepack pnpm --filter @nexus/desktop tauri build
```

Saída: `target/release/bundle/nsis/NEXUS_<versão>_x64-setup.exe`.

Características do instalador (configuradas em `apps/desktop/src-tauri/tauri.conf.json`):

- instalação por usuário (`currentUser`), sem privilégios de administrador;
- idiomas pt-BR e inglês;
- WebView2: baixado via bootstrapper silencioso se não existir;
- ícone próprio gerado de `apps/desktop/assets/icon.svg` (`pnpm tauri icon assets/icon.svg -o src-tauri/icons`).

Verificação mais recente (Fase 2, build de depuração, 4,60 MiB): instalação silenciosa (`/S /D=<pasta>`), suíte E2E completa contra o executável instalado (7/7, incluindo indexação e monitor de arquivos) e desinstalação silenciosa sem resíduos do instalador.

No CI, o job `bundle` (disparo manual com `bundle: true`) gera o mesmo instalador **não assinado** como artefato.

## Checklist de release (a cumprir)

1. Atualizar versão em `Cargo.toml` (workspace), `apps/desktop/package.json` e `tauri.conf.json`.
2. Bateria completa verde (ver [testing.md](testing.md)), incluindo E2E no Windows.
3. Revisar o [modelo de ameaças](threat-model.md) e `cargo audit` / `pnpm audit`.
4. Gerar o instalador em ambiente limpo (CI) a partir de uma tag.
5. **Assinar** o instalador e o executável — _pendente_: requer certificado de assinatura de código (OV/EV ou Azure Trusted Signing) e configuração de `bundle.windows.signCommand`/`certificateThumbprint`.
6. Publicar notas de versão com mudanças de schema (migrations) e limitações conhecidas.

## Atualização segura (pendente)

O plugin de atualização do Tauri exige um par de chaves de assinatura de updates e um endpoint de distribuição. Nada disso existe ainda; o app **não** verifica atualizações.

## Reprodutibilidade

- `Cargo.lock` e `pnpm-lock.yaml` versionados; instalações com `--locked` / `--frozen-lockfile`.
- Toolchain Rust fixada (`rust-toolchain.toml`), Node ≥ 22.12, pnpm fixado em `packageManager`.
- O pnpm aplica idade mínima de publicação para pacotes (proteção de cadeia de suprimentos); não adicionamos exceções.
