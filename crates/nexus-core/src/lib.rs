//! Núcleo do NEXUS. Toda a lógica de negócio vive aqui; desktop, CLI e MCP são adaptadores finos.

pub mod chunk;
pub mod classify;
pub mod context;
pub mod diagnostics;
pub mod error;
pub mod git;
pub mod index;
pub mod logging;
pub mod ops;
pub mod paths;
pub mod projects;
pub mod scan;
pub mod search;
pub mod sensitive;
pub mod settings;
pub mod storage;
pub mod time_util;

use std::path::Path;
use std::time::Instant;

pub use error::{Error, ErrorCode, ErrorInfo, Result};
pub use paths::DataPaths;
use serde_json::json;

use crate::context::export::{ExportFormat, ExportOptions};
use crate::context::{ContextPack, ContextPackRequest, ContextPackSummary};
use crate::diagnostics::{DiagnosticsReport, IntegrityReport};
use crate::index::{
    IndexError, IndexJob, IndexMode, IndexObserver, IndexOptions, IndexTrigger, JobControl,
    JobStatus,
};
use crate::ops::{OperationQuery, OperationRecord};
use crate::projects::{Project, ProjectOverview, ProjectSummary, SkippedFile};
use crate::search::evidence::FileContext;
use crate::search::{SearchRequest, SearchResponse};
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
            let recovered = ops::recover_interrupted(conn, now, ops::STALE_AFTER_MS)?
                + index::job::recover_stale(conn, None, now)?;
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

    // ---- Projetos -------------------------------------------------------------------------

    /// Autoriza uma pasta como projeto (validada e canonicalizada).
    pub fn add_project(&self, path: &Path) -> Result<Project> {
        let data_dir = self.paths.root().to_path_buf();
        ops::run(&self.db, ops::kinds::PROJECT_ADD, None, |_| {
            let project = self.db.write(|c| projects::add(c, path, &data_dir))?;
            let detail = json!({ "projectId": project.id });
            Ok((project, Some(detail)))
        })
    }

    pub fn project(&self, id: i64) -> Result<Project> {
        self.db.read(|c| projects::require(c, id))
    }

    /// Localiza por ID, nome ou caminho.
    pub fn find_project(&self, reference: &str) -> Result<Project> {
        self.db.read(|c| projects::find(c, reference))
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        self.db.read(projects::summaries)
    }

    pub fn project_summary(&self, id: i64) -> Result<ProjectSummary> {
        self.db
            .read(|c| projects::summary(c, projects::require(c, id)?))
    }

    pub fn project_overview(&self, id: i64) -> Result<ProjectOverview> {
        self.db.read(|c| projects::overview(c, id))
    }

    pub fn touch_project(&self, id: i64) -> Result<()> {
        self.db.write(|c| projects::touch(c, id))
    }

    /// Remove o projeto do índice. **Não** altera arquivos no disco.
    pub fn remove_project(&self, id: i64) -> Result<Project> {
        ops::run(&self.db, ops::kinds::PROJECT_REMOVE, Some(id), |_| {
            let project = self.db.write(|c| projects::remove(c, id))?;
            Ok((project, Some(json!({ "projectId": id }))))
        })
    }

    pub fn skipped_files(&self, id: i64, limit: u32) -> Result<Vec<SkippedFile>> {
        let limit = limit.clamp(1, 1000);
        self.db.read(|c| projects::skipped_files(c, id, limit))
    }

    // ---- Indexação ------------------------------------------------------------------------

    /// Opções de indexação a partir das configurações atuais.
    pub fn index_options(&self, mode: IndexMode, trigger: IndexTrigger) -> Result<IndexOptions> {
        Ok(IndexOptions::from_settings(
            &self.settings()?,
            mode,
            trigger,
        ))
    }

    /// Executa um job de indexação (bloqueante). Use `control` para pausar/cancelar de outra thread.
    pub fn index_project(
        &self,
        project_id: i64,
        opts: &IndexOptions,
        control: &JobControl,
        observer: &dyn IndexObserver,
    ) -> Result<IndexJob> {
        let project = self.project(project_id)?;
        let op = ops::start(&self.db, ops::kinds::INDEX_RUN, Some(project_id))?;
        let result = index::run(
            &self.db,
            project_id,
            Path::new(&project.root_path),
            op.id(),
            opts,
            control,
            observer,
        );
        let logged = match &result {
            Ok(job) if job.status == JobStatus::Cancelled => {
                ops::fail(&self.db, &op, &Error::Cancelled)
            }
            Ok(job) => ops::succeed(
                &self.db,
                &op,
                Some(&json!({
                    "mode": job.mode.as_str(),
                    "trigger": job.trigger.as_str(),
                    "discovered": job.discovered,
                    "processed": job.processed,
                    "unchanged": job.unchanged,
                    "newContents": job.new_contents,
                    "reusedContents": job.reused_contents,
                    "skipped": job.skipped,
                    "removed": job.removed,
                    "renamed": job.renamed,
                    "failed": job.failed,
                    "generation": job.generation,
                })),
            ),
            Err(e) => ops::fail(&self.db, &op, e),
        };
        if let Err(e) = logged {
            tracing::warn!(error = %e, "falha ao registrar a operação de indexação");
        }
        result.map_err(|e| Error::InOperation {
            op_id: op.id().to_owned(),
            source: Box::new(e),
        })
    }

    pub fn list_jobs(&self, project_id: i64, limit: u32) -> Result<Vec<IndexJob>> {
        self.db
            .read(|c| index::job::list(c, project_id, limit.clamp(1, 100)))
    }

    pub fn active_job(&self, project_id: i64) -> Result<Option<IndexJob>> {
        self.db.read(|c| index::job::active(c, project_id))
    }

    pub fn job_errors(&self, job_id: &str, limit: u32) -> Result<Vec<IndexError>> {
        self.db
            .read(|c| index::job::errors(c, job_id, limit.clamp(1, 1000)))
    }

    // ---- Busca e Context Packs ------------------------------------------------------------

    /// Busca híbrida no índice do projeto. Registra apenas estatísticas (nunca o texto da consulta).
    pub fn search(&self, req: &SearchRequest) -> Result<SearchResponse> {
        let project = self.project(req.project_id)?;
        let response = self.db.read(|c| search::search(c, &project, req))?;
        let detail = json!({
            "projectId": project.id,
            "mode": response.mode,
            "hits": response.hits.len(),
            "candidates": response.candidates,
            "tookMs": response.took_ms,
        });
        if let Err(e) = ops::record_completed(
            &self.db,
            ops::kinds::SEARCH,
            time_util::now_ms() - response.took_ms,
            response.took_ms,
            Some(&detail),
        ) {
            tracing::warn!(error = %e, "não foi possível registrar a busca");
        }
        Ok(response)
    }

    /// Trecho de um arquivo **indexado** (arquivos ignorados/sensíveis não podem ser lidos).
    pub fn file_context(
        &self,
        project_id: i64,
        rel_path: &str,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<FileContext> {
        let project = self.project(project_id)?;
        let root = std::path::PathBuf::from(&project.root_path);
        self.db
            .read(|c| search::evidence::file_context(c, &root, project_id, rel_path, start, end))
    }

    /// Compila, persiste (só referências) e devolve o pack materializado.
    pub fn compile_context_pack(&self, req: &ContextPackRequest) -> Result<ContextPack> {
        let project = self.project(req.project_id)?;
        ops::run(
            &self.db,
            ops::kinds::CONTEXT_COMPILE,
            Some(project.id),
            |_| {
                let plan = self.db.read(|c| context::compile(c, &project, req))?;
                self.db.write(|c| context::save(c, &plan))?;
                let detail = json!({
                    "packId": plan.id,
                    "sources": plan.sources.len(),
                    "omitted": plan.omitted.len(),
                    "estimatedTokens": plan.estimated_tokens,
                    "tokenBudget": plan.token_budget,
                });
                let pack = context::materialize(Path::new(&project.root_path), plan)?;
                Ok((pack, Some(detail)))
            },
        )
    }

    pub fn list_context_packs(
        &self,
        project_id: i64,
        limit: u32,
    ) -> Result<Vec<ContextPackSummary>> {
        self.db
            .read(|c| context::list(c, project_id, limit.clamp(1, 200)))
    }

    /// Reabre um pack: os trechos são lidos do disco agora e comparados com o hash da criação.
    pub fn context_pack(&self, id: &str) -> Result<ContextPack> {
        let plan = self.db.read(|c| context::load(c, id))?;
        let project = self.project(plan.project_id)?;
        context::materialize(Path::new(&project.root_path), plan)
    }

    pub fn delete_context_pack(&self, id: &str) -> Result<()> {
        self.db.write(|c| context::delete(c, id))
    }

    pub fn export_context_pack(
        &self,
        id: &str,
        format: ExportFormat,
        options: &ExportOptions,
    ) -> Result<String> {
        let pack = self.context_pack(id)?;
        context::export::render(&pack, format, options)
    }
}
