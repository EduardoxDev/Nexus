# NEXUS

**A camada de contexto local para engenharia de software assistida por IA.**

O NEXUS é um aplicativo desktop (Windows primeiro) que vai indexar os projetos que você autorizar, buscar código e documentação com **evidências verificáveis** e compilar **Context Packs** para tarefas — entregues a você, à CLI `nexus` e a agentes via **MCP**. Tudo roda localmente: nenhum código, caminho, consulta ou memória sai da sua máquina.

> **Estado atual: Fase 1 (fundação) concluída.** O app abre, migra o banco local com segurança, registra operações, mostra diagnóstico real e persiste configurações. Indexação, busca, Context Packs, memória e MCP chegam nas Fases 2–5 — veja o [roadmap](docs/roadmap.md). Seções futuras aparecem desabilitadas no app, nunca simuladas.

![Visão geral do NEXUS no tema escuro](docs/assets/visao-geral-escuro.png)

<table>
<tr>
<td><img src="docs/assets/visao-geral-claro.png" alt="Tema claro"></td>
<td><img src="docs/assets/paleta-de-comandos.png" alt="Paleta de comandos (Ctrl+K)"></td>
</tr>
</table>

_Capturas reais do executável (Fase 1), feitas pelos testes automatizados via CDP._

## O que já funciona

| Área                   | Detalhes                                                                                                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Banco local            | SQLite embutido (com FTS5), WAL, migrations versionadas com checksum, **backup automático antes de migrar**, recusa de banco criado por versão mais nova                 |
| Histórico de operações | Cada operação tem `op_id` (UUIDv7), status, duração; operações abandonadas viram "interrompidas"; retenção automática                                                    |
| Diagnóstico            | Versões (app, core, SQLite, WebView2, Tauri), caminhos e tamanhos, migrations aplicadas, backups, verificação de integridade, filtro de operações com painel de detalhes |
| Configurações          | Tema (sistema/escuro/claro) e movimento reduzido, persistidos no banco; local dos dados; declarações de privacidade verificáveis                                         |
| Interface              | Sidebar, breadcrumbs, barra de status com dados reais, paleta de comandos (Ctrl+K), atalhos de teclado (F1), histórico voltar/avançar, temas com contraste AA testado    |
| CLI                    | `nexus init` e `nexus diagnostics [--check] [--json]` sobre o mesmo banco do app                                                                                         |
| Segurança              | Webview sem acesso a fs/shell/http; comandos em allowlist explícita (testada); CSP sem origens remotas; instância única                                                  |

## Instalação

Ainda não há release publicada. Para gerar o instalador localmente (Windows):

```bash
corepack pnpm install
corepack pnpm --filter @nexus/desktop tauri build
```

O instalador NSIS fica em `target/release/bundle/nsis/`. Ele instala por usuário (sem admin) e baixa o WebView2 se faltar. **O binário não é assinado** — o SmartScreen pode alertar. Veja [release.md](docs/release.md).

## Desenvolvimento

Pré-requisitos: Windows 11, Visual Studio Build Tools 2022 (C++), Rust (instalado via `rustup`; a versão é fixada em `rust-toolchain.toml`), Node.js ≥ 22.12. O pnpm vem pelo Corepack.

```bash
corepack pnpm install
corepack pnpm dev                  # app em modo desenvolvimento
cargo test --workspace             # testes Rust
corepack pnpm check                # tipos, lint, formatação e testes do frontend
```

Guia completo: [docs/development.md](docs/development.md) · Testes: [docs/testing.md](docs/testing.md).

## Documentação

- [Arquitetura](docs/architecture.md) e [decisões (ADRs)](docs/decisions/)
- [Roadmap](docs/roadmap.md) — fases, critérios de aceitação e riscos
- [Schema do banco](docs/database-schema.md) — migrations, retenção, backup e restauração
- [Modelo de ameaças](docs/threat-model.md) e [privacidade](docs/privacy.md)
- [CLI](docs/cli.md) · [Testes](docs/testing.md) · [Solução de problemas](docs/troubleshooting.md) · [Release](docs/release.md)

## Princípios

Local-first · privacy-first · evidence-first · incremental-first · agnóstico de agentes · seguro por padrão · consciente de recursos · reversível · mensurável · **UI honesta**: nada de métricas, progresso ou resultados inventados.

## Licença

[MIT](LICENSE).
