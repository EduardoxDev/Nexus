//! Indexador incremental.
//!
//! Fluxo: varredura (sem banco) → comparação com o índice (tamanho, mtime, versão) → processamento
//! paralelo em lotes (leitura estável, exclusões, hash, chunks) → gravação de cada lote numa
//! transação curta → remoções, renomeações e coleta de conteúdo órfão → nova geração do índice.
//!
//! Cada lote gravado é um checkpoint: se o processo morrer, a próxima execução pula o que já foi
//! gravado (mesmo tamanho/mtime) e o job antigo é marcado como interrompido.

pub mod control;
pub mod job;
mod process;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use rusqlite::{OptionalExtension, Transaction, params};

pub use control::JobControl;
pub use job::{IndexError, IndexJob, IndexMode, IndexTrigger, IndexingQueue, JobPhase, JobStatus};
use process::{Outcome, Processed};

use crate::chunk::{self, Chunk, INDEXER_VERSION};
use crate::error::{Error, Result};
use crate::scan::{self, FileEntry};
use crate::storage::Db;
use crate::time_util::now_ms;

const HEARTBEAT_EVERY: Duration = Duration::from_secs(2);
const RETRY_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct IndexOptions {
    pub mode: IndexMode,
    pub trigger: IndexTrigger,
    /// Threads de processamento; 0 = metade dos núcleos (mínimo 1).
    pub threads: usize,
    pub max_file_size: u64,
    pub batch_size: usize,
}

impl IndexOptions {
    pub fn new(mode: IndexMode, trigger: IndexTrigger) -> Self {
        Self {
            mode,
            trigger,
            threads: 0,
            max_file_size: 1024 * 1024,
            batch_size: 64,
        }
    }

    pub fn from_settings(
        settings: &crate::settings::Settings,
        mode: IndexMode,
        trigger: IndexTrigger,
    ) -> Self {
        Self {
            threads: settings.indexing_threads as usize,
            max_file_size: u64::from(settings.max_file_size_kb) * 1024,
            ..Self::new(mode, trigger)
        }
    }

    fn effective_threads(&self) -> usize {
        if self.threads > 0 {
            return self.threads.min(32);
        }
        std::thread::available_parallelism()
            .map(|n| (n.get() / 2).max(1))
            .unwrap_or(1)
    }
}

/// Recebe o estado do job a cada checkpoint (progresso). Implementações devem ser rápidas.
pub trait IndexObserver: Send + Sync {
    fn on_update(&self, job: &IndexJob);
}

pub struct NoopObserver;

impl IndexObserver for NoopObserver {
    fn on_update(&self, _: &IndexJob) {}
}

/// Estado do arquivo já indexado.
struct Known {
    size: i64,
    mtime_ns: i64,
    hash: Option<String>,
    status: String,
    indexer_version: i64,
}

impl Known {
    fn is_unchanged(&self, entry: &FileEntry, max_file_size: u64) -> bool {
        let same_stat = self.size == i64::try_from(entry.size).unwrap_or(i64::MAX)
            && self.mtime_ns == entry.mtime_ns
            && self.indexer_version == INDEXER_VERSION;
        // Arquivos grandes demais voltam a ser processados se o limite aumentou.
        let still_too_large = self.status != "skipped_large" || entry.size > max_file_size;
        same_stat && self.status != "error" && still_too_large
    }
}

struct Runner<'a> {
    db: &'a Db,
    root: PathBuf,
    project_id: i64,
    job: IndexJob,
    opts: &'a IndexOptions,
    control: &'a JobControl,
    observer: &'a dyn IndexObserver,
    last_save: Instant,
    /// Arquivos cujo conteúdo/estado mudou (decide se há nova geração).
    changed: i64,
    /// hash → caminhos que não existiam no índice antes deste job (para detectar renomeações).
    added_by_hash: HashMap<String, Vec<String>>,
}

