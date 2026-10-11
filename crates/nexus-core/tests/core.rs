//! Testes de integração da fachada `Core`: abertura, configurações, operações, diagnóstico e integridade.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Seek, SeekFrom, Write};

use nexus_core::ops::{self, OperationQuery, OperationStatus};
use nexus_core::settings::{MotionPreference, SettingKey, ThemePreference};
use nexus_core::{Core, DataPaths, ErrorCode};
use rusqlite::params;
use serde_json::json;

fn open(dir: &tempfile::TempDir) -> Core {
    Core::open(DataPaths::at(dir.path().join("data"))).unwrap()
}

#[test]
fn open_creates_layout_and_records_operation() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    let paths = core.paths();
    assert!(paths.db().is_file());
    assert!(paths.logs().is_dir());
    assert!(paths.backups().is_dir());

    let report = core.open_report();
    assert_eq!(report.migration.from_version, 0);
    let ops = core
        .list_operations(&OperationQuery {
            kind_prefix: Some(ops::kinds::CORE_OPEN.into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].id, report.op_id);
    assert_eq!(ops[0].status, OperationStatus::Succeeded);
    assert_eq!(
        ops[0].detail.as_ref().unwrap()["toVersion"],
        json!(nexus_core::storage::migrate::latest_version())
    );
}

#[test]
fn settings_have_defaults_and_persist_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let core = open(&dir);
        let s = core.settings().unwrap();
        assert_eq!(s.theme, ThemePreference::Dark);
        assert_eq!(s.reduced_motion, MotionPreference::System);
        assert!(!s.sidebar_collapsed);

        let s = core
            .set_setting(SettingKey::UiTheme, json!("light"))
            .unwrap();
        assert_eq!(s.theme, ThemePreference::Light);
        core.set_setting(SettingKey::UiReducedMotion, json!("on"))
            .unwrap();
    }
    let core = open(&dir);
    let s = core.settings().unwrap();
    assert_eq!(s.theme, ThemePreference::Light);
    assert_eq!(s.reduced_motion, MotionPreference::On);
    assert!(core.open_report().migration.applied.is_empty());
}

#[test]
fn invalid_setting_fails_with_op_id_and_is_logged() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    let err = core
        .set_setting(SettingKey::UiTheme, json!("neon"))
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    let op_id = err.op_id().expect("erro deve carregar op_id").to_owned();

    let record = core
        .db()
        .read(|c| ops::get(c, &op_id))
        .unwrap()
        .expect("operação registrada");
    assert_eq!(record.status, OperationStatus::Failed);
    assert_eq!(record.error_code.as_deref(), Some("invalid_input"));
    assert_eq!(core.settings().unwrap().theme, ThemePreference::Dark);
}

#[test]
fn setting_values_are_not_recorded_in_history() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    core.set_setting(SettingKey::UiTheme, json!("light"))
        .unwrap();
    let ops = core
        .list_operations(&OperationQuery {
            kind_prefix: Some("settings.".into()),
            ..Default::default()
        })
        .unwrap();
    let detail = ops[0].detail.as_ref().unwrap();
    assert_eq!(detail, &json!({ "key": "ui.theme" }));
}

#[test]
fn list_operations_validates_input() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    for q in [
        OperationQuery {
            limit: Some(0),
            ..Default::default()
        },
        OperationQuery {
            limit: Some(201),
            ..Default::default()
        },
        OperationQuery {
            kind_prefix: Some("x' OR 1=1 --".into()),
            ..Default::default()
        },
        OperationQuery {
            kind_prefix: Some(String::new()),
            ..Default::default()
        },
    ] {
        let err = core.list_operations(&q).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidInput);
    }
}

#[test]
fn list_operations_filters_by_status_and_respects_limit() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    for _ in 0..5 {
        core.integrity_check().unwrap();
    }
    let _ = core.set_setting(SettingKey::UiTheme, json!("bad"));
    let checks = core
        .list_operations(&OperationQuery {
            limit: Some(3),
            kind_prefix: Some("storage.".into()),
            status: None,
        })
        .unwrap();
    assert_eq!(checks.len(), 3);
    assert!(
        checks
            .windows(2)
            .all(|w| w[0].started_at >= w[1].started_at)
    );
    let failed = core
        .list_operations(&OperationQuery {
            status: Some(OperationStatus::Failed),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(failed.len(), 1);
}

#[test]
fn stale_running_operations_become_interrupted_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let op_id;
    {
        let core = open(&dir);
        let op = ops::start(core.db(), "test.long", None).unwrap();
        op_id = op.id().to_owned();
        // Simula um processo que morreu há 11 minutos sem finalizar a operação.
        core.db()
            .write(|c| {
                c.execute(
                    "UPDATE operations SET heartbeat_at = heartbeat_at - ?2 WHERE id = ?1",
                    params![op_id, 11 * 60 * 1000],
                )?;
                Ok(())
            })
            .unwrap();
    }
    let core = open(&dir);
    assert_eq!(core.open_report().recovered_operations, 1);
    let record = core.db().read(|c| ops::get(c, &op_id)).unwrap().unwrap();
    assert_eq!(record.status, OperationStatus::Interrupted);
    assert!(record.finished_at.is_some());
}

