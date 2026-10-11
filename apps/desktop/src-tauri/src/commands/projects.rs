use std::path::{Path, PathBuf};

use nexus_core::index::{IndexError, IndexJob, IndexMode, IndexTrigger, IndexingQueue};
use nexus_core::projects::{ProjectOverview, ProjectSummary, SkippedFile};
use nexus_core::{ErrorCode, ErrorInfo};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::indexing::Request;
use crate::state::{AppState, Indexing, blocking};

const MAX_PATH_LEN: usize = 4096;

fn invalid(message: impl Into<String>) -> ErrorInfo {
    ErrorInfo {
        code: ErrorCode::InvalidInput,
        message: message.into(),
        op_id: None,
    }
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, ErrorInfo> {
    blocking(&state, |core| core.list_projects()).await
}

#[tauri::command]
pub async fn get_project_overview(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<ProjectOverview, ErrorInfo> {
    blocking(&state, move |core| core.project_overview(project_id)).await
}

/// Abre o seletor de pastas nativo. O caminho escolhido ainda passa pela validação do core.
#[tauri::command]
pub async fn pick_project_folder(app: tauri::AppHandle) -> Result<Option<String>, ErrorInfo> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Escolha a pasta do projeto")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| ErrorInfo {
        code: ErrorCode::Internal,
        message: format!("o seletor de pastas falhou: {e}"),
        op_id: None,
    })?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string()))
}

/// Autoriza a pasta, passa a monitorá-la e enfileira a primeira indexação.
#[tauri::command]
pub async fn add_project(
    state: State<'_, AppState>,
    indexing: State<'_, Indexing>,
    path: String,
) -> Result<ProjectSummary, ErrorInfo> {
    if path.trim().is_empty() || path.len() > MAX_PATH_LEN {
        return Err(invalid("Caminho de pasta inválido."));
    }
    let path = PathBuf::from(path.trim());
    let summary = blocking(&state, move |core| {
        let project = core.add_project(&path)?;
        core.project_summary(project.id)
    })
    .await?;
    let parts = indexing.parts()?;
    parts.resync();
    parts.manager.enqueue(Request {
        project_id: summary.project.id,
        mode: IndexMode::Incremental,
        trigger: IndexTrigger::User,
    })?;
    Ok(summary)
}

/// Remove do índice. Os arquivos da pasta não são alterados.
#[tauri::command]
pub async fn remove_project(
    state: State<'_, AppState>,
    indexing: State<'_, Indexing>,
    project_id: i64,
) -> Result<(), ErrorInfo> {
    let parts = indexing.parts()?;
    parts.manager.cancel(project_id);
    blocking(&state, move |core| {
        // O job cancelado pode levar um instante para liberar a trava.
        let mut attempt = 0;
        loop {
            match core.remove_project(project_id) {
                Err(e) if e.code() == ErrorCode::Conflict && attempt < 30 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                other => return other,
            }
        }
    })
    .await?;
    parts.resync();
    Ok(())
}

#[tauri::command]
pub fn start_index(
    indexing: State<'_, Indexing>,
    project_id: i64,
    mode: IndexMode,
) -> Result<(), ErrorInfo> {
    indexing.parts()?.manager.enqueue(Request {
        project_id,
        mode,
        trigger: IndexTrigger::User,
    })
}

#[tauri::command]
pub fn pause_index(indexing: State<'_, Indexing>, project_id: i64) -> Result<bool, ErrorInfo> {
    Ok(indexing.parts()?.manager.pause(project_id))
}

#[tauri::command]
pub fn resume_index(indexing: State<'_, Indexing>, project_id: i64) -> Result<bool, ErrorInfo> {
    Ok(indexing.parts()?.manager.resume(project_id))
}

#[tauri::command]
pub fn cancel_index(indexing: State<'_, Indexing>, project_id: i64) -> Result<bool, ErrorInfo> {
    Ok(indexing.parts()?.manager.cancel(project_id))
}

#[tauri::command]
pub fn get_indexing_queue(indexing: State<'_, Indexing>) -> Result<IndexingQueue, ErrorInfo> {
    Ok(indexing.parts()?.manager.queue())
}

#[tauri::command]
pub async fn list_jobs(
    state: State<'_, AppState>,
    project_id: i64,
    limit: u32,
) -> Result<Vec<IndexJob>, ErrorInfo> {
    blocking(&state, move |core| core.list_jobs(project_id, limit)).await
}

#[tauri::command]
pub async fn get_job_errors(
    state: State<'_, AppState>,
    job_id: String,
    limit: u32,
) -> Result<Vec<IndexError>, ErrorInfo> {
    let valid = !job_id.is_empty()
        && job_id.len() <= 64
        && job_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid {
        return Err(invalid("Identificador de job inválido."));
    }
    blocking(&state, move |core| core.job_errors(&job_id, limit)).await
}

#[tauri::command]
pub async fn list_skipped_files(
    state: State<'_, AppState>,
    project_id: i64,
    limit: u32,
) -> Result<Vec<SkippedFile>, ErrorInfo> {
    blocking(&state, move |core| core.skipped_files(project_id, limit)).await
}

#[tauri::command]
pub async fn touch_project(state: State<'_, AppState>, project_id: i64) -> Result<(), ErrorInfo> {
    blocking(&state, move |core| core.touch_project(project_id)).await
}

/// Abre o explorador na pasta de um projeto registrado (nunca em caminho arbitrário).
#[tauri::command]
pub async fn reveal_project_dir(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<(), ErrorInfo> {
    let project = blocking(&state, move |core| core.project(project_id)).await?;
    crate::commands::app::open_in_file_manager(Path::new(&project.root_path)).map_err(|e| {
        ErrorInfo {
            code: ErrorCode::Io,
            message: format!("Não foi possível abrir a pasta: {e}"),
            op_id: None,
        }
    })
}
