//! Análise de código: cobertura por linguagem, grafo, dependências e histórico de um arquivo.

use nexus_core::analysis::map::{MapRequest, ProjectMap};
use nexus_core::analysis::query::{AnalysisOverview, FileAnalysisView};
use nexus_core::git_history::FileHistory;
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

/// Mapa do código (imports internos, graus e centralidade), cortado nos nós mais centrais.
#[tauri::command]
pub async fn get_project_map(
    state: State<'_, AppState>,
    request: MapRequest,
) -> Result<ProjectMap, ErrorInfo> {
    if request
        .folder
        .as_ref()
        .is_some_and(|f| f.len() > MAX_PATH_LEN)
    {
        return Err(ErrorInfo {
            code: ErrorCode::InvalidInput,
            message: "Pasta inválida.".to_owned(),
            op_id: None,
        });
    }
    blocking(&state, move |core| core.project_map(&request)).await
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

/// Últimos commits de um arquivo indexado, pelo `git` instalado (só leitura, ADR-0013).
#[tauri::command]
pub async fn get_file_history(
    state: State<'_, AppState>,
    project_id: i64,
    rel_path: String,
) -> Result<FileHistory, ErrorInfo> {
    if rel_path.is_empty() || rel_path.len() > MAX_PATH_LEN {
        return Err(ErrorInfo {
            code: ErrorCode::InvalidInput,
            message: "Caminho de arquivo inválido.".to_owned(),
            op_id: None,
        });
    }
    blocking(&state, move |core| {
        core.file_history_view(project_id, &rel_path, 10)
    })
    .await
}
