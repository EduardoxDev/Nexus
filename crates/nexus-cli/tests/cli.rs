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
    assert_eq!(v["schemaVersion"], 2);
    assert_eq!(v["appliedMigrations"], serde_json::json!([1, 2]));
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
    assert!(stdout.contains("Schema:         v2"), "{stdout}");
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
    assert_eq!(v["diagnostics"]["schemaVersion"], 2);
    assert_eq!(v["diagnostics"]["fts5Available"], true);
    assert_eq!(v["integrity"]["ok"], true);
    assert!(v["app"]["coreVersion"].is_string());
    assert!(v["diagnostics"]["migrations"].as_array().unwrap().len() == 2);
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

fn fixture_copy(dst: &std::path::Path) {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/py-service");
    copy_dir(&src, dst);
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).unwrap();
        }
    }
}

fn json_out(cmd: &mut Command) -> Value {
    let out = cmd.assert().success();
    serde_json::from_slice(&out.get_output().stdout).unwrap()
}

#[test]
fn project_lifecycle_add_index_status_remove() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let repo = dir.path().join("repo");
    fixture_copy(&repo);

    let added = json_out(
        nexus(&data)
            .args(["projects", "add", "--index", "--json"])
            .arg(&repo),
    );
    let id = added["project"]["id"].as_i64().unwrap();
    assert_eq!(added["job"]["status"], "succeeded");
    assert_eq!(added["job"]["discovered"], 8);
    assert_eq!(added["job"]["skipped"], 0);

    let list = json_out(nexus(&data).args(["projects", "--json"]));
    assert_eq!(list[0]["counts"]["indexed"], 8);

    // Reindexar por nome: nada muda.
    let job = json_out(nexus(&data).args(["index", "repo", "--json"]));
    assert_eq!(job["processed"], 0);
    assert_eq!(job["unchanged"], 8);

    std::fs::write(
        repo.join("app/novo.py"),
        "def nova_funcao():\n    return 1\n",
    )
    .unwrap();
    let job = json_out(nexus(&data).args(["index", &id.to_string(), "--json"]));
    assert_eq!(job["processed"], 1);

    let status = json_out(nexus(&data).args(["status", "repo", "--json"]));
    assert_eq!(status[0]["counts"]["indexed"], 9);
    assert_eq!(status[0]["project"]["indexGeneration"], 2);

    // Remoção exige confirmação explícita.
    nexus(&data)
        .args(["projects", "remove", "repo"])
        .assert()
        .code(2);
    let removed = json_out(nexus(&data).args(["projects", "remove", "repo", "--yes", "--json"]));
    assert_eq!(removed["id"], id);
    assert!(
        repo.join("app/payments/service.py").is_file(),
        "arquivos originais intactos"
    );
    let list = json_out(nexus(&data).args(["projects", "--json"]));
    assert_eq!(list, serde_json::json!([]));
}

#[test]
fn unknown_project_exits_with_2() {
    let dir = tempfile::tempdir().unwrap();
    let out = nexus(dir.path())
        .args(["index", "inexistente", "--json"])
        .assert()
        .code(2);
    let v: Value = serde_json::from_slice(&out.get_output().stderr).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[test]
fn human_index_output_reports_counts() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    fixture_copy(&repo);
    let out = nexus(&dir.path().join("data"))
        .args(["projects", "add", "--index"])
        .arg(&repo)
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Indexação concluída"), "{stdout}");
    assert!(stdout.contains("Encontrados 8"), "{stdout}");
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
