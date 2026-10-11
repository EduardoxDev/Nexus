//! Acesso ao SQLite: abertura, pragmas, migrations e um pool mínimo de leitores.

pub mod migrate;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::error::{Error, Result, poisoned};
pub use migrate::{AppliedMigration, BackupPolicy, MigrationReport};

const BUSY_TIMEOUT: Duration = Duration::from_millis(5_000);
const MAX_IDLE_READERS: usize = 4;

/// Banco do NEXUS. Um escritor serializado e leitores reaproveitáveis (WAL permite leituras
/// concorrentes com a escrita). Seguro para compartilhar entre threads via `Arc`.
pub struct Db {
    path: PathBuf,
    writer: Mutex<Connection>,
    readers: Mutex<Vec<Connection>>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Db {
    /// Abre (ou cria) o banco e aplica as migrations pendentes.
    pub fn open(path: &Path, backup: Option<&BackupPolicy>) -> Result<(Self, MigrationReport)> {
        let mut writer = open_connection(path, false)?;
        let report = migrate::migrate(&mut writer, backup)?;
        let db = Self {
            path: path.to_path_buf(),
            writer: Mutex::new(writer),
            readers: Mutex::new(Vec::new()),
        };
        Ok((db, report))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Executa `f` com uma conexão de leitura.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = match self.readers.lock().map_err(poisoned)?.pop() {
            Some(conn) => conn,
            None => open_connection(&self.path, true)?,
        };
        let result = f(&conn);
        let mut idle = self.readers.lock().map_err(poisoned)?;
        if idle.len() < MAX_IDLE_READERS {
            idle.push(conn);
        }
        result
    }

    /// Executa `f` com a conexão de escrita (serializada). Use transações curtas e nunca faça
    /// IO de filesystem demorado dentro de `f`.
    pub fn write<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut conn = self.writer.lock().map_err(poisoned)?;
        f(&mut conn)
    }
}

fn open_connection(path: &Path, read_only: bool) -> Result<Connection> {
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
    };
    let conn = Connection::open_with_flags(path, flags).map_err(|e| {
        Error::Storage(format!(
            "não foi possível abrir o banco em {}: {e}",
            path.display()
        ))
    })?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    if !read_only {
        let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            tracing::warn!(
                mode,
                "não foi possível ativar WAL; seguindo com o modo atual"
            );
        }
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

/// Versão do SQLite embutido.
pub fn sqlite_version() -> &'static str {
    rusqlite::version()
}

/// Verifica se o SQLite embutido foi compilado com FTS5.
pub fn fts5_available(conn: &Connection) -> Result<bool> {
    let used: i64 = conn.query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
        r.get(0)
    })?;
    Ok(used == 1)
}
