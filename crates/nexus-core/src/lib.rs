//! Núcleo do NEXUS. Toda a lógica de negócio vive aqui; desktop, CLI e MCP são adaptadores finos.

pub mod diagnostics;
pub mod error;
pub mod logging;
pub mod ops;
pub mod paths;
pub mod settings;
pub mod storage;
mod time_util;

use std::time::Instant;

pub use error::{Error, ErrorCode, ErrorInfo, Result};
pub use paths::DataPaths;
use serde_json::json;

use crate::diagnostics::{DiagnosticsReport, IntegrityReport};
use crate::ops::{OperationQuery, OperationRecord};
use crate::settings::{SettingKey, Settings};
use crate::storage::{BackupPolicy, Db, MigrationReport};

/// Versão do core (igual à do workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const BACKUPS_KEPT: usize = 5;

/// O que aconteceu ao abrir o core; útil para diagnóstico e para a CLI.
#[derive(Debug, Clone)]
pub struct OpenReport {
    pub migration: MigrationReport,
    pub recovered_operations: usize,
    pub pruned_operations: usize,
    pub op_id: String,
}

/// Ponto de entrada do core: diretório de dados + banco migrado.
#[derive(Debug)]
pub struct Core {
    paths: DataPaths,
    db: Db,
    open_report: OpenReport,
}

impl Core {
    /// Cria diretórios, abre o banco, aplica migrations (com backup quando necessário),
    /// recupera operações interrompidas e aplica a retenção.
    pub fn open(paths: DataPaths) -> Result<Self> {
        let started_at = time_util::now_ms();
        let started = Instant::now();
        paths.ensure()?;
        let policy = BackupPolicy {
            dir: paths.backups(),
            keep: BACKUPS_KEPT,
        };
        let (db, migration) = Db::open(&paths.db(), Some(&policy))?;

        let now = time_util::now_ms();
        let (recovered, pruned) = db.write(|conn| {
            let recovered = ops::recover_interrupted(conn, now, ops::STALE_AFTER_MS)?;
            let pruned = ops::prune(
                conn,
                now,
                ops::RETENTION_MAX_AGE_MS,
                ops::RETENTION_MAX_ROWS,
            )?;
            Ok((recovered, pruned))
        })?;

        let duration = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
        let detail = json!({
            "fromVersion": migration.from_version,
            "toVersion": migration.to_version,
            "appliedMigrations": migration.applied,
            "backupCreated": migration.backup.is_some(),
            "recoveredOperations": recovered,
            "prunedOperations": pruned,
        });
        let op_id = ops::record_completed(
            &db,
            ops::kinds::CORE_OPEN,
            started_at,
            duration,
            Some(&detail),
        )?;
        tracing::info!(
            op_id,
            data_dir = %paths.root().display(),
            schema_version = migration.to_version,
            applied = ?migration.applied,
            recovered,
            pruned,
            "core aberto"
        );

        Ok(Self {
            paths,
            db,
            open_report: OpenReport {
                migration,
                recovered_operations: recovered,
                pruned_operations: pruned,
                op_id,
            },
        })
    }

    pub fn paths(&self) -> &DataPaths {
        &self.paths
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn open_report(&self) -> &OpenReport {
        &self.open_report
    }

    pub fn settings(&self) -> Result<Settings> {
        self.db.read(settings::load)
    }

    pub fn set_setting(&self, key: SettingKey, value: serde_json::Value) -> Result<Settings> {
        ops::run(&self.db, ops::kinds::SETTINGS_UPDATE, None, |_| {
            let updated = self.db.write(|conn| settings::set(conn, key, value))?;
            // Apenas a chave é registrada; valores ficam fora do histórico.
            Ok((updated, Some(json!({ "key": key.as_str() }))))
        })
    }

    pub fn list_operations(&self, query: &OperationQuery) -> Result<Vec<OperationRecord>> {
        self.db.read(|conn| ops::list(conn, query))
    }

    pub fn diagnostics(&self) -> Result<DiagnosticsReport> {
        let (schema_version, migrations, fts5_available, operations_total, last_integrity_check) =
            self.db.read(diagnostics::schema_section)?;
        let db_path = self.paths.db();
        let wal_path = db_path.with_extension("db-wal");
        let (logs_size_bytes, log_files) = diagnostics::dir_usage(&self.paths.logs())?;
        Ok(DiagnosticsReport {
            core_version: VERSION.to_owned(),
            sqlite_version: storage::sqlite_version().to_owned(),
            fts5_available,
            data_dir: self.paths.root().display().to_string(),
            database: diagnostics::file_stat(&db_path),
            wal: diagnostics::file_stat(&wal_path),
            logs_dir: self.paths.logs().display().to_string(),
            logs_size_bytes,
            log_files,
            backups_dir: self.paths.backups().display().to_string(),
            backups: diagnostics::backups(&self.paths.backups())?,
            schema_version,
            latest_schema_version: storage::migrate::latest_version(),
            migrations,
            operations_total,
            last_integrity_check,
        })
    }

    /// Executa `PRAGMA integrity_check` como operação registrada.
    pub fn integrity_check(&self) -> Result<IntegrityReport> {
        ops::run(&self.db, ops::kinds::INTEGRITY_CHECK, None, |op| {
            let (ok, messages, duration_ms) = self.db.read(diagnostics::integrity_check)?;
            let detail = json!({ "ok": ok, "messages": messages.len() });
            Ok((
                IntegrityReport {
                    ok,
                    messages,
                    duration_ms,
                    op_id: op.id().to_owned(),
                },
                Some(detail),
            ))
        })
    }

    /// Registra uma operação pontual do adaptador (ex.: inicialização do app).
    pub fn record_event(&self, kind: &str, detail: serde_json::Value) -> Result<String> {
        ops::record_completed(&self.db, kind, time_util::now_ms(), 0, Some(&detail))
    }
}
