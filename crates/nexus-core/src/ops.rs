//! Histórico local de operações: cada operação relevante tem um `op_id` (UUIDv7), status e duração.
//! Nunca registra conteúdo de arquivos.

use std::time::Instant;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::storage::Db;
use crate::time_util::now_ms;

/// Operações `running` sem heartbeat há mais que isso são consideradas interrompidas.
pub const STALE_AFTER_MS: i64 = 10 * 60 * 1000;
/// Retenção: idade máxima e quantidade máxima de registros.
pub const RETENTION_MAX_AGE_MS: i64 = 90 * 24 * 60 * 60 * 1000;
pub const RETENTION_MAX_ROWS: i64 = 10_000;
pub const MAX_LIST_LIMIT: u32 = 200;

/// Tipos de operação conhecidos.
pub mod kinds {
    pub const CORE_OPEN: &str = "core.open";
    pub const APP_STARTUP: &str = "app.startup";
    pub const INTEGRITY_CHECK: &str = "storage.integrity_check";
    pub const SETTINGS_UPDATE: &str = "settings.update";
    pub const CLI_INIT: &str = "cli.init";
    pub const PROJECT_ADD: &str = "project.add";
    pub const PROJECT_REMOVE: &str = "project.remove";
    pub const INDEX_RUN: &str = "index.run";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub kind: String,
    pub project_id: Option<i64>,
    pub status: OperationStatus,
    pub started_at: i64,
    pub heartbeat_at: i64,
    pub finished_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub detail: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Filtro para listar operações.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationQuery {
    /// 1..=200; padrão 50.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Prefixo do tipo, ex.: `storage.`.
    #[serde(default)]
    pub kind_prefix: Option<String>,
    #[serde(default)]
    pub status: Option<OperationStatus>,
}

/// Operação em andamento.
#[derive(Debug)]
pub struct Operation {
    id: String,
    kind: String,
    started: Instant,
}

impl Operation {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }
}

pub fn new_op_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

pub fn start(db: &Db, kind: &str, project_id: Option<i64>) -> Result<Operation> {
    let op = Operation {
        id: new_op_id(),
        kind: kind.to_owned(),
        started: Instant::now(),
    };
    let now = now_ms();
    db.write(|conn| {
        conn.execute(
            "INSERT INTO operations (id, kind, project_id, status, started_at, heartbeat_at)
             VALUES (?1, ?2, ?3, 'running', ?4, ?4)",
            params![op.id, op.kind, project_id, now],
        )?;
        Ok(())
    })?;
    tracing::info!(op_id = %op.id, kind = %op.kind, "operação iniciada");
    Ok(op)
}

pub fn heartbeat(db: &Db, op: &Operation) -> Result<()> {
    db.write(|conn| {
        conn.execute(
            "UPDATE operations SET heartbeat_at = ?2 WHERE id = ?1 AND status = 'running'",
            params![op.id, now_ms()],
        )?;
        Ok(())
    })
}

pub fn succeed(db: &Db, op: &Operation, detail: Option<&serde_json::Value>) -> Result<()> {
    finish(db, op, OperationStatus::Succeeded, detail, None)
}

pub fn fail(db: &Db, op: &Operation, err: &Error) -> Result<()> {
    let status = if err.code() == crate::ErrorCode::Cancelled {
        OperationStatus::Cancelled
    } else {
        OperationStatus::Failed
    };
    finish(db, op, status, None, Some(err))
}