impl Runner<'_> {
    /// Notifica o observador a cada lote; grava no banco (heartbeat) no máximo a cada 2 s,
    /// salvo quando `force`. Os dados dos arquivos já foram gravados pelo lote.
    fn checkpoint(&mut self, force: bool) -> Result<()> {
        if force || self.last_save.elapsed() >= HEARTBEAT_EVERY {
            self.job.heartbeat_at = now_ms();
            let job = self.job.clone();
            self.db.write(|c| job::save(c, &job))?;
            self.last_save = Instant::now();
        }
        self.observer.on_update(&self.job);
        Ok(())
    }

    /// Bloqueia enquanto pausado; `Err(Cancelled)` se cancelado.
    fn check_control(&mut self) -> Result<()> {
        if self.control.is_paused() {
            let db = self.db;
            let observer = self.observer;
            let job = &mut self.job;
            let cancelled = self.control.wait_while_paused(&mut |paused| {
                job.status = if paused {
                    JobStatus::Paused
                } else {
                    JobStatus::Running
                };
                job.heartbeat_at = now_ms();
                if let Err(e) = db.write(|c| job::save(c, job)) {
                    tracing::warn!(error = %e, "falha ao gravar heartbeat durante pausa");
                }
                observer.on_update(job);
            });
            if cancelled {
                return Err(Error::Cancelled);
            }
        }
        if self.control.is_cancelled() {
            return Err(Error::Cancelled);
        }
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        // 1. Varredura — nenhuma conexão é mantida durante o IO.
        let root_meta = std::fs::metadata(&self.root);
        if !root_meta.is_ok_and(|m| m.is_dir()) {
            return Err(Error::NotFound(format!(
                "a pasta do projeto não existe mais: {}",
                self.root.display()
            )));
        }
        let scan_beat = std::cell::Cell::new(Instant::now());
        let (db, job_id, control) = (self.db, self.job.id.clone(), self.control);
        let outcome = scan::scan(&self.root, &|| {
            if scan_beat.get().elapsed() >= HEARTBEAT_EVERY {
                scan_beat.set(Instant::now());
                let _ = db.write(|c| {
                    c.execute(
                        "UPDATE indexing_jobs SET heartbeat_at = ?2 WHERE id = ?1",
                        params![job_id, now_ms()],
                    )?;
                    Ok(())
                });
            }
            control.is_cancelled()
        })?;
        for (path, message) in &outcome.errors {
            let (id, pid) = (self.job.id.clone(), self.project_id);
            self.db
                .write(|c| job::record_error(c, &id, pid, path, "scan", message, now_ms()))?;
        }

        // 2. Comparação com o índice.
        let known = self.db.read(|c| load_known(c, self.project_id))?;
        let mut to_process = Vec::new();
        let mut unchanged = 0i64;
        for entry in &outcome.files {
            let skip = self.opts.mode == IndexMode::Incremental
                && known
                    .get(&entry.rel_path)
                    .is_some_and(|k| k.is_unchanged(entry, self.opts.max_file_size));
            if skip {
                unchanged += 1;
            } else {
                to_process.push(entry.clone());
            }
        }
        self.job.discovered = count(outcome.files.len());
        self.job.to_process = count(to_process.len());
        self.job.unchanged = unchanged;
        self.job.phase = JobPhase::Indexing;
        self.checkpoint(true)?;

        // 3. Processamento em lotes.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.opts.effective_threads())
            .thread_name(|i| format!("nexus-index-{i}"))
            .build()
            .map_err(|e| {
                Error::Internal(format!("não foi possível criar o pool de indexação: {e}"))
            })?;
        let mut unstable = Vec::new();
        for batch in to_process.chunks(self.opts.batch_size.max(1)) {
            self.check_control()?;
            let (root, max, control) = (&self.root, self.opts.max_file_size, self.control);
            let results: Vec<Processed> = pool.install(|| {
                batch
                    .par_iter()
                    .filter(|_| !control.is_cancelled())
                    .map(|entry| process::process(root, entry, max))
                    .collect()
            });
            self.check_control()?;
            unstable.extend(self.write_batch(results, &known)?);
            self.checkpoint(false)?;
        }

        // 4. Arquivos que mudaram durante a leitura: uma nova tentativa.
        if !unstable.is_empty() {
            std::thread::sleep(RETRY_DELAY);
            let mut retried = Vec::new();
            for entry in unstable {
                let fresh = scan::resolve_rel(&self.root, &entry.rel_path)
                    .ok()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| FileEntry {
                        rel_path: entry.rel_path.clone(),
                        size: m.len(),
                        mtime_ns: scan::mtime_ns(&m),
                    });
                if let Some(fresh) = fresh {
                    retried.push(process::process(
                        &self.root,
                        &fresh,
                        self.opts.max_file_size,
                    ));
                }
            }
            let still_unstable = self.write_batch(retried, &known)?;
            for entry in still_unstable {
                self.job.failed += 1;
                let (id, pid) = (self.job.id.clone(), self.project_id);
                self.db.write(|c| {
                    job::record_error(
                        c,
                        &id,
                        pid,
                        &entry.rel_path,
                        "unstable",
                        "o arquivo mudou durante a leitura; será tentado na próxima indexação",
                        now_ms(),
                    )
                })?;
            }
        }

        // 5. Remoções, renomeações, coleta de órfãos e nova geração.
        self.check_control()?;
        self.job.phase = JobPhase::Finalizing;
        self.checkpoint(true)?;
        let seen: HashSet<&str> = outcome.files.iter().map(|f| f.rel_path.as_str()).collect();
        let removed: Vec<(String, Option<String>)> = known
            .iter()
            .filter(|(path, _)| !seen.contains(path.as_str()))
            .map(|(path, k)| (path.clone(), k.hash.clone()))
            .collect();
        let mut renamed = 0i64;
        for (_, hash) in &removed {
            if let Some(targets) = hash.as_ref().and_then(|h| self.added_by_hash.get_mut(h))
                && targets.pop().is_some()
            {
                renamed += 1;
            }
        }
        let project_id = self.project_id;
        let root = self.root.clone();
        let head = crate::git::read_head(&root);
        let changed = self.changed + count(removed.len());
        let generation = self.db.write(|conn| {
            let tx = conn.transaction()?;
            for (path, _) in &removed {
                tx.execute(
                    "DELETE FROM files WHERE project_id = ?1 AND rel_path = ?2",
                    params![project_id, path],
                )?;
            }
            tx.execute(
                "DELETE FROM contents WHERE project_id = ?1 AND NOT EXISTS (
                    SELECT 1 FROM files f
                    WHERE f.project_id = contents.project_id AND f.content_hash = contents.content_hash)",
                [project_id],
            )?;
            let bump = i64::from(changed > 0);
            tx.execute(
                "UPDATE projects SET index_generation = index_generation + ?2, last_indexed_at = ?3,
                    head_ref = ?4, head_commit = ?5
                 WHERE id = ?1",
                params![
                    project_id,
                    bump,
                    now_ms(),
                    head.as_ref().and_then(|h| h.reference.clone()),
                    head.as_ref().and_then(|h| h.commit.clone())
                ],
            )?;
            let generation: i64 = tx.query_row(
                "SELECT index_generation FROM projects WHERE id = ?1",
                [project_id],
                |r| r.get(0),
            )?;
            tx.commit()?;
            Ok(generation)
        })?;
        self.job.removed = count(removed.len());
        self.job.renamed = renamed;
        self.job.generation = Some(generation);
        // 6. Grafo de dependências: resolvido de novo quando o índice mudou (ou nunca foi feito).
        if self
            .db
            .read(|c| crate::analysis::graph::needs_rebuild(c, project_id, generation))?
        {
            crate::analysis::graph::rebuild(self.db, project_id, &root, generation)?;
        }
        Ok(())
    }

    /// Grava um lote numa transação. Devolve os arquivos instáveis (para nova tentativa).
    fn write_batch(
        &mut self,
        results: Vec<Processed>,
        known: &HashMap<String, Known>,
    ) -> Result<Vec<FileEntry>> {
        let project_id = self.project_id;
        let job_id = self.job.id.clone();
        let mut unstable = Vec::new();
        let mut stats = BatchStats::default();
        self.db.write(|conn| {
            let tx = conn.transaction()?;
            let now = now_ms();
            for p in results {
                let previous = known.get(&p.entry.rel_path);
                let (status, hash, reason, lines) = match &p.outcome {
                    Outcome::Unstable => {
                        unstable.push(p.entry.clone());
                        continue;
                    }
                    Outcome::Indexed {
                        hash,
                        line_count,
                        chunks,
                        bytes_read,
                        analysis,
                    } => {
                        stats.bytes_read += i64::try_from(*bytes_read).unwrap_or(i64::MAX);
                        if ensure_content(
                            &tx,
                            project_id,
                            hash,
                            *line_count,
                            chunks,
                            analysis.as_ref(),
                            now,
                        )? {
                            stats.new_contents += 1;
                        } else {
                            stats.reused_contents += 1;
                        }
                        if previous.is_none() {
                            stats.added.push((hash.clone(), p.entry.rel_path.clone()));
                        }
                        ("indexed", Some(hash.clone()), None, Some(*line_count))
                    }
                    Outcome::Skipped {
                        status,
                        reason,
                        bytes_read,
                    } => {
                        stats.bytes_read += i64::try_from(*bytes_read).unwrap_or(i64::MAX);
                        stats.skipped += 1;
                        (status.as_str(), None, Some(reason.clone()), None)
                    }
                    Outcome::Failed { code, message } => {
                        stats.failed += 1;
                        job::record_error(
                            &tx,
                            &job_id,
                            project_id,
                            &p.entry.rel_path,
                            code,
                            message,
                            now,
                        )?;
                        ("error", None, Some(message.clone()), None)
                    }
                };
                let is_change = previous
                    .is_none_or(|k| k.hash.as_deref() != hash.as_deref() || k.status != status);
                if is_change {
                    stats.changed += 1;
                }
                upsert_file(
                    &tx,
                    project_id,
                    &p,
                    status,
                    hash.as_deref(),
                    reason.as_deref(),
                    lines,
                    now,
                )?;
                stats.processed += 1;
            }
            tx.commit()?;
            Ok(())
        })?;
        self.job.processed += stats.processed;
        self.job.new_contents += stats.new_contents;
        self.job.reused_contents += stats.reused_contents;
        self.job.skipped += stats.skipped;
        self.job.failed += stats.failed;
        self.job.bytes_read += stats.bytes_read;
        self.changed += stats.changed;
        for (hash, path) in stats.added {
            self.added_by_hash.entry(hash).or_default().push(path);
        }
        Ok(unstable)
    }
}

