//! Servidor MCP do NEXUS sobre stdio (ADR-0005), implementado à mão sobre JSON-RPC 2.0.
//!
//! Dual-era: atende a revisão 2026-07-28 da especificação (sem handshake, `_meta` por
//! requisição, `server/discover`) e clientes que ainda usam o `initialize` das revisões
//! 2025-03-26 a 2025-11-25. Ferramentas somente leitura, exceto `refresh_project_index` e
//! `build_context_pack`, que só gravam dados locais do NEXUS.

pub mod protocol;
pub mod server;
pub mod tools;

pub use server::Server;
