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

## Comandos

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

### `nexus projects [list]`

Lista os projetos autorizados com contagens e data da última indexação. JSON: `ProjectSummary[]`.

### `nexus projects add <PASTA> [--index]`

Autoriza a pasta (validada e canonicalizada; raiz de disco, pasta pessoal, pastas do sistema, a pasta de dados do NEXUS e sobreposições são recusadas). Com `--index`, indexa em seguida. JSON: `{ "project": Project, "job": IndexJob | null }`.

### `nexus projects remove <PROJETO> --yes`

Remove o projeto **do índice**. Sem `--yes`, recusa com código 2. Nenhum arquivo da pasta é alterado. `<PROJETO>` aceita ID, nome ou caminho.

### `nexus index <PROJETO> [--full]`

Indexação incremental (padrão) ou completa (`--full`, relê e confere o hash de todos os arquivos). Progresso em stderr na saída humana. Usa as configurações `indexing.threads` e `indexing.max_file_size_kb`. Se outro processo (ex.: o app) já estiver indexando o mesmo projeto, falha com `conflict`. JSON: `IndexJob`.

```text
Indexação concluída em 0.5 s (geração 1).
  Encontrados 255 · processados 255 · inalterados 0 · ignorados 20 · removidos 0 · renomeados 0 · erros 0
  Conteúdos novos 235 · reaproveitados 0 · lidos 916,0 KiB
```

### `nexus status [PROJETO]`

Arquivos indexados/ignorados/com erro, tamanho, linhas, geração do índice, ref e commit do git, último job. JSON: `ProjectSummary[]`.

### `nexus search <CONSULTA> [--project P] [--limit N] [--kind K]... [--path PASTA] [--plain]`

Busca híbrida (padrão) ou simples (`--plain`, só bm25 com os termos digitados). Com um único projeto, `--project` é opcional. `--kind` aceita `code`, `test`, `doc`, `config`, `manifest`, `ci` (pode repetir). Saída humana mostra caminho, linhas, pontuação, motivos, "também em" e as 6 primeiras linhas do trecho (`>` marca linhas com correspondência). JSON: `SearchResponse`.

```text
1. src/checkout/shipping.ts:1-7  (pontuação 0.882)
   motivos: Conteúdo corresponde à consulta · Caminho corresponde a: frete→shipping · …
```

### `nexus context <TAREFA> [--project P] [--budget N] [--max-files N] [--format md|json] [--output ARQUIVO] [--exclude CAMINHO]... [--notes TEXTO]`

Compila e persiste um Context Pack e imprime (ou grava) a exportação. `--budget` 1000–50000 tokens estimados (padrão 8000); `--max-files` 1–40 (padrão 12). Com `--json` global, exporta em JSON. O Markdown começa com o aviso de que trechos citados são dados, não instruções.

### `nexus memory [list] [--project P] [--status S]... [--kind K]... [--query TEXTO] [--limit N]`

Lista memórias (padrão: propostas e confirmadas). Estados: `proposed`, `confirmed`, `archived`, `superseded`. Tipos: `decision`, `convention`, `requirement`, `constraint`, `bug`, `solution`, `dependency`, `task`, `lesson`. A coluna ID mostra os 8 últimos caracteres; qualquer comando aceita o ID completo ou 6+ caracteres do início ou do fim.

### `nexus memory show <ID>`

Texto, referências com estado atual (`inalterado`, `MUDOU desde o registro`, `MOVIDO → novo`, `AUSENTE`, `fora do índice`) e histórico de revisões.

### `nexus memory add --kind K --title T [--body TEXTO | --body-file ARQUIVO] [--ref CAMINHO[:LINHAS]]... [--confirm] [--note TEXTO]`

Registra uma memória. Sem `--confirm`, fica como proposta. Referências precisam ser arquivos indexados: `src/a.ts`, `src/a.ts:12` ou `src/a.ts:10-40`.

### `nexus memory edit <ID> [--kind] [--title] [--body | --body-file] [--ref ...]... [--clear-refs] [--revision N] [--note]`

Cria uma revisão. `--revision` é a revisão que você viu (padrão: a atual); se a memória mudou nesse meio-tempo, sai com código 5.

### `nexus memory confirm|archive <ID> [--note]`, `nexus memory restore <ID>`, `nexus memory review-refs <ID>`

Mudanças de estado e revisão de referências (registra o hash atual e corrige caminhos movidos).

### `nexus memory import-adrs [--project P]`

Importa os ADRs indexados como propostas. Idempotente; decisões confirmadas nunca são sobrescritas (uma mudança no ADR vira proposta de substituição).

### `nexus freshness [list] [--all] [--project P]`, `nexus freshness scan`

Resumo e alertas abertos (com `--all`, também dispensados e resolvidos): motivo, fontes (`caminho:linha`), recomendação e se requer revisão humana. `scan` verifica agora; a verificação também roda sozinha depois de cada indexação que muda o índice.

### `nexus freshness dismiss <ID> [--note TEXTO]`, `nexus freshness reopen <ID>`

Dispensa (continua dispensado enquanto o problema persistir) ou reabre um alerta.

A CLI age como o usuário: pode confirmar e revisar memórias. Veja [ADR-0011](decisions/ADR-0011-memoria-revisada-e-alertas-estaveis.md).

## Códigos de saída

| Código | Significado                                                              |
| ------ | ------------------------------------------------------------------------ |
| 0      | sucesso                                                                  |
| 1      | falha interna ou não classificada                                        |
| 2      | uso inválido (argumentos), confirmação ausente ou recurso não encontrado |
| 3      | erro de armazenamento ou de E/S                                          |
| 4      | banco criado por uma versão mais nova do NEXUS (`incompatible_schema`)   |
| 5      | conflito: revisão desatualizada ou ação não permitida para a origem      |

## Planejado (Fase 5)

`mcp serve` — ver [roadmap](roadmap.md).
