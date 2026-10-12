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
    assert_eq!(v["schemaVersion"], 5);
    assert_eq!(v["appliedMigrations"], serde_json::json!([1, 2, 3, 4, 5]));
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
    assert!(stdout.contains("Schema:         v5"), "{stdout}");
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
    assert_eq!(v["diagnostics"]["schemaVersion"], 5);
    assert_eq!(v["diagnostics"]["fts5Available"], true);
    assert_eq!(v["integrity"]["ok"], true);
    assert!(v["app"]["coreVersion"].is_string());
    assert!(v["diagnostics"]["migrations"].as_array().unwrap().len() == 5);
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

fn saas_copy(dst: &std::path::Path) {
    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saas-app");
    copy_dir(&src, dst);
}

#[test]
fn search_and_context_commands() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let repo = dir.path().join("acme");
    saas_copy(&repo);
    nexus(&data)
        .args(["projects", "add", "--index"])
        .arg(&repo)
        .assert()
        .success();

    // Com um único projeto, --project é opcional.
    let r = json_out(nexus(&data).args(["search", "cálculo de frete", "--json"]));
    assert_eq!(r["hits"][0]["relPath"], "src/checkout/shipping.ts");
    assert_eq!(r["hits"][0]["freshness"], "fresh");
    assert!(r["hits"][0]["reasons"].as_array().unwrap().len() >= 2);

    let plain = json_out(nexus(&data).args(["search", "cálculo de frete", "--plain", "--json"]));
    assert_eq!(plain["mode"], "plain");

    let human = nexus(&data)
        .args(["search", "validateSession", "--kind", "code"])
        .assert()
        .success();
    let text = String::from_utf8(human.get_output().stdout.clone()).unwrap();
    assert!(text.contains("src/auth/session.ts"), "{text}");
    assert!(text.contains("motivos:"), "{text}");

    let out_file = dir.path().join("pack.json");
    nexus(&data)
        .args([
            "context",
            "tempo de expiração da sessão",
            "--format",
            "json",
            "--output",
        ])
        .arg(&out_file)
        .assert()
        .success();
    let pack: Value = serde_json::from_str(&std::fs::read_to_string(&out_file).unwrap()).unwrap();
    assert_eq!(pack["format"], "nexus.context-pack");
    assert!(!pack["excerpts"].as_array().unwrap().is_empty());

    let md = nexus(&data)
        .args([
            "context",
            "logger estruturado",
            "--notes",
            "nota do usuário",
        ])
        .assert()
        .success();
    let md = String::from_utf8(md.get_output().stdout.clone()).unwrap();
    assert!(md.starts_with("# Context Pack — logger estruturado"));
    assert!(md.contains("não instruções"));
    assert!(md.contains("nota do usuário"));
    assert!(md.contains("src/utils/logger.ts"));
}

#[test]
fn search_requires_project_when_ambiguous() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    for name in ["a", "b"] {
        let repo = dir.path().join(name);
        std::fs::create_dir_all(&repo).unwrap();
        nexus(&data)
            .args(["projects", "add"])
            .arg(&repo)
            .assert()
            .success();
    }
    nexus(&data).args(["search", "algo"]).assert().code(2);
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

