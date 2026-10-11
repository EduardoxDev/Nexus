use nexus_core::ErrorInfo;
use nexus_core::diagnostics::{DiagnosticsReport, IntegrityReport};
use nexus_core::ops::{OperationQuery, OperationRecord};
use tauri::State;

use crate::state::{AppState, blocking};

#[tauri::command]
pub async fn get_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticsReport, ErrorInfo> {
    blocking(&state, |core| core.diagnostics()).await
}

#[tauri::command]
pub async fn run_integrity_check(state: State<'_, AppState>) -> Result<IntegrityReport, ErrorInfo> {
    blocking(&state, |core| core.integrity_check()).await
}

/// `query` é validado pelo core (limite 1..=200, prefixo restrito).
#[tauri::command]
pub async fn list_operations(
    state: State<'_, AppState>,
    query: OperationQuery,
) -> Result<Vec<OperationRecord>, ErrorInfo> {
    blocking(&state, move |core| core.list_operations(&query)).await
}
