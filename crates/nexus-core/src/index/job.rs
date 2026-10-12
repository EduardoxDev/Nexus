//! Registro persistente dos jobs de indexação e trava entre processos (um job ativo por projeto).

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};

/// Sem heartbeat há mais que isso ⇒ o processo dono do job morreu.
pub const STALE_JOB_MS: i64 = 2 * 60 * 1000;
/// Jobs mantidos por projeto (retenção).
pub const JOBS_KEPT_PER_PROJECT: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IndexMode {
    /// Relê e confere o hash de todos os arquivos.
    Full,
    /// Só processa arquivos novos ou com tamanho/data de modificação diferentes.
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IndexTrigger {
    User,
    Watcher,
    Reconcile,
    Cli,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Running,
    Paused,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum JobPhase {
    Scanning,
    Indexing,
    Finalizing,
    Done,
}

macro_rules! str_enum {
    ($ty:ty { $($variant:ident => $s:literal),+ $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $s),+ }
            }
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

str_enum!(IndexMode { Full => "full", Incremental => "incremental" });
str_enum!(IndexTrigger { User => "user", Watcher => "watcher", Reconcile => "reconcile", Cli => "cli", Resume => "resume" });
str_enum!(JobStatus { Running => "running", Paused => "paused", Succeeded => "succeeded", Failed => "failed", Cancelled => "cancelled", Interrupted => "interrupted" });
str_enum!(JobPhase { Scanning => "scanning", Indexing => "indexing", Finalizing => "finalizing", Done => "done" });

/// Estado de um job. Também é o payload dos eventos de progresso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IndexJob {
    pub id: String,
    pub project_id: i64,
    pub mode: IndexMode,
    pub trigger: IndexTrigger,
    pub status: JobStatus,
    pub phase: JobPhase,
    pub started_at: i64,
    pub heartbeat_at: i64,
    pub finished_at: Option<i64>,
    /// Arquivos encontrados na varredura.
    pub discovered: i64,
    /// Arquivos que precisam ser (re)processados nesta execução.
    pub to_process: i64,
    pub processed: i64,
    pub unchanged: i64,
    /// Conteúdos novos divididos em chunks e indexados.
    pub new_contents: i64,
    /// Arquivos cujo conteúdo já estava no índice (deduplicação).
    pub reused_contents: i64,
    pub skipped: i64,
    pub removed: i64,
    pub renamed: i64,
    pub failed: i64,
    pub bytes_read: i64,
    pub generation: Option<i64>,
    pub error_message: Option<String>,
}

impl IndexJob {
    pub fn is_active(&self) -> bool {
        matches!(self.status, JobStatus::Running | JobStatus::Paused)
    }
}

/// Fila de indexação de um processo (desktop): o que está rodando e o que espera.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IndexingQueue {
    pub running: Option<i64>,
    pub paused: bool,
    pub queued: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IndexError {
    pub rel_path: String,
    pub code: String,
    pub message: String,
    pub at: i64,
}

const JOB_COLUMNS: &str =
    "id, project_id, mode, trigger, status, phase, started_at, heartbeat_at, finished_at,
     discovered, to_process, processed, unchanged, new_contents, reused_contents, skipped, removed,
     renamed, failed, bytes_read, generation, error_message";

fn row_to_job(r: &rusqlite::Row<'_>) -> rusqlite::Result<IndexJob> {
    let mode: String = r.get(2)?;
    let trigger: String = r.get(3)?;
    let status: String = r.get(4)?;
    let phase: String = r.get(5)?;
    Ok(IndexJob {
        id: r.get(0)?,
        project_id: r.get(1)?,
        mode: IndexMode::parse(&mode).unwrap_or(IndexMode::Incremental),
        trigger: IndexTrigger::parse(&trigger).unwrap_or(IndexTrigger::User),
        status: JobStatus::parse(&status).unwrap_or(JobStatus::Failed),
        phase: JobPhase::parse(&phase).unwrap_or(JobPhase::Done),
        started_at: r.get(6)?,
        heartbeat_at: r.get(7)?,
        finished_at: r.get(8)?,
        discovered: r.get(9)?,
        to_process: r.get(10)?,
        processed: r.get(11)?,
        unchanged: r.get(12)?,
        new_contents: r.get(13)?,
        reused_contents: r.get(14)?,
        skipped: r.get(15)?,
        removed: r.get(16)?,
        renamed: r.get(17)?,
        failed: r.get(18)?,
        bytes_read: r.get(19)?,
        generation: r.get(20)?,
        error_message: r.get(21)?,
    })
}

/// Marca como `interrupted` os jobs ativos de um projeto cujo processo dono comprovadamente
/// terminou (o lock de arquivo dele foi liberado pelo sistema operacional). A operação
/// `index.run` correspondente (mesmo ID) também é encerrada.
pub fn interrupt_orphaned(conn: &Connection, project_id: i64, now: i64) -> Result<usize> {
    conn.execute(
        "UPDATE operations SET status = 'interrupted', finished_at = ?1,
             error_message = 'O processo terminou antes de concluir a indexação.'
         WHERE status = 'running' AND id IN (
             SELECT id FROM indexing_jobs WHERE project_id = ?2 AND status IN ('running', 'paused'))",
        params![now, project_id],
    )?;
    Ok(conn.execute(
        "UPDATE indexing_jobs
         SET status = 'interrupted', phase = 'done', finished_at = ?1,
             error_message = 'O processo terminou antes de concluir a indexação.'
         WHERE project_id = ?2 AND status IN ('running', 'paused')",
        params![now, project_id],
    )?)
}

/// Marca como `interrupted` jobs ativos sem heartbeat recente (de qualquer projeto ou de um só).
pub fn recover_stale(conn: &Connection, project_id: Option<i64>, now: i64) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE indexing_jobs
         SET status = 'interrupted', phase = 'done', finished_at = ?1,
             error_message = 'O processo terminou antes de concluir a indexação.'
         WHERE status IN ('running', 'paused') AND heartbeat_at < ?2
           AND (?3 IS NULL OR project_id = ?3)",
        params![now, now - STALE_JOB_MS, project_id],
    )?)
}

