# Schema do banco

Arquivo: `nexus.db` no diretório de dados (`%LOCALAPPDATA%\Nexus` no Windows, `$XDG_DATA_HOME/Nexus` no Linux, `~/Library/Application Support/Nexus` no macOS; `NEXUS_DATA_DIR` sobrescreve).

Pragmas aplicados em toda conexão: `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`.

## Migrations

| Versão | Nome               | Fase | Tabelas                                                                                              |
| ------ | ------------------ | ---- | ---------------------------------------------------------------------------------------------------- |
| 1      | `foundation`       | 1    | `settings`, `operations`                                                                             |
| 2      | `projects_index`   | 2    | `projects`, `contents`, `files`, `chunks`, `chunks_fts`, `indexing_jobs`, `index_errors`             |
| 3      | `context_packs`    | 3    | `context_packs`, `context_sources`                                                                   |
| 4      | `memory_freshness` | 4    | `memories`, `memory_refs`, `memory_revisions`, `memories_fts`, `freshness_issues`, `freshness_scans` |
| 5      | `analysis`         | 6    | `symbols`, `imports`, `edges`, `graph_state`                                                         |

Regras (ADR-0007):

- Migrations são SQL embutido no binário (`crates/nexus-core/src/storage/migrations/`), aplicadas em ordem, **uma transação por migration**.
- `schema_migrations` registra `version`, `name`, `checksum` (blake3 do SQL), `applied_at`, `duration_ms`.
- Se o SQL de uma migration já aplicada mudar, a abertura falha com `storage` + mensagem explícita (o binário não corresponde ao histórico do banco).
- Se o banco tiver versão maior do que o binário conhece, a abertura falha com `incompatible_schema`.
- Antes de aplicar migrations pendentes num banco que já tem dados, é criado um backup em `backups/nexus-v{versão_atual}-{timestamp}.db` via Online Backup API. São mantidos os 5 mais recentes.

## Versão 1 — `foundation`

### `schema_migrations`

| Coluna        | Tipo             | Notas             |
| ------------- | ---------------- | ----------------- |
| `version`     | INTEGER PK       |                   |
| `name`        | TEXT NOT NULL    |                   |
| `checksum`    | TEXT NOT NULL    | blake3 hex do SQL |
| `applied_at`  | INTEGER NOT NULL | Unix ms           |
| `duration_ms` | INTEGER NOT NULL |                   |

### `settings`

| Coluna       | Tipo             | Notas                                                |
| ------------ | ---------------- | ---------------------------------------------------- |
| `key`        | TEXT PK          | somente chaves da allowlist (`nexus-core::settings`) |
| `value_json` | TEXT NOT NULL    | JSON validado por chave                              |
| `updated_at` | INTEGER NOT NULL | Unix ms                                              |

Chaves atuais: `ui.theme` (`"system" \| "dark" \| "light"`), `ui.reduced_motion` (`"system" \| "on" \| "off"`), `ui.sidebar_collapsed` (bool), `indexing.threads` (0–16; 0 = automático), `indexing.max_file_size_kb` (16–10240; padrão 1024), `indexing.watch` (bool; padrão `true`).

### `operations`

Histórico local de operações (diagnóstico). Nunca contém conteúdo de arquivos.

| Coluna          | Tipo             | Notas                                                                              |
| --------------- | ---------------- | ---------------------------------------------------------------------------------- |
| `id`            | TEXT PK          | UUIDv7 (`op_id`)                                                                   |
| `kind`          | TEXT NOT NULL    | ex.: `app.startup`, `storage.migrate`, `storage.integrity_check`, `storage.backup` |
| `project_id`    | INTEGER NULL     | sem FK até a migration 2                                                           |
| `status`        | TEXT NOT NULL    | `running` \| `succeeded` \| `failed` \| `cancelled` \| `interrupted` (CHECK)       |
| `started_at`    | INTEGER NOT NULL | Unix ms                                                                            |
| `heartbeat_at`  | INTEGER NOT NULL | atualizado periodicamente por operações longas                                     |
| `finished_at`   | INTEGER NULL     |                                                                                    |
| `duration_ms`   | INTEGER NULL     |                                                                                    |
| `detail_json`   | TEXT NULL        | metadados estruturados (contagens, versões)                                        |
| `error_code`    | TEXT NULL        |                                                                                    |
| `error_message` | TEXT NULL        | mensagem sem conteúdo sensível                                                     |

Índices: `operations(started_at DESC)`, `operations(kind, started_at DESC)`.

Recuperação: na abertura do banco, operações `running` sem heartbeat há mais de 10 minutos são marcadas `interrupted` (o processo que as executava terminou sem finalizá-las).

## Versão 2 — `projects_index`

O código **não é copiado** para o banco (ADR-0003). O que é guardado:

| Tabela          | Conteúdo                                                                                                                                                                                                                         |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `projects`      | pasta autorizada (caminho canônico, único), nome, datas, `index_generation`, `head_ref`/`head_commit` lidos de `.git`                                                                                                            |
| `contents`      | um registro por conteúdo único (blake3) por projeto: linhas, nº de chunks, versão do indexador. Arquivos idênticos compartilham o mesmo registro (deduplicação)                                                                  |
| `files`         | caminho relativo (`/`), tamanho, mtime (ns), hash, linguagem, tipo, status (`indexed`, `skipped_sensitive`, `skipped_binary`, `skipped_large`, `skipped_encoding`, `error`), motivo, linhas, `change_count`, versão do indexador |
| `chunks`        | blocos de até 60 linhas / 8 KiB de cada conteúdo: intervalo de linhas e hash do bloco                                                                                                                                            |
| `chunks_fts`    | FTS5 **contentless** (`content='`, `contentless_delete=1`, `unicode61 remove_diacritics 2`); `rowid = chunks.id`. Guarda o índice invertido de termos (texto do bloco + subpalavras de identificadores), não o texto             |
| `indexing_jobs` | um registro por execução (mesmo ID da operação `index.run`): modo, origem, status, fase, PID, heartbeat e contadores                                                                                                             |
| `index_errors`  | erros por arquivo de cada job (código e mensagem; nunca conteúdo)                                                                                                                                                                |

