//! Testes de integração do armazenamento: migrations, checksum, downgrade, backup e pragmas.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use nexus_core::storage::migrate::{self, BackupPolicy, Migration};
use nexus_core::storage::{self, Db};
use nexus_core::{Error, ErrorCode};
use rusqlite::Connection;

const V1: Migration = Migration {
    version: 1,
    name: "one",
    sql: "CREATE TABLE a (x INTEGER NOT NULL);",
};
const V2: Migration = Migration {
    version: 2,
    name: "two",
    sql: "CREATE TABLE b (y INTEGER NOT NULL);",
};

fn mem() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap()
        == 1
}

#[test]
fn fresh_database_migrates_to_latest() {
    let dir = tempfile::tempdir().unwrap();
    let (db, report) = Db::open(&dir.path().join("n.db"), None).unwrap();
    assert_eq!(report.from_version, 0);
    assert_eq!(report.to_version, migrate::latest_version());
    assert_eq!(report.applied.len(), migrate::MIGRATIONS.len());
    assert!(report.backup.is_none(), "banco novo não precisa de backup");
    let applied = db.read(migrate::applied_migrations).unwrap();
    assert_eq!(applied.len(), migrate::MIGRATIONS.len());
    assert_eq!(applied[0].checksum, migrate::MIGRATIONS[0].checksum());
}

#[test]
fn reopening_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("n.db");
    drop(Db::open(&path, None).unwrap());
    let (_db, report) = Db::open(&path, None).unwrap();
    assert!(report.applied.is_empty());
    assert_eq!(report.from_version, report.to_version);
}

#[test]
fn tampered_migration_is_rejected() {
    let mut conn = mem();
    migrate::migrate_with(&mut conn, &[V1], None).unwrap();
    let tampered = Migration {
        sql: "CREATE TABLE a (x TEXT);",
        ..V1
    };
    let err = migrate::migrate_with(&mut conn, &[tampered], None).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Storage);
    assert!(err.to_string().contains("alterada"), "{err}");
}

#[test]
fn checksum_ignores_line_ending_style() {
    let lf = Migration {
        sql: "CREATE TABLE a (x);\nCREATE TABLE b (y);\n",
        ..V1
    };
    let crlf = Migration {
        sql: "CREATE TABLE a (x);\r\nCREATE TABLE b (y);\r\n",
        ..V1
    };
    assert_eq!(lf.checksum(), crlf.checksum());
}

#[test]
fn newer_schema_is_refused() {
    let mut conn = mem();
    migrate::migrate_with(&mut conn, &[V1, V2], None).unwrap();
    let err = migrate::migrate_with(&mut conn, &[V1], None).unwrap_err();
    assert!(matches!(
        err,
        Error::IncompatibleSchema {
            found: 2,
            supported: 1
        }
    ));
    assert_eq!(err.code(), ErrorCode::IncompatibleSchema);
}

#[test]
fn failed_migration_rolls_back_completely() {
    let mut conn = mem();
    migrate::migrate_with(&mut conn, &[V1], None).unwrap();
    let broken = Migration {
        version: 2,
        name: "broken",
        sql: "CREATE TABLE b (y); INSERT INTO tabela_inexistente VALUES (1);",
    };
    let err = migrate::migrate_with(&mut conn, &[V1, broken], None).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Storage);
    assert!(
        !table_exists(&conn, "b"),
        "a transação deve desfazer a migration parcial"
    );
    let applied = migrate::applied_migrations(&conn).unwrap();
    assert_eq!(applied.last().unwrap().version, 1);
}

#[test]
fn backup_is_taken_before_migrating_existing_data_and_is_restorable() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("n.db");
    let policy = BackupPolicy {
        dir: dir.path().join("backups"),
        keep: 5,
    };

    let mut conn = Connection::open(&db_path).unwrap();
    migrate::migrate_with(&mut conn, &[V1], Some(&policy)).unwrap();
    conn.execute("INSERT INTO a (x) VALUES (42)", []).unwrap();

    let report = migrate::migrate_with(&mut conn, &[V1, V2], Some(&policy)).unwrap();
    assert_eq!(report.applied, vec![2]);
    let backup = report.backup.expect("backup deve existir");
    assert!(
        backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("nexus-v1-")
    );

    // O backup é um banco completo no estado anterior à migration.
    let restored = Connection::open(&backup).unwrap();
    let x: i64 = restored
        .query_row("SELECT x FROM a", [], |r| r.get(0))
        .unwrap();
    assert_eq!(x, 42);
    assert!(!table_exists(&restored, "b"));
    assert_eq!(migrate::applied_migrations(&restored).unwrap().len(), 1);
}

#[test]
fn backup_retention_keeps_most_recent() {
    let dir = tempfile::tempdir().unwrap();
    let policy = BackupPolicy {
        dir: dir.path().join("backups"),
        keep: 3,
    };
    let mut conn = mem();
    migrate::migrate_with(&mut conn, &[V1], None).unwrap();
    for _ in 0..6 {
        migrate::backup_database(&conn, &policy, 1).unwrap();
    }
    assert_eq!(migrate::list_backups(&policy.dir).unwrap().len(), 3);
}

#[test]
fn pragmas_are_applied_to_writer_and_readers() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Db::open(&dir.path().join("n.db"), None).unwrap();
    let (mode, fk_w) = db
        .write(|c| {
            let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
            Ok((mode, fk))
        })
        .unwrap();
    assert_eq!(mode.to_lowercase(), "wal");
    assert_eq!(fk_w, 1);
    let fk_r: i64 = db
        .read(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(fk_r, 1);
}

#[test]
fn readers_cannot_write() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Db::open(&dir.path().join("n.db"), None).unwrap();
    let result = db.read(|c| {
        c.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('x', '1', 0)",
            [],
        )?;
        Ok(())
    });
    assert!(result.is_err());
}

#[test]
fn embedded_sqlite_has_fts5() {
    let conn = mem();
    assert!(storage::fts5_available(&conn).unwrap());
    conn.execute_batch(
        "CREATE VIRTUAL TABLE t USING fts5(body, content='', contentless_delete=1);
         INSERT INTO t (rowid, body) VALUES (1, 'contexto verificável');
         DELETE FROM t WHERE rowid = 1;",
    )
    .expect("FTS5 contentless_delete (ADR-0003) precisa estar disponível");
}
