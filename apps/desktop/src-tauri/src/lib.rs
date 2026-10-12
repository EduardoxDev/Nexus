//! Shell desktop do NEXUS. Mantém-se fino: inicializa logging e core, registra plugins e comandos,
//! e hospeda a fila de indexação e o monitor de arquivos.

mod commands;
mod indexing;
mod state;
mod watch;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use nexus_core::index::{IndexMode, IndexTrigger, JobStatus};
use nexus_core::logging::{self, Console};
use nexus_core::{Core, DataPaths};
use tauri::Manager;

use crate::indexing::{JobManager, Request};
use crate::state::{AppState, Indexing, IndexingParts};
use crate::watch::ProjectWatcher;

/// Reconciliação periódica: corrige eventos de arquivo perdidos.
const RECONCILE_EVERY: Duration = Duration::from_secs(10 * 60);

/// Enfileira uma passada incremental em todos os projetos cuja pasta existe.
/// Projetos cujo último job foi interrompido são marcados como retomada.
fn queue_all(core: &Core, manager: &JobManager, default_trigger: IndexTrigger) {
    let Ok(projects) = core.list_projects() else {
        return;
    };
    for summary in projects.into_iter().filter(|s| s.root_exists) {
        let interrupted = summary
            .last_job
            .as_ref()
            .is_some_and(|j| j.status == JobStatus::Interrupted);
        let _ = manager.enqueue(Request {
            project_id: summary.project.id,
            mode: IndexMode::Incremental,
            trigger: if interrupted {
                IndexTrigger::Resume
            } else {
                default_trigger
            },
        });
    }
}

fn start_reconciler(core: Arc<Core>, manager: JobManager, stop: Arc<AtomicBool>) {
    let spawned = std::thread::Builder::new()
        .name("nexus-reconcile".into())
        .spawn(move || {
            let mut last = Instant::now();
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
                if last.elapsed() < RECONCILE_EVERY {
                    continue;
                }
                last = Instant::now();
                if core.settings().is_ok_and(|s| s.watch_projects) {
                    queue_all(&core, &manager, IndexTrigger::Reconcile);
                }
            }
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "não foi possível iniciar a reconciliação periódica");
    }
}

/// Argumentos padrão do Tauri/wry para o WebView2 (mantidos ao acrescentar outros).
#[cfg(debug_assertions)]
const DEFAULT_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

/// Porta CDP pedida pelos testes E2E (`NEXUS_E2E_CDP_PORT`). Só existe em builds de
/// desenvolvimento: o WebView2 deixou de respeitar `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`
/// neste ambiente, então a porta é passada explicitamente na criação da janela.
#[cfg(debug_assertions)]
fn e2e_cdp_port() -> Option<u16> {
    std::env::var("NEXUS_E2E_CDP_PORT").ok()?.parse().ok()
}

/// Cria a janela principal a partir da configuração (`create: false` no tauri.conf.json).
fn create_main_window(app: &tauri::App) -> tauri::Result<()> {
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main") else {
        tracing::error!("configuração da janela principal ausente");
        return Ok(());
    };
    #[allow(unused_mut)]
    let mut builder = tauri::WebviewWindowBuilder::from_config(app.handle(), config)?;
    #[cfg(debug_assertions)]
    if let Some(port) = e2e_cdp_port() {
        builder = builder.additional_browser_args(&format!(
            "{DEFAULT_BROWSER_ARGS} --remote-debugging-port={port}"
        ));
    }
    builder.build()?;
    Ok(())
}

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
    let stop = Arc::new(AtomicBool::new(false));
    let stop_setup = Arc::clone(&stop);

    let app = tauri::Builder::default()
        // Precisa ser o primeiro plugin: uma segunda execução apenas foca a janela existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Usado apenas pelo Rust (seletor de pastas); o webview não recebe permissões do plugin.
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| {
            create_main_window(app)?;
            let info = commands::app::app_info(app.handle());
            tracing::info!(
                version = %info.app_version,
                webview = ?info.webview_version,
                "app iniciado"
            );
            let core = app.state::<AppState>().core().ok();
            let parts = core.map(|core| {
                let detail = serde_json::json!({
                    "appVersion": info.app_version,
                    "webviewVersion": info.webview_version,
                    "buildProfile": info.build_profile,
                });
                if let Err(e) = core.record_event(nexus_core::ops::kinds::APP_STARTUP, detail) {
                    tracing::warn!(error = %e, "não foi possível registrar a inicialização");
                }
                let manager = JobManager::start(Arc::clone(&core), app.handle().clone());
                let watcher = ProjectWatcher::new(manager.clone());
                let parts = IndexingParts {
                    core: Arc::clone(&core),
                    manager: manager.clone(),
                    watcher,
                };
                parts.resync();
                // Mudanças feitas com o app fechado: uma passada incremental ao abrir.
                if core.settings().is_ok_and(|s| s.watch_projects) {
                    queue_all(&core, &manager, IndexTrigger::Reconcile);
                }
                start_reconciler(core, manager, Arc::clone(&stop_setup));
                parts
            });
            app.manage(Indexing::new(parts));
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
            commands::projects::list_projects,
            commands::projects::get_project_overview,
            commands::projects::pick_project_folder,
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::start_index,
            commands::projects::pause_index,
            commands::projects::resume_index,
            commands::projects::cancel_index,
            commands::projects::get_indexing_queue,
            commands::projects::list_jobs,
            commands::projects::get_job_errors,
            commands::projects::list_skipped_files,
            commands::projects::touch_project,
            commands::projects::reveal_project_dir,
            commands::context::search_project,
            commands::context::get_file_context,
            commands::context::compile_context_pack,
            commands::context::list_context_packs,
            commands::context::get_context_pack,
            commands::context::delete_context_pack,
            commands::context::render_context_pack,
            commands::context::save_context_pack,
            commands::memory::list_memories,
            commands::memory::get_memory,
            commands::memory::get_memory_counts,
            commands::memory::create_memory,
            commands::memory::update_memory,
            commands::memory::transition_memory,
            commands::memory::review_memory_refs,
            commands::memory::import_adrs,
            commands::memory::list_freshness_issues,
            commands::memory::get_freshness_summary,
            commands::memory::scan_freshness,
            commands::memory::set_issue_dismissed,
        ])
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(err) => {
            tracing::error!(error = %err, "erro fatal ao iniciar o Tauri");
            drop(log_guard);
            std::process::exit(1);
        }
    };
    app.run(move |handle, event| {
        if let tauri::RunEvent::Exit = event {
            stop.store(true, Ordering::SeqCst);
            if let Some(indexing) = handle.try_state::<Indexing>()
                && let Ok(parts) = indexing.parts()
            {
                // O job atual termina como "cancelado" (não "interrompido").
                parts.manager.shutdown(Duration::from_secs(5));
            }
        }
    });
    drop(log_guard);
}
