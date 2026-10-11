# ADR-0004 — Contratos TypeScript gerados a partir do Rust (ts-rs)

- Estado: aceita
- Data: 2026-10-10

## Contexto

Tipos duplicados à mão entre Rust e TypeScript divergem silenciosamente.

## Decisão

DTOs são definidos em Rust com `serde` + `ts-rs`. Os arquivos em `apps/desktop/src/lib/bindings/` são gerados pelo teste `crates/nexus-core/tests/bindings.rs` com `NEXUS_UPDATE_BINDINGS=1` (ver `docs/development.md`). Sem a variável, o mesmo teste falha se o conteúdo versionado divergir do gerado — por isso roda em toda execução de `cargo test` e no CI. Inteiros de 64 bits são mapeados para `number` (timestamps em ms e contagens cabem com folga em 2^53).

## Alternativas consideradas

`tauri-specta` (gera também os wrappers de comando) — descartada por ainda depender de releases candidatas para Tauri 2 no momento da decisão.
