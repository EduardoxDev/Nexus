# Arquitetura do NEXUS

> Estado: **Fase 1 (fundação)**. Seções marcadas com *(planejado)* descrevem o desenho aprovado para fases futuras e ainda não existem no código.

## 1. Visão

O NEXUS é a camada de contexto local para engenharia de software assistida por IA. Ele indexa repositórios que o usuário autoriza explicitamente, permite buscar código e documentação com evidências verificáveis, compila *Context Packs* para tarefas, mantém memória técnica por projeto, detecta contexto desatualizado e expõe tudo por uma CLI (`nexus`) e por um servidor MCP local.

Princípios que orientam todas as decisões: local-first, privacy-first, evidence-first, incremental-first, agente-agnóstico, segurança por desenho, consciência de recursos, operações reversíveis, mensurabilidade, manutenibilidade e **UI honesta** (nada de métricas ou estados inventados).

## 2. Processos

```
┌──────────── Desktop (Tauri 2, instância única) ───────┐   ┌── nexus (CLI) ─┐  ┌─ nexus mcp serve ─┐
│ WebView2: React UI ──invoke/events──▶ commands/ (fino)│   │  clap          │  │ JSON-RPC stdio    │
│                                       │               │   └──────┬─────────┘  └────────┬──────────┘
│                               nexus-core (lib) ◀──────┼──────────┴─────────────────────┘
└───────────────────────────────────────┼───────────────┘
                                        ▼
              %LOCALAPPDATA%\Nexus\   (sobrescrevível por NEXUS_DATA_DIR)
                nexus.db (SQLite, WAL) · logs\ · backups\
```

- **Um único core** (`crates/nexus-core`) concentra a lógica de negócio. Desktop, CLI e MCP são adaptadores finos: validam entrada, chamam o core e formatam saída.
- **Banco compartilhado**: os três processos usam o mesmo arquivo SQLite em modo WAL com `busy_timeout` e transações curtas (ADR-0002).
- O frontend **nunca** acessa o sistema de arquivos diretamente; só chama comandos explicitamente registrados e listados nas *capabilities* do Tauri.

## 3. Módulos lógicos

| # | Módulo | Onde vive | Estado |
|---|---|---|---|
| 1 | Desktop Shell | `apps/desktop/src-tauri` | Fase 1 |
| 2 | Application Commands | `apps/desktop/src-tauri/src/commands` | Fase 1 (app, diagnóstico, configurações) |
| 3 | Project Registry | `nexus-core::projects` | *(planejado — Fase 2)* |
| 4 | Filesystem Scanner | `nexus-core::scan` | *(planejado — Fase 2)* |
| 5 | Incremental Indexer | `nexus-core::index` | *(planejado — Fase 2)* |
| 6 | Language/Symbol Analyzer | `nexus-core::analysis` | *(planejado — Fases 2/6)* |
| 7 | Search Engine | `nexus-core::search` | *(planejado — Fase 3)* |
| 8 | Context Retrieval Engine | `nexus-core::context` | *(planejado — Fase 3)* |
| 9 | Project Memory | `nexus-core::memory` | *(planejado — Fase 4)* |
| 10 | Freshness & Conflict Detection | `nexus-core::freshness` | *(planejado — Fase 4)* |
| 11 | Context Pack Compiler | `nexus-core::context` | *(planejado — Fase 3)* |
| 12 | MCP Server | `crates/nexus-mcp` | *(planejado — Fase 5)* |
| 13 | CLI | `crates/nexus-cli` | Fase 1 (`init`, `diagnostics`) |
| 14 | Storage & Migrations | `nexus-core::storage` | Fase 1 |
| 15 | Telemetry & Diagnostics (local) | `nexus-core::{ops,diagnostics,logging}` | Fase 1 |
| 16 | Security & Permissions | capabilities Tauri, `nexus-core::settings` (allowlist), validação | Fase 1 (linha de base) |