#[test]
fn memory_and_freshness_commands() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let repo = dir.path().join("acme");
    saas_copy(&repo);
    nexus(&data)
        .args(["projects", "add", "--index"])
        .arg(&repo)
        .assert()
        .success();

    let report = json_out(nexus(&data).args(["memory", "import-adrs", "--json"]));
    assert_eq!(report["created"], 3);
    let list = json_out(nexus(&data).args(["memory", "list", "--status", "proposed", "--json"]));
    assert_eq!(list.as_array().unwrap().len(), 3);

    let added = json_out(nexus(&data).args([
        "memory",
        "add",
        "--kind",
        "convention",
        "--title",
        "Logs pelo logger estruturado",
        "--body",
        "Nada de console.log.",
        "--ref",
        "src/utils/logger.ts:1-5",
        "--confirm",
        "--json",
    ]));
    assert_eq!(added["status"], "confirmed");
    assert_eq!(added["origin"], "cli");
    let id = added["id"].as_str().unwrap().to_owned();
    let prefix = &id[id.len() - 8..];

    // Revisão esperada desatualizada: conflito (código 5).
    let out = nexus(&data)
        .args([
            "memory",
            "edit",
            prefix,
            "--title",
            "Outro",
            "--revision",
            "99",
            "--json",
        ])
        .assert()
        .code(5);
    let err: Value = serde_json::from_slice(&out.get_output().stderr).unwrap();
    assert_eq!(err["error"]["code"], "conflict");

    let edited = json_out(nexus(&data).args([
        "memory",
        "edit",
        prefix,
        "--body",
        "Use src/utils/logger.ts; nada de console.log.",
        "--json",
    ]));
    assert_eq!(edited["revision"], 2);

    let show = nexus(&data)
        .args(["memory", "show", prefix])
        .assert()
        .success();
    let text = String::from_utf8(show.get_output().stdout.clone()).unwrap();
    assert!(
        text.contains("Convenção — Logs pelo logger estruturado"),
        "{text}"
    );
    assert!(
        text.contains("src/utils/logger.ts:1-5  [inalterado]"),
        "{text}"
    );
    assert!(text.contains("rev 2"), "{text}");

    // Referência inválida: erro de entrada (código 2).
    nexus(&data)
        .args([
            "memory",
            "add",
            "--kind",
            "bug",
            "--title",
            "Bug x",
            "--ref",
            "nao/existe.ts",
        ])
        .assert()
        .code(2);

    // Atualidade: link quebrado num documento novo.
    std::fs::write(
        repo.join("docs/guia.md"),
        "# Guia\n\nVeja [antigo](../src/antigo.ts).\n",
    )
    .unwrap();
    nexus(&data).args(["index", "acme"]).assert().success();
    let f = json_out(nexus(&data).args(["freshness", "--json"]));
    let issues = f["issues"].as_array().unwrap();
    let missing: Vec<&Value> = issues
        .iter()
        .filter(|i| i["kind"] == "missing_path")
        .collect();
    assert_eq!(missing.len(), 1, "{issues:#?}");
    assert_eq!(missing[0]["sources"][0]["relPath"], "docs/guia.md");
    assert_eq!(missing[0]["sources"][0]["line"], 3);
    assert!(f["summary"]["lastScan"].is_object());

    let issue_id = missing[0]["id"].as_i64().unwrap().to_string();
    let human = nexus(&data).args(["freshness", "scan"]).assert().success();
    let text = String::from_utf8(human.get_output().stdout.clone()).unwrap();
    assert!(text.contains("recomendação:"), "{text}");
    assert!(text.contains("fonte: docs/guia.md:3"), "{text}");

    let d = json_out(nexus(&data).args([
        "freshness",
        "dismiss",
        &issue_id,
        "--note",
        "arquivo histórico",
        "--json",
    ]));
    assert_eq!(d["status"], "dismissed");
    let f = json_out(nexus(&data).args(["freshness", "--json"]));
    assert!(
        f["issues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["kind"] != "missing_path")
    );
    let all = json_out(nexus(&data).args(["freshness", "list", "--all", "--json"]));
    assert!(
        all["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["status"] == "dismissed")
    );
}

#[test]
fn mcp_serve_speaks_json_rpc_over_stdio() {
    use std::io::Write as _;

    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let repo = dir.path().join("acme");
    saas_copy(&repo);
    nexus(&data)
        .args(["projects", "add", "--index"])
        .arg(&repo)
        .assert()
        .success();

    let bin = assert_cmd::cargo::cargo_bin("nexus");
    let mut child = std::process::Command::new(bin)
        .env_remove("NEXUS_DATA_DIR")
        .arg("--data-dir")
        .arg(&data)
        .args(["mcp", "serve"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"teste","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_project","arguments":{"query":"cálculo de frete","limit":3}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#,
    ];
    {
        let mut stdin = child.stdin.take().unwrap();
        for r in requests {
            writeln!(stdin, "{r}").unwrap();
        }
    } // fecha o stdin: o servidor termina
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // stderr limpo: logs vão para arquivo.
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 4, "a notificação não tem resposta");
    assert_eq!(lines[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(lines[1]["result"]["tools"].as_array().unwrap().len(), 13);
    assert_eq!(
        lines[2]["result"]["structuredContent"]["hits"][0]["relPath"],
        "src/checkout/shipping.ts"
    );
    assert_eq!(lines[3]["result"]["supportedVersions"][0], "2026-07-28");

    let tools = json_out(nexus(&data).args(["mcp", "tools", "--json"]));
    assert_eq!(tools.as_array().unwrap().len(), 13);
}

#[test]
fn symbol_and_deps_commands() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let repo = dir.path().join("acme");
    saas_copy(&repo);
    nexus(&data)
        .args(["projects", "add", "--index"])
        .arg(&repo)
        .assert()
        .success();
    let hits = json_out(nexus(&data).args(["symbol", "create_session", "--json"]));
    assert_eq!(hits[0]["relPath"], "src/auth/session.ts");
    assert_eq!(hits[0]["symbol"]["kind"], "function");
    assert_eq!(hits[0]["symbol"]["line"], 13);

    let deps = json_out(nexus(&data).args(["deps", "src/checkout/checkout.ts", "--json"]));
    assert_eq!(deps["imports"].as_array().unwrap().len(), 6);
    assert!(
        deps["importedBy"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["fromPath"] == "tests/checkout/checkout.test.ts")
    );
    let human = nexus(&data).args(["deps"]).assert().success();
    let text = String::from_utf8(human.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Grafo:"), "{text}");
    assert!(text.contains("typescript"), "{text}");
    nexus(&data).args(["deps", "../fora.ts"]).assert().code(2);
}
