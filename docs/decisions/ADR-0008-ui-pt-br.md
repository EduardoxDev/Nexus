# ADR-0008 — Interface e documentação em pt-BR com camada de i18n

- Estado: aceita
- Data: 2026-10-10

## Decisão
Textos da interface e documentação em português do Brasil. Identificadores de código, nomes de crates, contratos IPC/MCP/JSON e nomes de ferramentas MCP permanecem em inglês. Todos os textos da UI ficam em `apps/desktop/src/lib/i18n/pt-BR.ts`, permitindo adicionar outros idiomas sem tocar nos componentes. Datas e números usam `Intl` com locale `pt-BR`.