## 4. Contrato UI ↔ core

- **Tipos únicos**: DTOs são definidos em Rust (`nexus-core::api` e módulos do core) com `serde` + `ts-rs`; os arquivos TypeScript em `apps/desktop/src/lib/bindings/` são **gerados** e um teste falha se estiverem desatualizados (ADR-0004).
- **Erros**: todo comando retorna `Result<T, CommandError>`; `CommandError { code, message, opId }`, com `code ∈ {invalid_input, not_found, storage, io, conflict, cancelled, incompatible_schema, internal}`. A mensagem é em pt-BR e voltada ao usuário; detalhes técnicos vão apenas para o log local.
- **Operações longas** (Fase 2+) emitem eventos tipados: `nexus://job-progress`, `nexus://job-finished`, `nexus://index-changed`.
- **Bloqueio**: comandos que tocam o banco rodam em `spawn_blocking`; nenhuma conexão SQLite fica presa durante IO de filesystem.

## 5. Persistência

Detalhes em [`database-schema.md`](database-schema.md).

- SQLite embutido (`rusqlite`, feature `bundled`), `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`.
- Migrations versionadas e embutidas no binário, uma transação por migration, checksum registrado (ADR-0007).
- Backup automático (SQLite Online Backup API) antes de aplicar migrations pendentes num banco existente.
- Banco criado por versão mais nova do NEXUS ⇒ recusa explícita (`incompatible_schema`), nunca downgrade silencioso.
- O código-fonte indexado **não** será copiado para o banco (ADR-0003): o FTS5 será *contentless* e os trechos serão lidos do disco e verificados por hash.

## 6. Observabilidade local

- `tracing` com saída JSON Lines diária em `logs/` (retenção de 7 arquivos) e console em desenvolvimento.
- Toda operação relevante recebe um `op_id` (UUIDv7) registrado na tabela `operations` com início, fim, duração, status e código de erro. O mesmo `op_id` aparece no log e no erro devolvido à UI.
- Nunca registrar conteúdo de arquivos, valores de configuração sensíveis ou segredos.
- **Sem telemetria externa.**

## 7. Segurança

Detalhes em [`threat-model.md`](threat-model.md). Linha de base da Fase 1:

- Capabilities do Tauri mínimas; sem plugins `fs`, `shell` ou `http` expostos ao webview; `withGlobalTauri: false`.
- CSP estrita sem origens remotas.
- Instância única do desktop.
- Configurações só aceitam chaves de uma allowlist com validação de valores.
- `unsafe_code = "forbid"` no workspace; Clippy com `-D warnings` no CI.

## 8. Frontend

- React + TypeScript strict + Vite + Tailwind CSS 4 + componentes no padrão shadcn/ui (Radix).
- Estado assíncrono via TanStack Query; Zustand apenas para estado de interface (rota, histórico, paleta).
- Navegação por router tipado mínimo (união discriminada de rotas) — o app desktop não usa URLs.
- Textos em pt-BR centralizados em `src/lib/i18n/pt-BR.ts` (ADR-0008).
- Design tokens em `src/styles/tokens.css`: tema escuro grafite padrão, tema claro completo, acento violeta usado com moderação.

## 9. Pipeline de contexto *(planejado)*

```
pasta autorizada ─▶ scanner (ignore, exclusões, sensíveis, sem symlinks)
                ─▶ indexador incremental (stat → hash → chunks → FTS5 contentless → símbolos/imports)
                ─▶ busca híbrida (bm25 + caminho + símbolo + intenção + recência + grafo) com motivos
                ─▶ compilador de Context Pack (expansão por dependências, testes, memória, atualidade, orçamento)
                ─▶ desktop · CLI · MCP · exportação Markdown/JSON
```

Cada resultado carregará caminho, intervalo de linhas, hash do conteúdo indexado e estado de atualidade (atual / modificado desde a indexação / ausente).
