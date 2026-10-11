use nexus_core::context::export::{ExportFormat, ExportOptions};
use nexus_core::context::{ContextPack, ContextPackRequest, ContextPackSummary};
use nexus_core::search::evidence::FileContext;
use nexus_core::search::{SearchRequest, SearchResponse};
use nexus_core::{ErrorCode, ErrorInfo};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::state::{AppState, blocking};

const MAX_PATH_LEN: usize = 1024;

fn invalid(message: &str) -> ErrorInfo {
    ErrorInfo {
        code: ErrorCode::InvalidInput,
        message: message.to_owned(),
        op_id: None,
    }
}

fn check_pack_id(id: &str) -> Result<(), ErrorInfo> {
    let ok =
        !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(invalid("Identificador de Context Pack inválido."))
    }
}

#[tauri::command]
pub async fn search_project(
    state: State<'_, AppState>,
    request: SearchRequest,
) -> Result<SearchResponse, ErrorInfo> {
    blocking(&state, move |core| core.search(&request)).await
}

/// Trecho de um arquivo **indexado**; arquivos ignorados (ex.: sensíveis) não podem ser lidos.
#[tauri::command]
pub async fn get_file_context(
    state: State<'_, AppState>,
    project_id: i64,
    rel_path: String,
    start_line: Option<i64>,
    end_line: Option<i64>,
) -> Result<FileContext, ErrorInfo> {
    if rel_path.is_empty() || rel_path.len() > MAX_PATH_LEN {
        return Err(invalid("Caminho de arquivo inválido."));
    }
    blocking(&state, move |core| {
        core.file_context(project_id, &rel_path, start_line, end_line)
    })
    .await
}

#[tauri::command]
pub async fn compile_context_pack(
    state: State<'_, AppState>,
    request: ContextPackRequest,
) -> Result<ContextPack, ErrorInfo> {
    blocking(&state, move |core| core.compile_context_pack(&request)).await
}

#[tauri::command]
pub async fn list_context_packs(
    state: State<'_, AppState>,
    project_id: i64,
    limit: u32,
) -> Result<Vec<ContextPackSummary>, ErrorInfo> {
    blocking(&state, move |core| {
        core.list_context_packs(project_id, limit)
    })
    .await
}

#[tauri::command]
pub async fn get_context_pack(
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<ContextPack, ErrorInfo> {
    check_pack_id(&pack_id)?;
    blocking(&state, move |core| core.context_pack(&pack_id)).await
}

#[tauri::command]
pub async fn delete_context_pack(
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<(), ErrorInfo> {
    check_pack_id(&pack_id)?;
    blocking(&state, move |core| core.delete_context_pack(&pack_id)).await
}

/// Texto exportado (para copiar ou pré-visualizar).
#[tauri::command]
pub async fn render_context_pack(
    state: State<'_, AppState>,
    pack_id: String,
    format: ExportFormat,
    options: ExportOptions,
) -> Result<String, ErrorInfo> {
    check_pack_id(&pack_id)?;
    blocking(&state, move |core| {
        core.export_context_pack(&pack_id, format, &options)
    })
    .await
}

/// Pergunta onde salvar (diálogo nativo) e grava o arquivo. `None` = usuário cancelou.
#[tauri::command]
pub async fn save_context_pack(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pack_id: String,
    format: ExportFormat,
    options: ExportOptions,
) -> Result<Option<String>, ErrorInfo> {
    check_pack_id(&pack_id)?;
    let rendered = blocking(&state, {
        let pack_id = pack_id.clone();
        move |core| core.export_context_pack(&pack_id, format, &options)
    })
    .await?;
    let (extension, label) = match format {
        ExportFormat::Markdown => ("md", "Markdown"),
        ExportFormat::Json => ("json", "JSON"),
    };
    let default_name = format!(
        "context-pack-{}.{extension}",
        &pack_id[..8.min(pack_id.len())]
    );
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Salvar Context Pack")
            .set_file_name(default_name)
            .add_filter(label, &[extension])
            .blocking_save_file()
    })
    .await
    .map_err(|e| ErrorInfo {
        code: ErrorCode::Internal,
        message: format!("o diálogo de salvar falhou: {e}"),
        op_id: None,
    })?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    std::fs::write(&path, rendered).map_err(|e| ErrorInfo {
        code: ErrorCode::Io,
        message: format!("Não foi possível gravar {}: {e}", path.display()),
        op_id: None,
    })?;
    Ok(Some(path.display().to_string()))
}
