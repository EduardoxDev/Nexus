//! Migrations versionadas, com checksum, backup prévio e recusa de downgrade (ADR-0007).

use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::time_util::{file_stamp, now_ms};

/// Uma migration embutida no binário.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

impl Migration {
    /// blake3 do SQL com finais de linha normalizados (imune a CRLF/LF do checkout).
    pub fn checksum(&self) -> String {
        let normalized = self.sql.replace("\r\n", "\n");
        blake3::hash(normalized.as_bytes()).to_hex().to_string()
    }
}

/// Migrations conhecidas por este binário, em ordem crescente.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "foundation",
        sql: include_str!("migrations/0001_foundation.sql"),
    },
    Migration {
        version: 2,
        name: "projects_index",
        sql: include_str!("migrations/0002_projects_index.sql"),
    },
    Migration {
        version: 3,
        name: "context_packs",
        sql: include_str!("migrations/0003_context_packs.sql"),
    },
    Migration {
        version: 4,
        name: "memory_freshness",
        sql: include_str!("migrations/0004_memory_freshness.sql"),
    },
    Migration {
        version: 5,
        name: "analysis",
        sql: include_str!("migrations/0005_analysis.sql"),
    },
    Migration {
        version: 6,
        name: "edges_targets",
        sql: include_str!("migrations/0006_edges_targets.sql"),
    },
    Migration {
        version: 7,
        name: "shared_memory",
        sql: include_str!("migrations/0007_shared_memory.sql"),
    },
];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Migration já registrada no banco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppliedMigration {
    pub version: i64,
    pub name: String,
    pub checksum: String,
    pub applied_at: i64,
    pub duration_ms: i64,
}

/// Resultado de uma execução do migrador.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub from_version: i64,
    pub to_version: i64,
    pub applied: Vec<i64>,
    pub backup: Option<PathBuf>,
    pub duration_ms: i64,
}

/// Política de backup antes de migrar um banco com dados.
#[derive(Debug, Clone)]
pub struct BackupPolicy {
    pub dir: PathBuf,
    pub keep: usize,
}

const BACKUP_PREFIX: &str = "nexus-v";

pub fn migrate(conn: &mut Connection, backup: Option<&BackupPolicy>) -> Result<MigrationReport> {
    migrate_with(conn, MIGRATIONS, backup)
}

pub fn migrate_with(
    conn: &mut Connection,
    migrations: &[Migration],
    backup: Option<&BackupPolicy>,
) -> Result<MigrationReport> {
    let started = Instant::now();
    validate_sequence(migrations)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY NOT NULL,
            name        TEXT NOT NULL,
            checksum    TEXT NOT NULL,
            applied_at  INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL
        ) STRICT;",
    )?;

    let applied = applied_migrations(conn)?;
    let known_latest = migrations.last().map_or(0, |m| m.version);
    let current = applied.last().map_or(0, |m| m.version);

    if current > known_latest {
        return Err(Error::IncompatibleSchema {
            found: current,
            supported: known_latest,
        });
    }
    for done in &applied {
        let Some(expected) = migrations.iter().find(|m| m.version == done.version) else {
            return Err(Error::Storage(format!(
                "a migration {} registrada no banco não existe neste binário",
                done.version
            )));
        };
        if expected.checksum() != done.checksum {
            return Err(Error::Storage(format!(
                "a migration {} ({}) foi alterada depois de aplicada; o banco não corresponde a este binário",
                done.version, done.name
            )));
        }
    }

    let pending: Vec<&Migration> = migrations.iter().filter(|m| m.version > current).collect();
    let mut report = MigrationReport {
        from_version: current,
        to_version: current,
        ..MigrationReport::default()
    };
    if pending.is_empty() {
        report.duration_ms = elapsed_ms(started);
        return Ok(report);
    }

    if current > 0
        && let Some(policy) = backup
    {
        report.backup = Some(backup_database(conn, policy, current)?);
    }

    // Recriar uma tabela (para mudar um CHECK) exige as chaves estrangeiras desligadas: com
    // elas ligadas, o DROP apagaria em cascata as linhas que apontam para a tabela. O pragma só
    // vale fora de transação; as referências são conferidas antes de cada commit.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let applied = apply_pending(conn, &pending, &mut report);
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    applied?;
    report.duration_ms = elapsed_ms(started);
    Ok(report)
}

