# Schema do banco

Arquivo: `nexus.db` no diretório de dados (`%LOCALAPPDATA%\Nexus` no Windows, `$XDG_DATA_HOME/Nexus` no Linux, `~/Library/Application Support/Nexus` no macOS; `NEXUS_DATA_DIR` sobrescreve).

Pragmas aplicados em toda conexão: `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`.

## Migrations

| Versão | Nome               | Fase              | Tabelas                                                                                   |
| ------ | ------------------ | ----------------- | ----------------------------------------------------------------------------------------- |
| 1      | `foundation`       | 1                 | `settings`, `operations`                                                                  |
| 2      | `projects_index`   | 2 _(planejada)_   | `projects`, `files`, `chunks`, `chunks_fts`, `paths_fts`, `indexing_jobs`, `index_errors` |
| 3      | `analysis`         | 2/6 _(planejada)_ | `symbols`, `symbols_fts`, `symbol_removals`, `edges`                                      |
| 4      | `context`          | 3 _(planejada)_   | `context_packs`, `context_sources`                                                        |
| 5      | `memory_freshness` | 4 _(planejada)_   | `memories`, `memory_refs`, `memory_revisions`, `memories_fts`, `freshness_issues`         |

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

Chaves atuais: `ui.theme` (`"system" \| "dark" \| "light"`), `ui.reduced_motion` (`"system" \| "on" \| "off"`), `ui.sidebar_collapsed` (bool).

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

## Retenção

- `operations`: removidas as com mais de 90 dias **e**, além disso, mantidas no máximo 10 000 linhas (as mais recentes). Executado na inicialização.
- Backups pré-migração: 5 mais recentes.
- Logs: 7 arquivos diários.

## Backup e restauração

- **Backup manual**: com o app e a CLI fechados, copie `nexus.db` (e `nexus.db-wal`, se existir) para outro local. Com processos abertos, prefira `sqlite3 nexus.db ".backup destino.db"`.
- **Restauração**: feche o NEXUS, substitua `nexus.db` pelo backup e remova `nexus.db-wal`/`nexus.db-shm` antigos.
- Backups automáticos ficam em `backups/` e são bancos SQLite completos.
- O índice (a partir da Fase 2) é reconstruível a partir dos arquivos dos projetos; memória e decisões **não** são — faça backup delas.