Integridade e concorrência:

- `files → contents` usa FK composta **adiada** (a linha de conteúdo é inserida na mesma transação). Arquivos ignorados têm `content_hash` nulo e nenhum conteúdo.
- `chunks → contents` com `ON DELETE CASCADE`; o gatilho `chunks_after_delete` remove a linha correspondente do FTS.
- Índice único parcial `indexing_jobs_one_active` (`status IN (running, paused)`) é a trava entre processos: no máximo um job ativo por projeto. Jobs sem heartbeat há mais de 2 minutos são marcados `interrupted` antes de uma nova tentativa e na abertura do core.
- Arquivos sensíveis nunca têm hash nem conteúdo no banco — apenas caminho, tamanho e motivo.

## Versão 3 — `context_packs`

| Tabela            | Conteúdo                                                                                                                                                        |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `context_packs`   | um registro por pack: tarefa, data, geração do índice, commit, orçamento, tokens estimados, nº de fontes e `plan_json` (plano completo **sem texto de código**) |
| `context_sources` | fontes do pack: caminho, papel, intervalo de linhas, hash do arquivo na criação, pontuação. Permite saber quais packs citaram um arquivo                        |

Os trechos são lidos do disco ao abrir/exportar e comparados com o hash da criação (ADR-0010).

## Versão 4 — `memory_freshness`

| Tabela             | Conteúdo                                                                                                                                                                                                                               |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `memories`         | uma linha por memória: tipo, estado (`proposed`/`confirmed`/`archived`/`superseded`), título, texto, origem (`user`/`cli`/`adr_import`/`agent`), documento e hash de origem, `supersedes`/`superseded_by`, revisão e datas             |
| `memory_refs`      | referências: caminho, linhas opcionais e hash do arquivo indexado quando a referência foi criada ou revisada                                                                                                                           |
| `memory_revisions` | retrato integral de cada revisão (tipo, estado, título, texto, referências em JSON), com autor (`actor`), tipo de mudança e nota                                                                                                       |
| `memories_fts`     | FTS5 (título, texto) mantido por gatilhos; `memory_id` não indexado                                                                                                                                                                    |
| `freshness_issues` | alertas: tipo, `fingerprint` único por projeto, severidade, estado (`open`/`dismissed`/`resolved`), requer revisão, caminho/memórias relacionados, `detail_json` (título, motivo, recomendação, fontes, sugestão, contradição) e datas |
| `freshness_scans`  | última verificação completa por projeto: data, geração do índice, duração, contagens e identificadores citados vistos no código (para detectar remoções)                                                                               |

Memórias e alertas são apagados em cascata com o projeto. `supersedes`/`superseded_by` usam `ON DELETE SET NULL`.

## Versão 5 — `analysis`

| Tabela        | Conteúdo                                                                                                                                                                     |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `symbols`     | declarações por **conteúdo** (hash): nome, `name_norm` (minúsculas, sem `_`/`$`), tipo, linha, fim do corpo, exportado, contêiner. Cascata com `contents`                    |
| `imports`     | imports por conteúdo: linha, especificador como escrito, tipo (`import`, `reexport`, `require`, `dynamic`, `module`). Cascata com `contents`                                 |
| `edges`       | grafo por **caminho**: origem, linha, especificador, tipo, `to_path` (arquivo do projeto) ou `external` (pacote/crate); ambos nulos = não resolvido. Recalculado por geração |
| `graph_state` | última montagem do grafo por projeto: geração, data, duração e contagens                                                                                                     |

As tabelas `symbols_fts` e `symbol_removals`, previstas no plano inicial, não foram criadas (ADR-0007): a busca por nome usa o índice `symbols(project_id, name_norm)`, e símbolos removidos citados em documentos são detectados pela verificação de atualidade. Com `INDEXER_VERSION` = 2, conteúdos indexados antes da Fase 6 são reanalisados na próxima indexação.

## Retenção

- `context_packs`: 100 mais recentes por projeto (fontes removidas em cascata).
- `freshness_issues` resolvidos: removidos 30 dias após a resolução. Memórias e revisões não têm retenção automática.

- `indexing_jobs` (e seus `index_errors`): 50 mais recentes por projeto.
- Conteúdos sem nenhum arquivo apontando para eles são removidos ao fim de cada job (com seus chunks e linhas do FTS).
- `operations`: removidas as com mais de 90 dias **e**, além disso, mantidas no máximo 10 000 linhas (as mais recentes). Executado na inicialização.
- Backups pré-migração: 5 mais recentes.
- Logs: 7 arquivos diários.

## Backup e restauração

- **Backup manual**: com o app e a CLI fechados, copie `nexus.db` (e `nexus.db-wal`, se existir) para outro local. Com processos abertos, prefira `sqlite3 nexus.db ".backup destino.db"`.
- **Restauração**: feche o NEXUS, substitua `nexus.db` pelo backup e remova `nexus.db-wal`/`nexus.db-shm` antigos.
- Backups automáticos ficam em `backups/` e são bancos SQLite completos.
- O índice (a partir da Fase 2) é reconstruível a partir dos arquivos dos projetos; memória e decisões **não** são — faça backup delas.
