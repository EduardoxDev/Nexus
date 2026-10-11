//! Shell desktop do NEXUS. Mantém-se fino: inicializa logging e core, registra plugins e comandos.

mod commands;
mod state;

use nexus_core::logging::{self, Console};
use nexus_core::{Core, DataPaths};
use tauri::Manager;

use crate::state::AppState;

pub fn run() {
    let paths = DataPaths::resolve();
    let console = if cfg!(debug_assertions) {
        Console::Stderr
    } else {
        Console::Off
    };
    let log_guard = paths.as_ref().ok().and_then(|p| {
        p.ensure().ok()?;
        logging::init(&p.logs(), console).ok()
    });

    let core = match paths {
        Ok(paths) => {
            let dir = paths.root().display().to_string();
            Core::open(paths).map_err(|e| (Some(dir), e))
        }
        Err(e) => Err((None, e)),
    };
    let state = AppState::new(core);

    let result = tauri::Builder::default()
        // Precisa ser o primeiro plugin: uma segunda execução apenas foca a janela existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(state)
        .setup(|app| {
            let info = commands::app::app_info(app.handle());
            tracing::info!(
                version = %info.app_version,
                webview = ?info.webview_version,
                "app iniciado"
            );
            if let Ok(core) = app.state::<AppState>().core() {
                let detail = serde_json::json!({
                    "appVersion": info.app_version,
                    "webviewVersion": info.webview_version,
                    "buildProfile": info.build_profile,
                });
                if let Err(e) = core.record_event(nexus_core::ops::kinds::APP_STARTUP, detail) {
                    tracing::warn!(error = %e, "não foi possível registrar a inicialização");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_startup_status,
            commands::app::get_app_info,
            commands::app::reveal_data_dir,
            commands::diagnostics::get_diagnostics,
            commands::diagnostics::run_integrity_check,
            commands::diagnostics::list_operations,
            commands::settings::get_settings,
            commands::settings::set_setting,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!(error = %err, "erro fatal no runtime do Tauri");
        drop(log_guard);
        std::process::exit(1);
    }
    drop(log_guard);
}
