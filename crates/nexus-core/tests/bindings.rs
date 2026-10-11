//! Contratos TypeScript gerados a partir dos tipos Rust (ADR-0004).
//!
//! - `cargo test -p nexus-core --test bindings` falha se `apps/desktop/src/lib/bindings` estiver
//!   desatualizado.
//! - `NEXUS_UPDATE_BINDINGS=1 cargo test -p nexus-core --test bindings` regenera os arquivos.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use nexus_core::classify::FileKind;
use nexus_core::diagnostics::{
    AppInfo, BackupInfo, DiagnosticsReport, FileStat, IntegrityReport, StartupStatus,
};
use nexus_core::index::{
    IndexError, IndexJob, IndexMode, IndexTrigger, IndexingQueue, JobPhase, JobStatus,
};
use nexus_core::ops::{OperationQuery, OperationRecord, OperationStatus};
use nexus_core::projects::{
    ChangedFile, DirStat, FileCounts, KindStat, LanguageStat, Project, ProjectOverview,
    ProjectSummary, SkippedFile, StatusStat,
};
use nexus_core::settings::{MotionPreference, SettingKey, Settings, ThemePreference};
use nexus_core::storage::AppliedMigration;
use nexus_core::{ErrorCode, ErrorInfo};
use ts_rs::{Config, TS};

fn bindings_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src/lib/bindings")
}

fn render<T: TS + 'static>(cfg: &Config, out: &mut BTreeMap<String, String>) {
    let file = T::output_path().expect("tipo exportável");
    let body = T::export_to_string(cfg).expect("exportar tipo");
    out.insert(file.to_string_lossy().replace('\\', "/"), body);
}

fn generate() -> BTreeMap<String, String> {
    let cfg = Config::new()
        .with_large_int("number")
        .with_out_dir(bindings_dir());
    let mut files = BTreeMap::new();
    render::<ErrorCode>(&cfg, &mut files);
    render::<ErrorInfo>(&cfg, &mut files);
    render::<OperationStatus>(&cfg, &mut files);
    render::<OperationRecord>(&cfg, &mut files);
    render::<OperationQuery>(&cfg, &mut files);
    render::<SettingKey>(&cfg, &mut files);
    render::<ThemePreference>(&cfg, &mut files);
    render::<MotionPreference>(&cfg, &mut files);
    render::<Settings>(&cfg, &mut files);
    render::<AppliedMigration>(&cfg, &mut files);
    render::<AppInfo>(&cfg, &mut files);
    render::<FileStat>(&cfg, &mut files);
    render::<BackupInfo>(&cfg, &mut files);
    render::<DiagnosticsReport>(&cfg, &mut files);
    render::<IntegrityReport>(&cfg, &mut files);
    render::<StartupStatus>(&cfg, &mut files);
    render::<serde_json::Value>(&cfg, &mut files);
    render::<FileKind>(&cfg, &mut files);
    render::<Project>(&cfg, &mut files);
    render::<FileCounts>(&cfg, &mut files);
    render::<ProjectSummary>(&cfg, &mut files);
    render::<LanguageStat>(&cfg, &mut files);
    render::<KindStat>(&cfg, &mut files);
    render::<StatusStat>(&cfg, &mut files);
    render::<DirStat>(&cfg, &mut files);
    render::<ChangedFile>(&cfg, &mut files);
    render::<SkippedFile>(&cfg, &mut files);
    render::<ProjectOverview>(&cfg, &mut files);
    render::<IndexJob>(&cfg, &mut files);
    render::<IndexMode>(&cfg, &mut files);
    render::<IndexTrigger>(&cfg, &mut files);
    render::<JobStatus>(&cfg, &mut files);
    render::<JobPhase>(&cfg, &mut files);
    render::<IndexError>(&cfg, &mut files);
    render::<IndexingQueue>(&cfg, &mut files);

    let mut index = String::from("// Gerado por crates/nexus-core/tests/bindings.rs. Não edite.\n");
    for name in files.keys() {
        let module = name.trim_end_matches(".ts");
        index.push_str(&format!("export type * from \"./{module}\";\n"));
    }
    files.insert("index.ts".to_owned(), index);
    files
}

#[test]
fn bindings_are_up_to_date() {
    let dir = bindings_dir();
    let generated = generate();
    let update = std::env::var_os("NEXUS_UPDATE_BINDINGS").is_some();

    if update {
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in &generated {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        return;
    }

    let mut stale = Vec::new();
    for (name, body) in &generated {
        let current = std::fs::read_to_string(dir.join(name))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if &current != body {
            stale.push(name.clone());
        }
    }
    for name in files_on_disk(&dir, &dir) {
        if !generated.contains_key(&name) {
            stale.push(format!("{name} (órfão)"));
        }
    }
    assert!(
        stale.is_empty(),
        "bindings desatualizados: {stale:?}. Rode `NEXUS_UPDATE_BINDINGS=1 cargo test -p nexus-core --test bindings`."
    );
}

/// Arquivos existentes sob `dir`, como caminhos relativos a `root` com `/`.
fn files_on_disk(root: &std::path::Path, dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files_on_disk(root, &path));
        } else {
            let rel = path.strip_prefix(root).unwrap();
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
    out
}
