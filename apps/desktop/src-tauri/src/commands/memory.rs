//! Memória e atualidade. A interface age como o usuário (`Actor::User`): pode confirmar,
//! arquivar e revisar. As regras de estado e de quem pode revisar vivem no core.

use nexus_core::freshness::{FreshnessIssue, FreshnessSummary, IssueQuery, ScanReport};
use nexus_core::memory::adr::ImportReport;
use nexus_core::memory::{
    Actor, Memory, MemoryCounts, MemoryDetail, MemoryQuery, MemoryUpdate, NewMemory, Transition,
};
use nexus_core::{ErrorCode, ErrorInfo};
use tauri::State;

use crate::state::{AppState, blocking};

const ACTOR: Actor = Actor::User;

fn check_memory_id(id: &str) -> Result<(), ErrorInfo> {
    let ok =
        !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(ErrorInfo {
            code: ErrorCode::InvalidInput,
            message: "Identificador de memória inválido.".to_owned(),
            op_id: None,
        })
    }
}

#[tauri::command]
pub async fn list_memories(
    state: State<'_, AppState>,
    query: MemoryQuery,
) -> Result<Vec<Memory>, ErrorInfo> {
    blocking(&state, move |core| core.list_memories(&query)).await
}

#[tauri::command]
pub async fn get_memory(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<MemoryDetail, ErrorInfo> {
    check_memory_id(&memory_id)?;
    blocking(&state, move |core| core.memory(&memory_id)).await
}

#[tauri::command]
pub async fn get_memory_counts(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<MemoryCounts, ErrorInfo> {
    blocking(&state, move |core| core.memory_counts(project_id)).await
}

#[tauri::command]
pub async fn create_memory(
    state: State<'_, AppState>,
    request: NewMemory,
) -> Result<Memory, ErrorInfo> {
    blocking(&state, move |core| core.create_memory(&request, ACTOR)).await
}

#[tauri::command]
pub async fn update_memory(
    state: State<'_, AppState>,
    memory_id: String,
    request: MemoryUpdate,
) -> Result<Memory, ErrorInfo> {
    check_memory_id(&memory_id)?;
    blocking(&state, move |core| {
        core.update_memory(&memory_id, &request, ACTOR)
    })
    .await
}

#[tauri::command]
pub async fn transition_memory(
    state: State<'_, AppState>,
    memory_id: String,
    transition: Transition,
    expected_revision: Option<i64>,
    note: Option<String>,
) -> Result<Memory, ErrorInfo> {
    check_memory_id(&memory_id)?;
    blocking(&state, move |core| {
        core.transition_memory(
            &memory_id,
            transition,
            expected_revision,
            note.as_deref(),
            ACTOR,
        )
    })
    .await
}

#[tauri::command]
pub async fn review_memory_refs(
    state: State<'_, AppState>,
    memory_id: String,
    expected_revision: i64,
) -> Result<Memory, ErrorInfo> {
    check_memory_id(&memory_id)?;
    blocking(&state, move |core| {
        core.review_memory_refs(&memory_id, expected_revision, ACTOR)
    })
    .await
}

#[tauri::command]
pub async fn import_adrs(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<ImportReport, ErrorInfo> {
    blocking(&state, move |core| core.import_adrs(project_id)).await
}

#[tauri::command]
pub async fn list_freshness_issues(
    state: State<'_, AppState>,
    query: IssueQuery,
) -> Result<Vec<FreshnessIssue>, ErrorInfo> {
    blocking(&state, move |core| core.freshness_issues(&query)).await
}

#[tauri::command]
pub async fn get_freshness_summary(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<FreshnessSummary, ErrorInfo> {
    blocking(&state, move |core| core.freshness_summary(project_id)).await
}

#[tauri::command]
pub async fn scan_freshness(
    state: State<'_, AppState>,
    project_id: i64,
) -> Result<ScanReport, ErrorInfo> {
    blocking(&state, move |core| core.scan_freshness(project_id)).await
}

#[tauri::command]
pub async fn set_issue_dismissed(
    state: State<'_, AppState>,
    issue_id: i64,
    dismissed: bool,
    note: Option<String>,
) -> Result<FreshnessIssue, ErrorInfo> {
    blocking(&state, move |core| {
        core.set_issue_dismissed(issue_id, dismissed, note.as_deref())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_ids_are_validated() {
        assert!(check_memory_id("0199aaaa-bbbb-7ccc-8ddd-eeeeffff0000").is_ok());
        assert!(check_memory_id("").is_err());
        assert!(check_memory_id("../x").is_err());
        assert!(check_memory_id(&"a".repeat(65)).is_err());
    }
}
