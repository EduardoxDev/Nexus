# Contribuindo com o NEXUS

## Antes de começar

Leia a [arquitetura](docs/architecture.md), o [roadmap](docs/roadmap.md) e as [decisões registradas](docs/decisions/). Mudanças que contrariem uma ADR precisam de uma nova ADR.

## Regras de engenharia

- **Lógica de negócio só no `nexus-core`.** Desktop, CLI e MCP são adaptadores finos.
- **Contratos gerados.** Alterou um DTO em Rust? Regenere os tipos: `NEXUS_UPDATE_BINDINGS=1 cargo test -p nexus-core --test bindings`.
- **Novo comando do app** precisa aparecer em três lugares (há teste garantindo): `build.rs` (`COMMANDS`), `capabilities/default.json` (`allow-…`) e `generate_handler!` em `src/lib.rs`.
- **Schema** muda apenas por nova migration numerada; migrations publicadas nunca são editadas.
- **Textos da UI** ficam em `apps/desktop/src/lib/i18n/pt-BR.ts`.
- **UI honesta**: nada de dados simulados fora de testes; funcionalidades futuras aparecem desabilitadas com a fase prevista.
- **Logs** nunca contêm conteúdo de arquivos nem segredos.
- Dependências novas precisam de justificativa (no PR e, se relevante, em `docs/development.md`).

## Fluxo

1. Crie um branch a partir de `main`.
2. Escreva testes junto com a mudança.
3. Rode a bateria completa ([docs/testing.md](docs/testing.md)) — clippy com `-D warnings`, tsc, eslint e prettier sem exceções não justificadas.
4. Atualize a documentação afetada.
5. Abra o PR descrevendo o que mudou, como foi testado e limitações conhecidas.

## Estilo

- Rust: `rustfmt` + Clippy; `unsafe` é proibido no workspace.
- TypeScript: strict, `typescript-eslint` (strict type-checked), Prettier.
- Mensagens de commit no imperativo, descrevendo o porquê.