fn apply_pending(
    conn: &mut Connection,
    pending: &[&Migration],
    report: &mut MigrationReport,
) -> Result<()> {
    for migration in pending {
        let step = Instant::now();
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql).map_err(|e| {
            Error::Storage(format!(
                "falha ao aplicar a migration {} ({}): {e}",
                migration.version, migration.name
            ))
        })?;
        let broken: Option<String> = tx
            .query_row("PRAGMA foreign_key_check", [], |r| r.get(0))
            .optional()?;
        if let Some(table) = broken {
            return Err(Error::Storage(format!(
                "a migration {} ({}) deixaria referências quebradas em {table}",
                migration.version, migration.name
            )));
        }
        tx.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                migration.version,
                migration.name,
                migration.checksum(),
                now_ms(),
                elapsed_ms(step)
            ],
        )?;
        tx.commit()?;
        tracing::info!(
            version = migration.version,
            name = migration.name,
            "migration aplicada"
        );
        report.applied.push(migration.version);
        report.to_version = migration.version;
    }
    Ok(())
}

pub fn applied_migrations(conn: &Connection) -> Result<Vec<AppliedMigration>> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT version, name, checksum, applied_at, duration_ms
         FROM schema_migrations ORDER BY version",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(AppliedMigration {
            version: r.get(0)?,
            name: r.get(1)?,
            checksum: r.get(2)?,
            applied_at: r.get(3)?,
            duration_ms: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn validate_sequence(migrations: &[Migration]) -> Result<()> {
    for (i, m) in migrations.iter().enumerate() {
        let expected = i64::try_from(i).unwrap_or(i64::MAX) + 1;
        if m.version != expected {
            return Err(Error::Internal(format!(
                "sequência de migrations inválida: esperado {expected}, encontrado {}",
                m.version
            )));
        }
    }
    Ok(())
}

/// Copia o banco inteiro com a Online Backup API e aplica a retenção.
pub fn backup_database(conn: &Connection, policy: &BackupPolicy, version: i64) -> Result<PathBuf> {
    std::fs::create_dir_all(&policy.dir)
        .map_err(|e| Error::io(format!("criar {}", policy.dir.display()), e))?;
    let mut target = policy.dir.join(format!(
        "{BACKUP_PREFIX}{version}-{}.db",
        file_stamp(now_ms())
    ));
    let mut n = 1;
    while target.exists() {
        target = policy.dir.join(format!(
            "{BACKUP_PREFIX}{version}-{}-{n}.db",
            file_stamp(now_ms())
        ));
        n += 1;
    }
    let mut dst = Connection::open(&target)?;
    {
        let backup = rusqlite::backup::Backup::new(conn, &mut dst)?;
        backup.run_to_completion(256, std::time::Duration::from_millis(5), None)?;
    }
    drop(dst);
    tracing::info!(path = %target.display(), "backup pré-migração criado");
    prune_backups(&policy.dir, policy.keep)?;
    Ok(target)
}

/// Lista backups gerados pelo NEXUS, do mais recente para o mais antigo.
pub fn list_backups(dir: &Path) -> Result<Vec<(PathBuf, std::fs::Metadata)>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(format!("listar {}", dir.display()), e)),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(format!("listar {}", dir.display()), e))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(BACKUP_PREFIX) && name.ends_with(".db") {
            let meta = entry
                .metadata()
                .map_err(|e| Error::io(format!("ler {}", entry.path().display()), e))?;
            backups.push((entry.path(), meta));
        }
    }
    backups.sort_by(|a, b| {
        let ta = a.1.modified().ok();
        let tb = b.1.modified().ok();
        tb.cmp(&ta).then_with(|| b.0.cmp(&a.0))
    });
    Ok(backups)
}

fn prune_backups(dir: &Path, keep: usize) -> Result<()> {
    for (path, _) in list_backups(dir)?.into_iter().skip(keep) {
        std::fs::remove_file(&path)
            .map_err(|e| Error::io(format!("remover backup antigo {}", path.display()), e))?;
    }
    Ok(())
}

fn elapsed_ms(start: Instant) -> i64 {
    i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX)
}