/// Cria o job, adquirindo a trava do projeto. Falha com `Conflict` se já houver job ativo.
pub fn acquire(
    conn: &mut Connection,
    id: &str,
    project_id: i64,
    mode: IndexMode,
    trigger: IndexTrigger,
    now: i64,
) -> Result<IndexJob> {
    let tx = conn.transaction()?;
    recover_stale(&tx, Some(project_id), now)?;
    let inserted = tx.execute(
        "INSERT INTO indexing_jobs (id, project_id, mode, trigger, status, phase, pid, started_at, heartbeat_at)
         VALUES (?1, ?2, ?3, ?4, 'running', 'scanning', ?5, ?6, ?6)",
        params![
            id,
            project_id,
            mode.as_str(),
            trigger.as_str(),
            i64::from(std::process::id()),
            now
        ],
    );
    match inserted {
        Ok(_) => {}
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            return Err(Error::Conflict(
                "já existe uma indexação em andamento para este projeto".into(),
            ));
        }
        Err(e) => return Err(e.into()),
    }
    // Retenção: mantém os jobs mais recentes do projeto.
    tx.execute(
        "DELETE FROM indexing_jobs
         WHERE project_id = ?1 AND status NOT IN ('running', 'paused')
           AND id NOT IN (SELECT id FROM indexing_jobs WHERE project_id = ?1
                          ORDER BY started_at DESC, id DESC LIMIT ?2)",
        params![project_id, JOBS_KEPT_PER_PROJECT],
    )?;
    tx.commit()?;
    get(conn, id)?.ok_or_else(|| Error::Internal("job recém-criado não encontrado".into()))
}

/// Grava o estado atual do job (contadores, fase, status, heartbeat).
pub fn save(conn: &Connection, job: &IndexJob) -> Result<()> {
    conn.execute(
        "UPDATE indexing_jobs SET
            status = ?2, phase = ?3, heartbeat_at = ?4, finished_at = ?5, discovered = ?6,
            to_process = ?7, processed = ?8, unchanged = ?9, new_contents = ?10,
            reused_contents = ?11, skipped = ?12, removed = ?13, renamed = ?14, failed = ?15,
            bytes_read = ?16, generation = ?17, error_message = ?18
         WHERE id = ?1",
        params![
            job.id,
            job.status.as_str(),
            job.phase.as_str(),
            job.heartbeat_at,
            job.finished_at,
            job.discovered,
            job.to_process,
            job.processed,
            job.unchanged,
            job.new_contents,
            job.reused_contents,
            job.skipped,
            job.removed,
            job.renamed,
            job.failed,
            job.bytes_read,
            job.generation,
            job.error_message
        ],
    )?;
    // O job compartilha o ID com a operação `index.run`: mantém o heartbeat dela vivo também.
    conn.execute(
        "UPDATE operations SET heartbeat_at = ?2 WHERE id = ?1 AND status = 'running'",
        params![job.id, job.heartbeat_at],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<IndexJob>> {
    Ok(conn
        .query_row(
            &format!("SELECT {JOB_COLUMNS} FROM indexing_jobs WHERE id = ?1"),
            [id],
            row_to_job,
        )
        .optional()?)
}

pub fn list(conn: &Connection, project_id: i64, limit: u32) -> Result<Vec<IndexJob>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {JOB_COLUMNS} FROM indexing_jobs WHERE project_id = ?1
         ORDER BY started_at DESC, id DESC LIMIT ?2"
    ))?;
    let rows = stmt.query_map(params![project_id, limit], row_to_job)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn active(conn: &Connection, project_id: i64) -> Result<Option<IndexJob>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {JOB_COLUMNS} FROM indexing_jobs
                 WHERE project_id = ?1 AND status IN ('running', 'paused')"
            ),
            [project_id],
            row_to_job,
        )
        .optional()?)
}

pub fn record_error(
    conn: &Connection,
    job_id: &str,
    project_id: i64,
    rel_path: &str,
    code: &str,
    message: &str,
    now: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO index_errors (job_id, project_id, rel_path, code, message, at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![job_id, project_id, rel_path, code, message, now],
    )?;
    Ok(())
}

pub fn errors(conn: &Connection, job_id: &str, limit: u32) -> Result<Vec<IndexError>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, code, message, at FROM index_errors WHERE job_id = ?1
         ORDER BY id LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![job_id, limit], |r| {
        Ok(IndexError {
            rel_path: r.get(0)?,
            code: r.get(1)?,
            message: r.get(2)?,
            at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}
