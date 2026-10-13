// Declara explicitamente os comandos do app: cada um vira uma permissão `allow-<comando>` que
// precisa constar em `capabilities/` para ser invocável pelo webview.
const COMMANDS: &[&str] = &[
    "get_startup_status",
    "get_app_info",
    "get_diagnostics",
    "run_integrity_check",
    "list_operations",
    "get_settings",
    "set_setting",
    "reveal_data_dir",
    "list_projects",
    "get_project_overview",
    "pick_project_folder",
    "add_project",
    "remove_project",
    "start_index",
    "pause_index",
    "resume_index",
    "cancel_index",
    "get_indexing_queue",
    "list_jobs",
    "get_job_errors",
    "list_skipped_files",
    "touch_project",
    "reveal_project_dir",
    "search_project",
    "get_file_context",
    "compile_context_pack",
    "list_context_packs",
    "get_context_pack",
    "delete_context_pack",
    "render_context_pack",
    "save_context_pack",
    "list_memories",
    "get_memory",
    "get_memory_counts",
    "create_memory",
    "update_memory",
    "transition_memory",
    "review_memory_refs",
    "import_adrs",
    "list_freshness_issues",
    "get_freshness_summary",
    "scan_freshness",
    "set_issue_dismissed",
    "get_integration_info",
    "get_analysis_overview",
    "get_file_analysis",
    "check_for_update",
    "install_update",
];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(err) = tauri_build::try_build(attributes) {
        eprintln!("falha no tauri-build: {err:#}");
        std::process::exit(1);
    }
}
