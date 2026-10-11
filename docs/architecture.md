# Arquitetura do NEXUS

> Estado: **Fase 2 (indexação) concluída**. Seções marcadas com _(planejado)_ descrevem o desenho aprovado para fases futuras e ainda não existem no código.

## 1. Visão

O NEXUS é a camada de contexto local para engenharia de software assistida por IA. Ele indexa repositórios que o usuário autoriza explicitamente, permite buscar código e documentação com evidências verificáveis, compila _Context Packs_ para tarefas, mantém memória técnica por projeto, detecta contexto desatualizado e expõe tudo por uma CLI (`nexus`) e por um servidor MCP local.

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
- O frontend **nunca** acessa o sistema de arquivos diretamente; só chama comandos explicitamente registrados e listados nas _capabilities_ do Tauri.

## 3. Módulos lógicos

| #   | Módulo                          | Onde vive                                                                                         | Estado                                                            |
| --- | ------------------------------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| 1   | Desktop Shell                   | `apps/desktop/src-tauri`                                                                          | Fase 1                                                            |
| 2   | Application Commands            | `apps/desktop/src-tauri/src/commands`                                                             | Fases 1–2 (app, diagnóstico, configurações, projetos e indexação) |
| 3   | Project Registry                | `nexus-core::projects`                                                                            | Fase 2                                                            |
| 4   | Filesystem Scanner              | `nexus-core::{scan,sensitive}`, watcher em `src-tauri/src/watch.rs`                               | Fase 2                                                            |
| 5   | Incremental Indexer             | `nexus-core::{index,chunk}`, fila em `src-tauri/src/indexing.rs`                                  | Fase 2                                                            |
| 6   | Language/Symbol Analyzer        | `nexus-core::classify` (linguagem/tipo por caminho); símbolos e imports _(planejado — Fases 3/6)_ | parcial                                                           |
| 7   | Search Engine                   | `nexus-core::search`                                                                              | _(planejado — Fase 3)_                                            |
| 8   | Context Retrieval Engine        | `nexus-core::context`                                                                             | _(planejado — Fase 3)_                                            |
| 9   | Project Memory                  | `nexus-core::memory`                                                                              | _(planejado — Fase 4)_                                            |
| 10  | Freshness & Conflict Detection  | `nexus-core::freshness`                                                                           | _(planejado — Fase 4)_                                            |
| 11  | Context Pack Compiler           | `nexus-core::context`                                                                             | _(planejado — Fase 3)_                                            |
| 12  | MCP Server                      | `crates/nexus-mcp`                                                                                | _(planejado — Fase 5)_                                            |
| 13  | CLI                             | `crates/nexus-cli`                                                                                | Fases 1–2 (`init`, `diagnostics`, `projects`, `index`, `status`)  |
| 14  | Storage & Migrations            | `nexus-core::storage`                                                                             | Fase 1                                                            |
| 15  | Telemetry & Diagnostics (local) | `nexus-core::{ops,diagnostics,logging}`                                                           | Fase 1                                                            |
| 16  | Security & Permissions          | capabilities Tauri, `nexus-core::settings` (allowlist), validação                                 | Fase 1 (linha de base)                                            |

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
- O código-fonte indexado **não** será copiado para o banco (ADR-0003): o FTS5 será _contentless_ e os trechos serão lidos do disco e verificados por hash.

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

## 9. Indexação (Fase 2)

Detalhes e justificativas em [ADR-0009](decisions/ADR-0009-indexacao-incremental.md); números em [benchmarks.md](benchmarks.md).

1. **Registro**: a pasta escolhida é canonicalizada uma vez; raiz de disco, pasta pessoal (ou acima dela), pastas do sistema, a pasta de dados do NEXUS e sobreposições com outros projetos são recusadas.
2. **Varredura** (`scan`): `ignore::WalkBuilder` com `.gitignore`, `.ignore` e `.nexusignore` (mesmo sem `.git`), sem ler `.gitignore` acima da raiz, profundidade máxima 64, diretórios padrão excluídos (`.git`, `node_modules`, `target`, `dist`, `build`, caches…). Links simbólicos e junctions/reparse points **não são seguidos**.
3. **Comparação**: tamanho + mtime (ns) + versão do indexador ⇒ inalterado.
4. **Processamento** (paralelo, fora de transações): nome sensível → ignorado; extensão binária → ignorado; acima do limite → ignorado; leitura estável (re-stat); byte nulo → binário; UTF-8 (com tolerância de 1% de substituições) senão codificação; conteúdo sensível (chaves privadas, tokens conhecidos) → ignorado; senão hash blake3, chunks de 60 linhas/8 KiB e texto do FTS com subpalavras de identificadores.
5. **Gravação**: lotes de 64 arquivos, uma transação curta cada; conteúdo novo gera chunks + FTS, conteúdo já conhecido é reaproveitado.
6. **Finalização**: remoções, detecção de renomeações por hash, coleta de conteúdo órfão, nova `index_generation` se algo mudou, HEAD do git, checkpoint passivo do WAL.
7. **Desktop**: fila com um job por vez (pedidos do usuário à frente), pausa/retomada/cancelamento, eventos `nexus://job-progress|job-finished|queue-changed`; monitor de arquivos com debounce de 1,5 s + reconciliação ao abrir e a cada 10 minutos.

### Cobertura dos analisadores (honesta)

| Recurso                                                           | Como                                         | Limitação                                                        |
| ----------------------------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------- |
| Linguagem                                                         | extensão / nome do arquivo (~60 linguagens)  | não lê shebang nem conteúdo                                      |
| Tipo (código, teste, doc, config, manifesto, lockfile, CI, dados) | convenções de caminho e nome                 | heurística; pastas fora do padrão podem ser mal classificadas    |
| Busca textual                                                     | FTS5 + subpalavras de identificadores        | consulta ainda não exposta na interface (Fase 3)                 |
| Símbolos, imports, grafo                                          | —                                            | _(planejado — Fases 3/6)_                                        |
| Git                                                               | leitura de `.git/HEAD`, refs e `packed-refs` | não lê histórico de commits                                      |
| "Mais alterados"                                                  | mudanças de hash observadas pelo NEXUS       | não usa `git log`; começa a contar quando o projeto é adicionado |

## 10. Pipeline de contexto _(planejado)_

```
pasta autorizada ─▶ scanner (ignore, exclusões, sensíveis, sem symlinks)
                ─▶ indexador incremental (stat → hash → chunks → FTS5 contentless → símbolos/imports)
                ─▶ busca híbrida (bm25 + caminho + símbolo + intenção + recência + grafo) com motivos
                ─▶ compilador de Context Pack (expansão por dependências, testes, memória, atualidade, orçamento)
                ─▶ desktop · CLI · MCP · exportação Markdown/JSON
```

Cada resultado carregará caminho, intervalo de linhas, hash do conteúdo indexado e estado de atualidade (atual / modificado desde a indexação / ausente).
