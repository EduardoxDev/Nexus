# ADR-0001 — Workspace Cargo com poucas crates de fronteira real

- Estado: aceita
- Data: 2026-10-10

## Contexto
O prompt de referência sugere várias crates (`nexus-domain`, `nexus-storage`, `nexus-indexer`…). Dividir cedo demais gera crates vazias, dependências circulares e churn de versões internas.

## Decisão
- `crates/nexus-core`: toda a lógica de negócio, sem dependência de UI, organizada em módulos (`storage`, `settings`, `ops`, `diagnostics`, depois `scan`, `index`, `search`, `context`, `memory`, `freshness`).
- `crates/nexus-cli`: binário `nexus`.
- `crates/nexus-mcp` (Fase 5): servidor MCP como biblioteca testável sem processo.
- `apps/desktop/src-tauri`: shell Tauri com comandos finos.

Um módulo vira crate própria apenas quando houver ganho concreto (tempo de compilação, reutilização externa, isolamento de dependências pesadas).

## Consequências
Fronteiras são mantidas por módulos e visibilidade (`pub(crate)`). Desktop, CLI e MCP não duplicam regras de negócio.
