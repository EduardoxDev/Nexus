//! Relatório de diagnóstico local e verificação de integridade. Tudo aqui é medido, nada estimado.

use std::path::Path;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::ops::{self, OperationQuery, OperationRecord, OperationStatus};
use crate::storage::{self, AppliedMigration, migrate};

/// Informações do executável, preenchidas pelo adaptador (desktop/CLI).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub core_version: String,
    /// `debug` ou `release`.
    pub build_profile: String,
    pub os: String,
    pub arch: String,
    /// Versão do WebView (WebView2 no Windows); ausente fora do desktop.
    pub webview_version: Option<String>,
    pub tauri_version: Option<String>,
}

impl AppInfo {
    /// Campos que não dependem do adaptador.
    pub fn base(app_version: &str) -> Self {
        Self {
            app_version: app_version.to_owned(),
            core_version: crate::VERSION.to_owned(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_owned(),
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            webview_version: None,
            tauri_version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub exists: bool,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub core_version: String,
    pub sqlite_version: String,
    pub fts5_available: bool,
    pub data_dir: String,
    pub database: FileStat,
    pub wal: FileStat,
    pub logs_dir: String,
    pub logs_size_bytes: i64,
    pub log_files: i64,
    pub backups_dir: String,
    pub backups: Vec<BackupInfo>,
    pub schema_version: i64,
    pub latest_schema_version: i64,
    pub migrations: Vec<AppliedMigration>,
    pub operations_total: i64,
    pub last_integrity_check: Option<OperationRecord>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub ok: bool,
    /// Mensagens do `PRAGMA integrity_check` (no máximo 100).
    pub messages: Vec<String>,
    pub duration_ms: i64,
    pub op_id: String,
}

pub(crate) fn file_stat(path: &Path) -> FileStat {
    let meta = std::fs::metadata(path).ok();
    FileStat {
        path: path.display().to_string(),
        exists: meta.is_some(),
        size_bytes: meta.map_or(0, |m| i64::try_from(m.len()).unwrap_or(i64::MAX)),
    }
}

/// Soma tamanhos e conta arquivos de um diretório (não recursivo).
pub(crate) fn dir_usage(dir: &Path) -> Result<(i64, i64)> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((0, 0)),
        Err(e) => return Err(Error::io(format!("listar {}", dir.display()), e)),
    };
    let (mut size, mut files) = (0i64, 0i64);
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata()
            && meta.is_file()
        {
            size = size.saturating_add(i64::try_from(meta.len()).unwrap_or(i64::MAX));
            files += 1;
        }
    }
    Ok((size, files))
}

pub(crate) fn backups(dir: &Path) -> Result<Vec<BackupInfo>> {
    Ok(migrate::list_backups(dir)?
        .into_iter()
        .map(|(path, meta)| BackupInfo {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size_bytes: i64::try_from(meta.len()).unwrap_or(i64::MAX),
            modified_at: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| i64::try_from(d.as_millis()).ok()),
        })
        .collect())
}

/// (versão do schema, migrations, FTS5 disponível, total de operações, última verificação).
pub(crate) type SchemaSection = (
    i64,
    Vec<AppliedMigration>,
    bool,
    i64,
    Option<OperationRecord>,
);

pub(crate) fn schema_section(conn: &Connection) -> Result<SchemaSection> {
    let migrations = migrate::applied_migrations(conn)?;
    let version = migrations.last().map_or(0, |m| m.version);
    let fts5 = storage::fts5_available(conn)?;
    let total = ops::count(conn)?;
    let last_check = ops::list(
        conn,
        &OperationQuery {
            limit: Some(1),
            kind_prefix: Some(ops::kinds::INTEGRITY_CHECK.to_owned()),
            status: None,
        },
    )?
    .into_iter()
    .next();
    Ok((version, migrations, fts5, total, last_check))
}

/// Executa `PRAGMA integrity_check` e devolve as mensagens (sem `op_id`; preenchido pelo chamador).
pub(crate) fn integrity_check(conn: &Connection) -> Result<(bool, Vec<String>, i64)> {
    let started = Instant::now();
    let mut stmt = conn.prepare("PRAGMA integrity_check(100)")?;
    let messages: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<_, _>>()?;
    let ok = messages.len() == 1 && messages[0] == "ok";
    let duration = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    Ok((ok, messages, duration))
}

/// Status textual curto para relatórios humanos.
pub fn status_label(status: OperationStatus) -> &'static str {
    match status {
        OperationStatus::Running => "em execução",
        OperationStatus::Succeeded => "concluída",
        OperationStatus::Failed => "falhou",
        OperationStatus::Cancelled => "cancelada",
        OperationStatus::Interrupted => "interrompida",
    }
}
