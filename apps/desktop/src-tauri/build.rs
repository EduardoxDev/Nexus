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
];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(err) = tauri_build::try_build(attributes) {
        eprintln!("falha no tauri-build: {err:#}");
        std::process::exit(1);
    }
}
