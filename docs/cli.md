# CLI `nexus`

A CLI é um adaptador fino sobre o `nexus-core`: usa o **mesmo banco** do app desktop e não duplica regras de negócio.

```bash
cargo run -p nexus-cli -- <comando>          # durante o desenvolvimento
nexus <comando>                              # binário compilado (target/release/nexus.exe)
```

## Opções globais

| Opção                 | Descrição                                                                          |
| --------------------- | ---------------------------------------------------------------------------------- |
| `--data-dir <PASTA>`  | Diretório de dados. Padrão: `NEXUS_DATA_DIR` ou `%LOCALAPPDATA%\Nexus`             |
| `--json`              | Saída JSON estável para automações (erros vão para stderr como `{"error": {...}}`) |
| `--version`, `--help` |                                                                                    |

## Comandos (Fase 1)

### `nexus init`

Cria o diretório de dados e o banco, aplicando migrations pendentes (com backup prévio se o banco já existir).

```text
NEXUS pronto.
  Dados:          C:\Users\voce\AppData\Local\Nexus
  Banco:          C:\Users\voce\AppData\Local\Nexus\nexus.db
  Schema:         v1
  Migrations:     aplicadas 1
```

JSON: `{ "dataDir", "database", "schemaVersion", "appliedMigrations": [..], "backup": null | "caminho" }`.

### `nexus diagnostics [--check]`

Mostra versões, caminhos, tamanhos, migrations, backups e operações. `--check` executa `PRAGMA integrity_check` como operação registrada.

JSON: `{ "app": AppInfo, "diagnostics": DiagnosticsReport, "integrity": IntegrityReport | null }` — os mesmos tipos gerados em `apps/desktop/src/lib/bindings/`.

## Códigos de saída

| Código | Significado                                                            |
| ------ | ---------------------------------------------------------------------- |
| 0      | sucesso                                                                |
| 1      | falha interna ou não classificada                                      |
| 2      | uso inválido (argumentos) ou recurso não encontrado                    |
| 3      | erro de armazenamento ou de E/S                                        |
| 4      | banco criado por uma versão mais nova do NEXUS (`incompatible_schema`) |

## Planejado (Fase 5)

`projects`, `index`, `status`, `search`, `context`, `memory`, `freshness` e `mcp serve` — ver [roadmap](roadmap.md).
