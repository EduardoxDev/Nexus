//! Comandos expostos ao webview. Cada um é fino: valida a entrada (via tipos) e delega ao core.
//! A lista precisa estar sincronizada com `build.rs` e `capabilities/default.json`.

pub mod app;
pub mod diagnostics;
pub mod projects;
pub mod settings;
