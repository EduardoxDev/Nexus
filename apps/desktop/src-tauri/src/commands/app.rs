use nexus_core::diagnostics::{AppInfo, StartupStatus};
use nexus_core::{ErrorCode, ErrorInfo};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn get_startup_status(state: State<'_, AppState>) -> StartupStatus {
    state.startup().clone()
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    app_info(&app)
}

pub fn app_info(app: &tauri::AppHandle) -> AppInfo {
    let mut info = AppInfo::base(&app.package_info().version.to_string());
    info.tauri_version = Some(tauri::VERSION.to_owned());
    info.webview_version = tauri::webview_version().ok();
    info
}

/// Abre o explorador de arquivos no diretório de dados do NEXUS — e somente nele.
#[tauri::command]
pub async fn reveal_data_dir(state: State<'_, AppState>) -> Result<(), ErrorInfo> {
    let dir = state.startup().data_dir.clone().ok_or_else(|| ErrorInfo {
        code: ErrorCode::NotFound,
        message: "O diretório de dados não foi determinado.".into(),
        op_id: None,
    })?;
    open_in_file_manager(std::path::Path::new(&dir)).map_err(|e| ErrorInfo {
        code: ErrorCode::Io,
        message: format!("Não foi possível abrir o explorador de arquivos: {e}"),
        op_id: None,
    })
}

pub(crate) fn open_in_file_manager(dir: &std::path::Path) -> std::io::Result<()> {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    // Programa fixo, argumento é um caminho resolvido pelo próprio app (sem shell).
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
}
