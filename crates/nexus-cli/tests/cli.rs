//! Testes da CLI executando o binário real contra um diretório de dados temporário.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use assert_cmd::Command;
use serde_json::Value;

fn nexus(data: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("nexus").unwrap();
    cmd.env_remove("NEXUS_DATA_DIR").arg("--data-dir").arg(data);
    cmd
}

#[test]
fn init_creates_database_and_reports_json() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let out = nexus(&data).args(["init", "--json"]).assert().success();
    let v: Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["schemaVersion"], 1);
    assert_eq!(v["appliedMigrations"], serde_json::json!([1]));
    assert!(data.join("nexus.db").is_file());

    // Segunda execução: nada pendente.
    let out = nexus(&data).args(["init", "--json"]).assert().success();
    let v: Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["appliedMigrations"], serde_json::json!([]));
}

#[test]
fn init_human_output_is_portuguese() {
    let dir = tempfile::tempdir().unwrap();
    let out = nexus(&dir.path().join("d")).arg("init").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("NEXUS pronto."), "{stdout}");
    assert!(stdout.contains("Schema:         v1"), "{stdout}");
}

#[test]
fn diagnostics_json_has_expected_shape() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let out = nexus(&data)
        .args(["diagnostics", "--check", "--json"])
        .assert()
        .success();
    let v: Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["diagnostics"]["schemaVersion"], 1);
    assert_eq!(v["diagnostics"]["fts5Available"], true);
    assert_eq!(v["integrity"]["ok"], true);
    assert!(v["app"]["coreVersion"].is_string());
    assert!(v["diagnostics"]["migrations"].as_array().unwrap().len() == 1);
}

#[test]
fn usage_errors_exit_with_2() {
    let dir = tempfile::tempdir().unwrap();
    nexus(dir.path())
        .arg("comando-inexistente")
        .assert()
        .code(2);
}

#[test]
fn incompatible_schema_exits_with_4_and_json_error() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    nexus(&data).arg("init").assert().success();
    // Simula um banco criado por uma versão futura do NEXUS.
    drop(insert_future_migration(&data.join("nexus.db")));

    let out = nexus(&data)
        .args(["diagnostics", "--json"])
        .assert()
        .code(4);
    let v: Value = serde_json::from_slice(&out.get_output().stderr).unwrap();
    assert_eq!(v["error"]["code"], "incompatible_schema");
}

/// Registra uma migration "futura" diretamente no banco de teste.
fn insert_future_migration(db: &std::path::Path) -> nexus_core::storage::Db {
    let (database, _) = nexus_core::storage::Db::open(db, None).unwrap();
    database
        .write(|c| {
            c.execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at, duration_ms)
                 VALUES (99, 'futura', 'x', 0, 0)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    database
}
