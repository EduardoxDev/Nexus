# ADR-0004 — Contratos TypeScript gerados a partir do Rust (ts-rs)

- Estado: aceita
- Data: 2026-10-10

## Contexto
Tipos duplicados à mão entre Rust e TypeScript divergem silenciosamente.

## Decisão
DTOs são definidos em Rust com `serde` + `ts-rs`. Os arquivos em `apps/desktop/src/lib/bindings/` são gerados por `cargo test -p nexus-core --features ts-export` (ver `docs/development.md`); um teste falha se o conteúdo gerado divergir do versionado.

## Alternativas consideradas
`tauri-specta` (gera também os wrappers de comando) — descartada por ainda depender de releases candidatas para Tauri 2 no momento da decisão.
