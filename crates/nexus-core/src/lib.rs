//! Núcleo do NEXUS. Toda a lógica de negócio vive aqui; desktop, CLI e MCP são adaptadores finos.

pub mod analysis;
pub mod chunk;
pub mod classify;
pub mod context;
pub mod diagnostics;
pub mod error;
pub mod freshness;
pub mod git;
pub mod index;
pub mod integration;
pub mod logging;
pub mod memory;
pub mod ops;
pub mod paths;
pub mod projects;
pub mod related;
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
use crate::freshness::{FreshnessIssue, FreshnessSummary, IssueQuery, ScanReport, ScanScope};
use crate::index::{
    IndexError, IndexJob, IndexMode, IndexObserver, IndexOptions, IndexTrigger, JobControl,
    JobStatus,
};
use crate::memory::adr::ImportReport;
use crate::memory::{
    Actor, Memory, MemoryCounts, MemoryDetail, MemoryQuery, MemoryUpdate, NewMemory, Transition,
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
        // O índice mudou: reavalia a atualidade (documentos, memória, arquivos). Uma falha aqui
        // não invalida a indexação — fica registrada como operação com erro.
        if let Ok(job) = &result
            && job.status == JobStatus::Succeeded
            && self.freshness_outdated(project_id).unwrap_or(true)
            && let Err(e) = self.scan_freshness(project_id)
        {
            tracing::warn!(error = %e, project_id, "verificação de atualidade após indexação falhou");
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

    /// Símbolos e dependências (imports e quem importa) de um arquivo indexado.
    pub fn file_analysis(
        &self,
        project_id: i64,
        rel_path: &str,
    ) -> Result<analysis::query::FileAnalysisView> {
        let rel = memory::normalize_rel_path(rel_path)?;
        self.db
            .read(|c| analysis::query::file_view(c, project_id, &rel))
    }

    /// Cobertura dos analisadores e estado do grafo do projeto.
    pub fn analysis_overview(&self, project_id: i64) -> Result<analysis::query::AnalysisOverview> {
        self.project(project_id)?;
        self.db.read(|c| analysis::query::overview(c, project_id))
    }

    /// Declarações com um nome (comparação normalizada).
    pub fn find_symbols(
        &self,
        project_id: i64,
        name: &str,
        limit: u32,
    ) -> Result<Vec<analysis::query::SymbolHit>> {
        self.project(project_id)?;
        self.db.read(|c| {
            analysis::query::find_symbols(c, project_id, name, limit.clamp(1, 100) as usize)
        })
    }

    /// Arquivos relacionados por heurística (testes, pasta, manifesto, citações) e memórias.
    pub fn related_files(&self, project_id: i64, rel_path: &str) -> Result<related::RelatedFiles> {
        let project = self.project(project_id)?;
        self.db
            .read(|c| related::related_files(c, &project, rel_path))
    }

    /// Declarações e usos prováveis de um identificador.
    pub fn symbol_context(&self, project_id: i64, symbol: &str) -> Result<related::SymbolContext> {
        let project = self.project(project_id)?;
        self.db
            .read(|c| related::symbol_context(c, &project, symbol))
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
                let pack = self.db.read(|c| context::attach_memory(c, pack))?;
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
        let pack = context::materialize(Path::new(&project.root_path), plan)?;
        self.db.read(|c| context::attach_memory(c, pack))
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

    // ---- Memória --------------------------------------------------------------------------

    pub fn create_memory(&self, req: &NewMemory, actor: Actor) -> Result<Memory> {
        self.project(req.project_id)?;
        let m = ops::run(
            &self.db,
            ops::kinds::MEMORY_CREATE,
            Some(req.project_id),
            |_| {
                let m = self.db.write(|c| memory::create(c, req, actor))?;
                let detail = memory_detail(&m, actor);
                Ok((m, Some(detail)))
            },
        )?;
        self.rescan_memories(m.project_id);
        Ok(m)
    }

    pub fn update_memory(&self, id: &str, req: &MemoryUpdate, actor: Actor) -> Result<Memory> {
        let project_id = self.db.read(|c| memory::get(c, id))?.project_id;
        let m = ops::run(
            &self.db,
            ops::kinds::MEMORY_UPDATE,
            Some(project_id),
            |_| {
                let m = self.db.write(|c| memory::update(c, id, req, actor))?;
                let detail = memory_detail(&m, actor);
                Ok((m, Some(detail)))
            },
        )?;
        self.rescan_memories(project_id);
        Ok(m)
    }

    /// Confirmar, arquivar ou restaurar (somente quem revisa).
    pub fn transition_memory(
        &self,
        id: &str,
        to: Transition,
        expected_revision: Option<i64>,
        note: Option<&str>,
        actor: Actor,
    ) -> Result<Memory> {
        let project_id = self.db.read(|c| memory::get(c, id))?.project_id;
        let m = ops::run(
            &self.db,
            ops::kinds::MEMORY_TRANSITION,
            Some(project_id),
            |_| {
                let m = self
                    .db
                    .write(|c| memory::transition(c, id, to, expected_revision, note, actor))?;
                let mut detail = memory_detail(&m, actor);
                detail["transition"] = json!(to);
                Ok((m, Some(detail)))
            },
        )?;
        self.rescan_memories(project_id);
        Ok(m)
    }

    /// Registra a versão atual dos arquivos referenciados (e corrige caminhos movidos).
    pub fn review_memory_refs(
        &self,
        id: &str,
        expected_revision: i64,
        actor: Actor,
    ) -> Result<Memory> {
        let project_id = self.db.read(|c| memory::get(c, id))?.project_id;
        let project = self.project(project_id)?;
        let root = std::path::PathBuf::from(&project.root_path);
        let m = ops::run(
            &self.db,
            ops::kinds::MEMORY_UPDATE,
            Some(project_id),
            |_| {
                let m = self
                    .db
                    .write(|c| memory::review_refs(c, &root, id, expected_revision, actor))?;
                let mut detail = memory_detail(&m, actor);
                detail["refsReviewed"] = json!(true);
                Ok((m, Some(detail)))
            },
        )?;
        self.rescan_memories(project_id);
        Ok(m)
    }

    pub fn memory(&self, id: &str) -> Result<MemoryDetail> {
        self.db.read(|c| {
            let m = memory::get(c, id)?;
            let project = projects::require(c, m.project_id)?;
            memory::detail(c, &project, id)
        })
    }

    /// ID completo a partir do ID ou de um prefixo único.
    pub fn resolve_memory_id(&self, reference: &str) -> Result<String> {
        self.db.read(|c| memory::resolve_id(c, reference))
    }

    pub fn list_memories(&self, query: &MemoryQuery) -> Result<Vec<Memory>> {
        self.project(query.project_id)?;
        self.db.read(|c| memory::list(c, query))
    }

    pub fn memory_counts(&self, project_id: i64) -> Result<MemoryCounts> {
        self.db.read(|c| memory::counts(c, project_id))
    }

    /// Importa os ADRs indexados do projeto como propostas.
    pub fn import_adrs(&self, project_id: i64) -> Result<ImportReport> {
        let project = self.project(project_id)?;
        let root = std::path::PathBuf::from(&project.root_path);
        let report = ops::run(
            &self.db,
            ops::kinds::MEMORY_IMPORT,
            Some(project_id),
            |_| {
                let report = self
                    .db
                    .write(|c| memory::adr::import(c, &root, project_id))?;
                let detail = json!({
                    "scanned": report.scanned,
                    "created": report.created,
                    "updated": report.updated,
                    "superseding": report.superseding,
                    "unchanged": report.unchanged,
                    "skippedInactive": report.skipped_inactive,
                    "skippedStale": report.skipped_stale.len(),
                });
                Ok((report, Some(detail)))
            },
        )?;
        self.rescan_memories(project_id);
        Ok(report)
    }

    // ---- Atualidade -----------------------------------------------------------------------

    /// Verificação completa de atualidade, registrada como operação.
    pub fn scan_freshness(&self, project_id: i64) -> Result<ScanReport> {
        let project = self.project(project_id)?;
        ops::run(
            &self.db,
            ops::kinds::FRESHNESS_SCAN,
            Some(project_id),
            |_| {
                let outcome = self
                    .db
                    .read(|c| freshness::scan(c, &project, ScanScope::FULL))?;
                let report = self.db.write(|c| freshness::apply(c, outcome))?;
                let detail = json!({
                    "projectId": project_id,
                    "detected": report.detected,
                    "newIssues": report.new_issues,
                    "resolved": report.resolved,
                    "docsChecked": report.info.docs_checked,
                    "memoriesChecked": report.info.memories_checked,
                    "durationMs": report.info.duration_ms,
                });
                Ok((report, Some(detail)))
            },
        )
    }

    /// Reavalia só os alertas de memória (rápido; chamado depois de cada mudança na memória).
    fn rescan_memories(&self, project_id: i64) {
        let result = self.project(project_id).and_then(|project| {
            let outcome = self
                .db
                .read(|c| freshness::scan(c, &project, ScanScope::MEMORIES))?;
            self.db.write(|c| freshness::apply(c, outcome))
        });
        if let Err(e) = result {
            tracing::warn!(error = %e, project_id, "reavaliação dos alertas de memória falhou");
        }
    }

    fn freshness_outdated(&self, project_id: i64) -> Result<bool> {
        let project = self.project(project_id)?;
        let last = self.db.read(|c| freshness::last_scan(c, project_id))?;
        Ok(last.is_none_or(|s| s.index_generation < project.index_generation))
    }

    pub fn freshness_issues(&self, query: &IssueQuery) -> Result<Vec<FreshnessIssue>> {
        self.project(query.project_id)?;
        self.db.read(|c| freshness::list(c, query))
    }

    pub fn freshness_summary(&self, project_id: i64) -> Result<FreshnessSummary> {
        let project = self.project(project_id)?;
        self.db.read(|c| freshness::summary(c, &project))
    }

    /// Dispensa (`true`) ou reabre (`false`) um alerta.
    pub fn set_issue_dismissed(
        &self,
        issue_id: i64,
        dismissed: bool,
        note: Option<&str>,
    ) -> Result<FreshnessIssue> {
        self.db
            .write(|c| freshness::set_dismissed(c, issue_id, dismissed, note))
    }
}

/// Detalhe registrado nas operações de memória: identificadores e estado, nunca o texto.
fn memory_detail(m: &Memory, actor: Actor) -> serde_json::Value {
    json!({
        "memoryId": m.id,
        "projectId": m.project_id,
        "kind": m.kind,
        "status": m.status,
        "revision": m.revision,
        "actor": actor,
    })
}
