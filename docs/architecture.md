# Arquitetura do NEXUS

> Estado: **Fase 3 (busca e Context Packs) concluída**. Seções marcadas com _(planejado)_ descrevem o desenho aprovado para fases futuras e ainda não existem no código.

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
| 7   | Search Engine                   | `nexus-core::search` (ranking híbrido, análise de consulta, evidência verificada)                 | Fase 3                                                            |
| 8   | Context Retrieval Engine        | `nexus-core::context` (seleção por papel, testes correspondentes, manifestos)                     | Fase 3                                                            |
| 9   | Project Memory                  | `nexus-core::memory`                                                                              | _(planejado — Fase 4)_                                            |
| 10  | Freshness & Conflict Detection  | `nexus-core::freshness`                                                                           | _(planejado — Fase 4)_                                            |
| 11  | Context Pack Compiler           | `nexus-core::context` + `context::export` (Markdown/JSON)                                         | Fase 3                                                            |
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

## 10. Pipeline de contexto

```
pasta autorizada ─▶ scanner ─▶ indexador incremental ─▶ busca híbrida (motivos) ─▶ Context Pack (referências)
                                                                   │                           │
                                                          desktop · CLI · (MCP — Fase 5)  exportação Markdown/JSON
```

## 11. Busca e Context Packs (Fase 3)

Decisões em [ADR-0010](decisions/ADR-0010-busca-explicavel-e-packs-por-referencia.md); qualidade medida em [evaluation.md](evaluation.md).

**Análise da consulta** (`search::query`): normalização (minúsculas, sem acentos — igual ao tokenizador FTS), stopwords pt/en, dicionário PT→EN de termos técnicos (~100 entradas) com tratamento leve de flexões (`validada` → `validar`), subpalavras de identificadores (`validateSession` → validate, session), detecção de intenção (testes, documentação/decisões, configuração, CI) e de caminhos citados.

**Ranking híbrido** (`search`), por arquivo:

| Componente                                             | Peso          | Observação                                                           |
| ------------------------------------------------------ | ------------- | -------------------------------------------------------------------- |
| Conteúdo (bm25 do melhor chunk, normalizado)           | 0,40          | FTS5 com OR dos termos e prefixo para termos ≥ 4 letras              |
| Caminho (nome do arquivo 1,0; pastas 0,5 por conceito) | 0,25          | arquivos só com correspondência de caminho também entram             |
| Cobertura de conceitos no arquivo                      | 0,20          | lido do disco para os candidatos de topo                             |
| Declaração do termo (função, classe, const…)           | 0,10          | casa por palavras do identificador, não por substring                |
| Intenção por tipo                                      | −0,30 a +0,25 | lockfile penalizado; testes/docs/config promovidos só quando pedidos |
| Recência                                               | até 0,03      | mtime nos últimos 7/30 dias                                          |
| Caminho citado na consulta                             | 0,50          |                                                                      |

Resultados são deduplicados por hash ("também em"), cada um com trecho de até 14 linhas ancorado na linha de correspondência mais forte, linhas destacadas, estado de atualidade (`fresh`/`modified`/`missing`) e avisos (termos sem correspondência, arquivos alterados). O modo `plain` (só bm25 com os termos digitados) existe como referência para a avaliação.

**Prévia** (`search::evidence::file_context`): até 400 linhas de um arquivo **indexado**; arquivos ignorados (sensíveis, binários) retornam "não encontrado".

**Context Pack** (`context`):

1. Busca híbrida pela tarefa + busca dedicada de documentação.
2. Papéis: principal (≥ 50% da melhor pontuação de código), relacionado, teste, decisão (ADR), documentação (no máximo 2 genéricos), convenção (guia de contribuição, só quando há evidência), dependências (manifesto mais próximo dos principais), configuração.
3. Testes correspondentes por nome (`payment.ts` ↔ `payment.test.ts`, `service.py` ↔ `test_service.py`).
4. Ordem por papel, promovendo o que a tarefa pede (decisões, testes, configuração); orçamento de tokens estimado (caracteres ÷ 4) e limite de arquivos; o que não cabe vai para "omitidos".
5. Riscos (fontes alteradas, ausência de testes, sensíveis excluídos nas pastas relevantes, índice antigo) e limitações sempre presentes.
6. Persistido como plano (sem texto); materializado ao abrir com verificação de hash; retenção de 100 packs por projeto.
7. Exportação Markdown (cercas seguras + aviso de conteúdo não confiável + tabela de fontes) e JSON (`nexus.context-pack`, `schemaVersion` 1), com exclusões e notas do usuário.