#[derive(Default)]
struct BatchStats {
    processed: i64,
    new_contents: i64,
    reused_contents: i64,
    skipped: i64,
    failed: i64,
    bytes_read: i64,
    changed: i64,
    added: Vec<(String, String)>,
}

fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn load_known(conn: &rusqlite::Connection, project_id: i64) -> Result<HashMap<String, Known>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, size, mtime_ns, content_hash, status, indexer_version
         FROM files WHERE project_id = ?1",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            Known {
                size: r.get(1)?,
                mtime_ns: r.get(2)?,
                hash: r.get(3)?,
                status: r.get(4)?,
                indexer_version: r.get(5)?,
            },
        ))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Garante o conteúdo (chunks + FTS). Devolve `true` se foi indexado agora.
fn ensure_content(
    tx: &Transaction<'_>,
    project_id: i64,
    hash: &str,
    line_count: i64,
    chunks: &[Chunk],
    analysis: Option<&crate::analysis::FileAnalysis>,
    now: i64,
) -> Result<bool> {
    let existing: Option<i64> = tx
        .query_row(
            "SELECT indexer_version FROM contents WHERE project_id = ?1 AND content_hash = ?2",
            params![project_id, hash],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        Some(version) if version == INDEXER_VERSION => return Ok(false),
        Some(_) => {
            // Versão antiga do indexador: refaz os chunks (o gatilho limpa o FTS).
            tx.execute(
                "DELETE FROM chunks WHERE project_id = ?1 AND content_hash = ?2",
                params![project_id, hash],
            )?;
            tx.execute(
                "UPDATE contents SET line_count = ?3, chunk_count = ?4, indexer_version = ?5
                 WHERE project_id = ?1 AND content_hash = ?2",
                params![
                    project_id,
                    hash,
                    line_count,
                    count(chunks.len()),
                    INDEXER_VERSION
                ],
            )?;
        }
        None => {
            tx.execute(
                "INSERT INTO contents (project_id, content_hash, line_count, chunk_count, indexer_version, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![project_id, hash, line_count, count(chunks.len()), INDEXER_VERSION, now],
            )?;
        }
    }
    let mut insert_chunk = tx.prepare_cached(
        "INSERT INTO chunks (project_id, content_hash, ordinal, start_line, end_line, chunk_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut insert_fts =
        tx.prepare_cached("INSERT INTO chunks_fts (rowid, body) VALUES (?1, ?2)")?;
    for (ordinal, c) in chunks.iter().enumerate() {
        insert_chunk.execute(params![
            project_id,
            hash,
            count(ordinal),
            c.start_line,
            c.end_line,
            blake3::hash(c.text.as_bytes()).to_hex().to_string()
        ])?;
        let id = tx.last_insert_rowid();
        insert_fts.execute(params![id, chunk::fts_body(&c.text)])?;
    }
    if let Some(a) = analysis {
        crate::analysis::graph::store_analysis(tx, project_id, hash, a)?;
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn upsert_file(
    tx: &Transaction<'_>,
    project_id: i64,
    p: &Processed,
    status: &str,
    hash: Option<&str>,
    reason: Option<&str>,
    lines: Option<i64>,
    now: i64,
) -> Result<()> {
    tx.prepare_cached(
        "INSERT INTO files (project_id, rel_path, size, mtime_ns, content_hash, language, kind, status,
                            skip_reason, line_count, first_seen_at, indexed_at, change_count, indexer_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 0, ?12)
         ON CONFLICT (project_id, rel_path) DO UPDATE SET
            change_count = files.change_count + (CASE
                WHEN files.content_hash IS NOT excluded.content_hash OR files.status <> excluded.status
                THEN 1 ELSE 0 END),
            size = excluded.size, mtime_ns = excluded.mtime_ns, content_hash = excluded.content_hash,
            language = excluded.language, kind = excluded.kind, status = excluded.status,
            skip_reason = excluded.skip_reason, line_count = excluded.line_count,
            indexed_at = excluded.indexed_at, indexer_version = excluded.indexer_version",
    )?
    .execute(params![
        project_id,
        p.entry.rel_path,
        i64::try_from(p.entry.size).unwrap_or(i64::MAX),
        p.entry.mtime_ns,
        hash,
        p.meta.language,
        p.meta.kind.as_str(),
        status,
        reason,
        lines,
        now,
        INDEXER_VERSION
    ])?;
    Ok(())
}

/// Executa um job de indexação completo para o projeto (bloqueante).
/// O resultado é o estado final do job; cancelamento devolve `Ok` com status `cancelled`.
pub(crate) fn run(
    db: &Db,
    project_id: i64,
    root: &Path,
    job_id: &str,
    opts: &IndexOptions,
    control: &JobControl,
    observer: &dyn IndexObserver,
) -> Result<IndexJob> {
    let job =
        db.write(|c| job::acquire(c, job_id, project_id, opts.mode, opts.trigger, now_ms()))?;
    tracing::info!(
        job_id,
        project_id,
        mode = opts.mode.as_str(),
        trigger = opts.trigger.as_str(),
        "indexação iniciada"
    );
    observer.on_update(&job);
    let mut runner = Runner {
        db,
        root: root.to_path_buf(),
        project_id,
        job,
        opts,
        control,
        observer,
        last_save: Instant::now(),
        changed: 0,
        added_by_hash: HashMap::new(),
    };
    let result = runner.run();
    let mut job = runner.job;
    job.finished_at = Some(now_ms());
    job.heartbeat_at = now_ms();
    job.phase = JobPhase::Done;
    let outcome = match &result {
        Ok(()) => {
            job.status = JobStatus::Succeeded;
            Ok(())
        }
        Err(Error::Cancelled) => {
            job.status = JobStatus::Cancelled;
            job.error_message = Some("Cancelada pelo usuário.".into());
            Ok(())
        }
        Err(e) => {
            job.status = JobStatus::Failed;
            job.error_message = Some(e.user_message());
            Err(())
        }
    };
    db.write(|c| job::save(c, &job))?;
    // Devolve as páginas do WAL ao banco principal sem bloquear leitores (PASSIVE).
    if let Err(e) = db.write(|c| Ok(c.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?)) {
        tracing::warn!(error = %e, "checkpoint do WAL não executado");
    }
    observer.on_update(&job);
    tracing::info!(
        job_id,
        status = job.status.as_str(),
        discovered = job.discovered,
        processed = job.processed,
        unchanged = job.unchanged,
        removed = job.removed,
        failed = job.failed,
        "indexação terminou"
    );
    match (outcome, result) {
        (Ok(()), _) => Ok(job),
        (Err(()), Err(e)) => Err(e),
        (Err(()), Ok(())) => Ok(job),
    }
}