fn finish(
    db: &Db,
    op: &Operation,
    status: OperationStatus,
    detail: Option<&serde_json::Value>,
    err: Option<&Error>,
) -> Result<()> {
    let duration = i64::try_from(op.started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let now = now_ms();
    let detail_json = detail.map(serde_json::Value::to_string);
    let (code, message) = match err {
        Some(e) => (Some(e.code().as_str()), Some(e.user_message())),
        None => (None, None),
    };
    db.write(|conn| {
        conn.execute(
            "UPDATE operations
             SET status = ?2, finished_at = ?3, heartbeat_at = ?3, duration_ms = ?4,
                 detail_json = COALESCE(?5, detail_json), error_code = ?6, error_message = ?7
             WHERE id = ?1",
            params![
                op.id,
                status.as_str(),
                now,
                duration,
                detail_json,
                code,
                message
            ],
        )?;
        Ok(())
    })?;
    match err {
        Some(e) => tracing::warn!(
            op_id = %op.id, kind = %op.kind, duration_ms = duration,
            status = status.as_str(), error = %e, "operação terminou com erro"
        ),
        None => tracing::info!(
            op_id = %op.id, kind = %op.kind, duration_ms = duration, "operação concluída"
        ),
    }
    Ok(())
}

/// Executa `f` como operação registrada. `f` devolve o resultado e um detalhe opcional.
/// Erros são registrados e devolvidos com o `op_id` anexado.
pub fn run<T>(
    db: &Db,
    kind: &str,
    project_id: Option<i64>,
    f: impl FnOnce(&Operation) -> Result<(T, Option<serde_json::Value>)>,
) -> Result<T> {
    let op = start(db, kind, project_id)?;
    match f(&op) {
        Ok((value, detail)) => {
            succeed(db, &op, detail.as_ref())?;
            Ok(value)
        }
        Err(err) => {
            if let Err(log_err) = fail(db, &op, &err) {
                tracing::error!(op_id = %op.id, error = %log_err, "falha ao registrar erro da operação");
            }
            Err(Error::InOperation {
                op_id: op.id,
                source: Box::new(err),
            })
        }
    }
}

/// Registra uma operação que já terminou (ex.: migrations, executadas antes de a tabela existir).
pub fn record_completed(
    db: &Db,
    kind: &str,
    started_at: i64,
    duration_ms: i64,
    detail: Option<&serde_json::Value>,
) -> Result<String> {
    let id = new_op_id();
    let finished = started_at.saturating_add(duration_ms);
    db.write(|conn| {
        conn.execute(
            "INSERT INTO operations
               (id, kind, status, started_at, heartbeat_at, finished_at, duration_ms, detail_json)
             VALUES (?1, ?2, 'succeeded', ?3, ?4, ?4, ?5, ?6)",
            params![
                id,
                kind,
                started_at,
                finished,
                duration_ms,
                detail.map(serde_json::Value::to_string)
            ],
        )?;
        Ok(())
    })?;
    Ok(id)
}

pub fn list(conn: &Connection, query: &OperationQuery) -> Result<Vec<OperationRecord>> {
    let limit = query.limit.unwrap_or(50);
    if limit == 0 || limit > MAX_LIST_LIMIT {
        return Err(Error::InvalidInput(format!(
            "limit deve estar entre 1 e {MAX_LIST_LIMIT}"
        )));
    }
    let kind_prefix = match &query.kind_prefix {
        Some(prefix) => {
            let valid = !prefix.is_empty()
                && prefix.len() <= 64
                && prefix
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_');
            if !valid {
                return Err(Error::InvalidInput(
                    "kindPrefix aceita apenas letras minúsculas, '.' e '_' (até 64)".into(),
                ));
            }
            Some(format!("{prefix}%"))
        }
        None => None,
    };
    let mut stmt = conn.prepare(
        "SELECT id, kind, project_id, status, started_at, heartbeat_at, finished_at,
                duration_ms, detail_json, error_code, error_message
         FROM operations
         WHERE (?1 IS NULL OR kind LIKE ?1)
           AND (?2 IS NULL OR status = ?2)
         ORDER BY started_at DESC, id DESC
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![
            kind_prefix,
            query.status.map(OperationStatus::as_str),
            limit
        ],
        row_to_record,
    )?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<OperationRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, project_id, status, started_at, heartbeat_at, finished_at,
                duration_ms, detail_json, error_code, error_message
         FROM operations WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_record)?;
    Ok(rows.next().transpose()?)
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM operations", [], |r| r.get(0))?)
}

fn row_to_record(r: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let status: String = r.get(3)?;
    let detail: Option<String> = r.get(8)?;
    Ok(OperationRecord {
        id: r.get(0)?,
        kind: r.get(1)?,
        project_id: r.get(2)?,
        status: OperationStatus::parse(&status).unwrap_or(OperationStatus::Failed),
        started_at: r.get(4)?,
        heartbeat_at: r.get(5)?,
        finished_at: r.get(6)?,
        duration_ms: r.get(7)?,
        detail: detail.and_then(|d| serde_json::from_str(&d).ok()),
        error_code: r.get(9)?,
        error_message: r.get(10)?,
    })
}

/// Marca como `interrupted` operações `running` sem heartbeat recente.
pub fn recover_interrupted(conn: &Connection, now: i64, stale_after_ms: i64) -> Result<usize> {
    let n = conn.execute(
        "UPDATE operations
         SET status = 'interrupted', finished_at = ?1,
             error_code = 'interrupted',
             error_message = 'O processo terminou antes de concluir a operação.'
         WHERE status = 'running' AND heartbeat_at < ?2",
        params![now, now - stale_after_ms],
    )?;
    Ok(n)
}

/// Aplica a política de retenção. Operações em execução nunca são removidas.
pub fn prune(conn: &Connection, now: i64, max_age_ms: i64, max_rows: i64) -> Result<usize> {
    let by_age = conn.execute(
        "DELETE FROM operations WHERE status <> 'running' AND started_at < ?1",
        params![now - max_age_ms],
    )?;
    let by_count = conn.execute(
        "DELETE FROM operations
         WHERE status <> 'running'
           AND id NOT IN (SELECT id FROM operations ORDER BY started_at DESC, id DESC LIMIT ?1)",
        params![max_rows],
    )?;
    Ok(by_age + by_count)
}
