# Guia de desenvolvimento

## Pré-requisitos

| Ferramenta | Versão usada | Observação |
|---|---|---|
| Windows | 11 (10.0.26200) | alvo primário; Linux/macOS previstos |
| Visual Studio Build Tools | 2022, workload "Desktop development with C++" | necessário para compilar SQLite embutido e o Tauri |
| WebView2 Runtime | 154.x | já vem no Windows 11; o instalador do NEXUS o instala se faltar |
| Rust | 1.99.0 (fixado em `rust-toolchain.toml`) | `rustup` instala automaticamente |
| Node.js | ≥ 22 (testado com 24.19) | |
| pnpm | 12.11.2 via Corepack | `corepack enable` ou use `corepack pnpm …` |

Linux (CI): `libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev build-essential`.

## Primeira configuração

```bash
corepack pnpm install
cargo build --workspace
```

## Comandos do dia a dia

```bash
corepack pnpm dev                 # app desktop em modo desenvolvimento (Vite + Tauri)
corepack pnpm check               # tsc + eslint + prettier + vitest
cargo test --workspace            # testes Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p nexus-cli -- diagnostics
```

Use `NEXUS_DATA_DIR=<pasta>` para isolar o banco durante experimentos.

## Contratos TypeScript gerados

Os tipos em `apps/desktop/src/lib/bindings/` são gerados a partir do Rust (ADR-0004):

```bash
NEXUS_UPDATE_BINDINGS=1 cargo test -p nexus-core --test bindings   # regenera
cargo test -p nexus-core --test bindings                           # verifica (falha se desatualizado)
```

No PowerShell: `$env:NEXUS_UPDATE_BINDINGS=1; cargo test -p nexus-core --test bindings; Remove-Item Env:NEXUS_UPDATE_BINDINGS`.

## Matriz de versões (verificada em 2026-10-10)

### Rust
| Crate | Versão | Motivo |
|---|---|---|
| `tauri` / `tauri-build` | 2.12.x / 2.7.x | shell desktop. **Não** usar 3.0 alpha |
| `tauri-plugin-dialog` | 2.8.x | seletor de pasta (Fase 2) |
| `tauri-plugin-single-instance` | 2.5.x | impede dois desktops escrevendo no mesmo banco |
| `tauri-plugin-window-state` | 2.5.x | restaura tamanho/posição da janela |
| `rusqlite` (`bundled`, `backup`) | 0.40.x | SQLite embutido com FTS5; backup online |
| `blake3` | 1.8.x | checksum de migrations; hashes de conteúdo (Fase 2) |
| `serde`, `serde_json` | 1.x | serialização de contratos |
| `thiserror` | 2.x | tipos de erro |
| `tracing`, `tracing-subscriber`, `tracing-appender` | 0.1 / 0.3 / 0.2 | logs estruturados em arquivo |
| `uuid` (`v7`) | 1.x | IDs de operação ordenáveis |
| `time` | 0.3 | formatação de datas em logs/backups |
| `dirs` | 7.x | diretório de dados por plataforma |
| `ts-rs` | 12.x | geração de tipos TypeScript |
| `clap` | 4.x | CLI |
| `tempfile`, `assert_cmd` | 3.x / 2.x | testes |

Fase 2+: `ignore` 0.4, `rayon` 1.12, `notify-debouncer-full` **0.7** (0.8 ainda é RC).

### JavaScript
| Pacote | Versão | Motivo |
|---|---|---|
| `@tauri-apps/cli` / `@tauri-apps/api` | 2.12.x | |
| `react`, `react-dom` | 19.3 | |
| `vite`, `@vitejs/plugin-react` | 8.3 / 6.1 | |
| `typescript` | **6.0.x** | 7.0 ainda não é suportado pelo typescript-eslint 8 |
| `tailwindcss`, `@tailwindcss/vite`, `tw-animate-css` | 4.3 / 4.3 / 1.4 | |
| `radix-ui`, `cmdk`, `class-variance-authority`, `tailwind-merge`, `clsx` | 1.7 / 1.1 / 0.7 / 3.7 / 2.1 | base dos componentes shadcn/ui |
| `@tanstack/react-query` | 5.x | estado assíncrono de comandos |
| `zustand` | 5.x | estado de UI (rota, paleta) |
| `lucide-react` | 1.x | ícones |
| `motion` | 14.x | animações curtas de overlay |
| `vitest`, `jsdom`, `@testing-library/*` | 5 / 30 / 16 | testes de componentes |
| `@playwright/test` | 1.64 | E2E via CDP no WebView2 |
| `eslint`, `typescript-eslint`, `eslint-plugin-react-hooks`, `prettier` | 10 / 8.71 / 7 / 3 | |

Atualizações de dependências são deliberadas: atualizar lockfile, rodar toda a bateria e registrar a mudança.
