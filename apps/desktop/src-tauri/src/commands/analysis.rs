//! Análise de código: cobertura por linguagem, grafo e dependências de um arquivo.

use nexus_core::analysis::query::{AnalysisOverview, FileAnalysisView};
use nexus_core::{ErrorCode, ErrorInfo};
use tauri::State;

use crate::state::{AppState, blocking};

const MAX_PATH_LEN: usize = 1024;

#[tauri::command]
pub async fn get_analysis_overview(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<AnalysisOverview, ErrorInfo> {
    blocking(&state, move |core| core.analysis_overview(project_id)).await
}

#[tauri::command]
pub async fn get_file_analysis(
    state: State<'_, AppState>,
    project_id: i64,
    rel_path: String,
) -> Result<FileAnalysisView, ErrorInfo> {
    if rel_path.is_empty() || rel_path.len() > MAX_PATH_LEN {
        return Err(ErrorInfo {
            code: ErrorCode::InvalidInput,
            message: "Caminho de arquivo inválido.".to_owned(),
            op_id: None,
        });
    }
    blocking(&state, move |core| {
        core.file_analysis(project_id, &rel_path)
    })
    .await
}
