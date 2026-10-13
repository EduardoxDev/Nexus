//! Comandos expostos ao webview. Cada um é fino: valida a entrada (via tipos) e delega ao core.
//! A lista precisa estar sincronizada com `build.rs` e `capabilities/default.json`.

pub mod analysis;
pub mod app;
pub mod context;
pub mod diagnostics;
pub mod integrations;
pub mod memory;
pub mod projects;
pub mod settings;
pub mod updates;
