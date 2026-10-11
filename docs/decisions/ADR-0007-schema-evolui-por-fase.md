# ADR-0007 — Migrations versionadas que evoluem com as fases

- Estado: aceita
- Data: 2026-10-10

## Decisão
- Migrations SQL numeradas, embutidas no binário, aplicadas em ordem, uma transação por migration.
- `schema_migrations` registra versão, nome, checksum (blake3 do SQL), data e duração. Checksum divergente em migration já aplicada ⇒ erro explícito.
- Antes de aplicar migrations pendentes num banco existente, é feito backup pela Online Backup API (retenção dos 5 mais recentes).
- Banco com versão maior que a conhecida pelo binário ⇒ recusa (`incompatible_schema`).
- Tabelas são criadas na fase em que passam a ser usadas; nada de schema especulativo.