#[test]
fn fresh_running_operations_are_not_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    let op = ops::start(core.db(), "test.long", None).unwrap();
    // Outro processo abrindo o mesmo banco não pode marcar a operação ativa.
    let other = open(&dir);
    assert_eq!(other.open_report().recovered_operations, 0);
    let record = core.db().read(|c| ops::get(c, op.id())).unwrap().unwrap();
    assert_eq!(record.status, OperationStatus::Running);
}

#[test]
fn retention_prunes_old_and_excess_operations() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    let now = 10_000_000_000i64;
    core.db()
        .write(|c| {
            let tx = c.transaction()?;
            for i in 0..20i64 {
                tx.execute(
                    "INSERT INTO operations (id, kind, status, started_at, heartbeat_at)
                     VALUES (?1, 'test', 'succeeded', ?2, ?2)",
                    params![format!("old-{i}"), now - ops::RETENTION_MAX_AGE_MS - 1 - i],
                )?;
                tx.execute(
                    "INSERT INTO operations (id, kind, status, started_at, heartbeat_at)
                     VALUES (?1, 'test', 'succeeded', ?2, ?2)",
                    params![format!("new-{i}"), now - i],
                )?;
            }
            tx.execute(
                "INSERT INTO operations (id, kind, status, started_at, heartbeat_at)
                 VALUES ('running-old', 'test', 'running', 1, ?1)",
                params![now],
            )?;
            tx.commit()?;
            Ok(())
        })
        .unwrap();
    let removed = core
        .db()
        .write(|c| ops::prune(c, now, ops::RETENTION_MAX_AGE_MS, 10))
        .unwrap();
    let remaining: Vec<String> = core
        .db()
        .read(|c| {
            let mut s = c.prepare("SELECT id FROM operations WHERE kind = 'test'")?;
            let ids = s
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            Ok(ids)
        })
        .unwrap();
    assert!(
        remaining.contains(&"running-old".to_owned()),
        "operações em execução não são podadas"
    );
    assert!(remaining.iter().all(|id| !id.starts_with("old-")));
    assert!(removed >= 20);
}

#[test]
fn diagnostics_reports_real_state() {
    let dir = tempfile::tempdir().unwrap();
    let core = open(&dir);
    let d = core.diagnostics().unwrap();
    assert_eq!(d.schema_version, d.latest_schema_version);
    assert!(d.fts5_available);
    assert!(d.database.exists);
    assert!(d.database.size_bytes > 0);
    assert_eq!(d.migrations.len() as i64, d.latest_schema_version);
    assert!(d.last_integrity_check.is_none());
    assert!(d.operations_total >= 1);

    let check = core.integrity_check().unwrap();
    assert!(check.ok);
    assert_eq!(check.messages, vec!["ok".to_owned()]);
    let d = core.diagnostics().unwrap();
    assert_eq!(d.last_integrity_check.unwrap().id, check.op_id);
}

#[test]
fn corruption_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let db_path;
    {
        let core = open(&dir);
        db_path = core.paths().db();
        core.db()
            .write(|c| {
                let tx = c.transaction()?;
                for i in 0..3000 {
                    tx.execute(
                        "INSERT INTO operations (id, kind, status, started_at, heartbeat_at, detail_json)
                         VALUES (?1, 'filler', 'succeeded', ?2, ?2, ?3)",
                        params![format!("f-{i:05}"), 10_000 + i, json!({ "pad": "x".repeat(64) }).to_string()],
                    )?;
                }
                tx.commit()?;
                Ok(())
            })
            .unwrap();
    }
    // Sobrescreve páginas no meio do arquivo.
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(&db_path)
        .unwrap();
    let len = f.metadata().unwrap().len();
    assert!(
        len > 64 * 1024,
        "o banco de teste precisa ter várias páginas"
    );
    f.seek(SeekFrom::Start(len / 2)).unwrap();
    f.write_all(&[0xA5; 16 * 1024]).unwrap();
    drop(f);

    let detected = match Core::open(DataPaths::at(dir.path().join("data"))) {
        Err(_) => true,
        Ok(core) => match core.integrity_check() {
            Err(_) => true,
            Ok(report) => !report.ok,
        },
    };
    assert!(
        detected,
        "corrupção deve ser detectada na abertura ou pela verificação"
    );
}
