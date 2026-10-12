//! Sessões JSON-RPC completas contra o servidor em processo: handshake legado, requisições
//! modernas sem estado, erros de protocolo, todas as ferramentas (saída validada contra o
//! `outputSchema`), limites de escopo e auditoria.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::memory::{Actor, Transition};
use nexus_core::ops::OperationQuery;
use nexus_core::{Core, DataPaths};
use nexus_mcp::Server;
use nexus_mcp::protocol::{LEGACY_VERSIONS, MODERN_VERSION};
use serde_json::{Value, json};

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    core: Core,
}

fn setup() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("acme");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saas-app"),
        &root,
    );
    std::fs::write(root.join(".env.production"), "API_KEY=segredo\n").unwrap();
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(&root).unwrap().id;
    core.index_project(
        pid,
        &IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli),
        &JobControl::new(),
        &NoopObserver,
    )
    .unwrap();
    Env {
        _tmp: tmp,
        root,
        core,
    }
}

fn send(server: &mut Server<'_>, msg: Value) -> Value {
    server
        .handle_line(&msg.to_string())
        .expect("requisição deve ter resposta")
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN_VERSION,
        "io.modelcontextprotocol/clientInfo": { "name": "teste", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn modern(server: &mut Server<'_>, id: i64, method: &str, mut params: Value) -> Value {
    params["_meta"] = meta();
    send(
        server,
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
    )
}

fn call(server: &mut Server<'_>, id: i64, tool: &str, args: Value) -> Value {
    let r = modern(
        server,
        id,
        "tools/call",
        json!({ "name": tool, "arguments": args }),
    );
    assert!(r.get("error").is_none(), "{tool}: {r}");
    r["result"].clone()
}

// ---- Validador de JSON Schema (subconjunto usado pelas ferramentas) ----------------------

fn type_matches(kind: &str, v: &Value) -> bool {
    match kind {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        other => panic!("tipo de esquema não suportado pelo validador: {other}"),
    }
}

fn validate(schema: &Value, v: &Value, path: &str) -> Result<(), String> {
    if let Some(t) = schema.get("type") {
        let ok = match t {
            Value::String(k) => type_matches(k, v),
            Value::Array(ks) => ks.iter().any(|k| type_matches(k.as_str().unwrap(), v)),
            _ => false,
        };
        if !ok {
            return Err(format!("{path}: esperado {t}, veio {v}"));
        }
    }
    if let Some(Value::Array(options)) = schema.get("enum")
        && !options.contains(v)
    {
        return Err(format!("{path}: {v} fora de {options:?}"));
    }
    if let Some(obj) = v.as_object() {
        if let Some(Value::Array(req)) = schema.get("required") {
            for r in req {
                let k = r.as_str().unwrap();
                if !obj.contains_key(k) {
                    return Err(format!("{path}: falta \"{k}\""));
                }
            }
        }
        if let Some(Value::Object(props)) = schema.get("properties") {
            for (k, sub) in props {
                if let Some(child) = obj.get(k) {
                    validate(sub, child, &format!("{path}.{k}"))?;
                }
            }
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                for k in obj.keys() {
                    if !props.contains_key(k) {
                        return Err(format!("{path}: propriedade não declarada \"{k}\""));
                    }
                }
            }
        }
    }
    if let (Some(items), Some(arr)) = (schema.get("items"), v.as_array()) {
        for (i, item) in arr.iter().enumerate() {
            validate(items, item, &format!("{path}[{i}]"))?;
        }
    }
    Ok(())
}

fn tool_defs(server: &mut Server<'_>) -> Vec<Value> {
    let r = modern(server, 900, "tools/list", json!({}));
    r["result"]["tools"].as_array().unwrap().clone()
}

// ---- Testes ------------------------------------------------------------------------------

#[test]
fn legacy_clients_use_the_initialize_handshake() {
    let env = setup();
    let mut s = Server::new(&env.core);
    // Antes do initialize, sem _meta: recusado com orientação.
    let early = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
    );
    assert_eq!(early["error"]["code"], -32600);
    assert!(early["error"]["data"]["supported"].is_array());

    let init = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "claude-code", "version": "2.1" },
        }}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "nexus");
    assert_eq!(
        init["result"]["capabilities"]["tools"]["listChanged"],
        false
    );
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("not instructions")
    );
    assert!(
        s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );

    let list = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    );
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 13);
    assert!(
        list["result"].get("resultType").is_none(),
        "campos modernos não vão para clientes legados"
    );
    let ping = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }),
    );
    assert_eq!(ping["result"], json!({}));

    let r = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "list_projects", "arguments": {} }}),
    );
    assert_eq!(r["result"]["isError"], false);
    assert_eq!(
        r["result"]["structuredContent"]["projects"][0]["name"],
        "acme"
    );
    // O texto é o JSON serializado do conteúdo estruturado (compatibilidade).
    let text: Value =
        serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, r["result"]["structuredContent"]);

    // Versão desconhecida no initialize: oferece a legada mais nova.
    let mut other = Server::new(&env.core);
    let init = send(
        &mut other,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-01-01" }}),
    );
    assert_eq!(init["result"]["protocolVersion"], LEGACY_VERSIONS[0]);
}

#[test]
fn modern_requests_are_stateless_and_versioned() {
    let env = setup();
    let mut s = Server::new(&env.core);
    let d = modern(&mut s, 1, "server/discover", json!({}));
    let r = &d["result"];
    assert_eq!(r["resultType"], "complete");
    assert_eq!(r["supportedVersions"][0], MODERN_VERSION);
    assert!(
        r["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2025-11-25"))
    );
    assert_eq!(
        r["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "nexus"
    );
    assert!(r["ttlMs"].as_i64().unwrap() > 0);
    assert_eq!(r["cacheScope"], "private");

    let list = modern(&mut s, 2, "tools/list", json!({}));
    assert_eq!(list["result"]["resultType"], "complete");
    assert_eq!(list["result"]["cacheScope"], "private");
    // Ordem determinística.
    let names = |v: &Value| -> Vec<String> {
        v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    };
    let again = modern(&mut s, 3, "tools/list", json!({}));
    assert_eq!(names(&list), names(&again));

    let call = modern(
        &mut s,
        4,
        "tools/call",
        json!({ "name": "get_index_status", "arguments": {} }),
    );
    assert_eq!(call["result"]["resultType"], "complete");
    assert_eq!(call["result"]["isError"], false);

    // Versão não suportada: UnsupportedProtocolVersionError com as versões aceitas.
    let bad = send(
        &mut s,
        json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/list", "params": {
            "_meta": { "io.modelcontextprotocol/protocolVersion": "1900-01-01" }
        }}),
    );
    assert_eq!(bad["error"]["code"], -32022);
    assert_eq!(bad["error"]["data"]["requested"], "1900-01-01");
    assert_eq!(bad["error"]["data"]["supported"][0], MODERN_VERSION);
}

#[test]
fn protocol_errors_follow_json_rpc() {
    let env = setup();
    let mut s = Server::new(&env.core);
    let parse = s.handle_line("{nao é json").unwrap();
    assert_eq!(parse["error"]["code"], -32700);
    assert_eq!(parse["id"], Value::Null);
    let batch = s.handle_line("[]").unwrap();
    assert_eq!(batch["error"]["code"], -32600);
    let no_version = s.handle_line(r#"{"id":1,"method":"ping"}"#).unwrap();
    assert_eq!(no_version["error"]["code"], -32600);
    let unknown = modern(&mut s, 2, "resources/list", json!({}));
    assert_eq!(unknown["error"]["code"], -32601);
    let tool = modern(
        &mut s,
        3,
        "tools/call",
        json!({ "name": "delete_everything", "arguments": {} }),
    );
    assert_eq!(tool["error"]["code"], -32602);
    assert!(
        tool["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool")
    );
    let args = modern(
        &mut s,
        4,
        "tools/call",
        json!({ "name": "list_projects", "arguments": [1] }),
    );
    assert_eq!(args["error"]["code"], -32602);
    let params = s
        .handle_line(r#"{"jsonrpc":"2.0","id":5,"method":"tools/list","params":3}"#)
        .unwrap();
    assert_eq!(params["error"]["code"], -32602);
    // Notificação desconhecida e resposta do cliente: sem resposta.
    assert!(
        s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{}}"#)
            .is_none()
    );
    assert!(
        s.handle_line(r#"{"jsonrpc":"2.0","id":9,"result":{}}"#)
            .is_none()
    );
}

#[test]
fn every_tool_output_matches_its_output_schema() {
    let env = setup();
    // Memória confirmada para as ferramentas de memória e decisões terem conteúdo.
    let pid = env.core.list_projects().unwrap()[0].project.id;
    env.core.import_adrs(pid).unwrap();
    let first = env
        .core
        .list_memories(&nexus_core::memory::MemoryQuery {
            project_id: pid,
            ..Default::default()
        })
        .unwrap()
        .remove(0);
    env.core
        .transition_memory(&first.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();

    let mut s = Server::new(&env.core);
    let defs = tool_defs(&mut s);
    let schema = |name: &str| -> Value {
        defs.iter().find(|d| d["name"] == name).unwrap()["outputSchema"].clone()
    };
    for d in &defs {
        assert_eq!(d["inputSchema"]["type"], "object", "{}", d["name"]);
        assert_eq!(d["outputSchema"]["type"], "object", "{}", d["name"]);
        assert_eq!(d["annotations"]["destructiveHint"], false);
        assert!(d["description"].as_str().unwrap().len() > 40);
    }

    let pack = call(
        &mut s,
        100,
        "build_context_pack",
        json!({ "task": "adicionar login com Google", "tokenBudget": 6000 }),
    );
    let pack_id = pack["structuredContent"]["packId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        pack["structuredContent"]["export"]
            .as_str()
            .unwrap()
            .starts_with("# Context Pack")
    );

    let cases: Vec<(&str, Value)> = vec![
        ("list_projects", json!({})),
        ("get_project_overview", json!({ "project": "acme" })),
        (
            "search_project",
            json!({ "query": "cálculo de frete", "limit": 5 }),
        ),
        (
            "get_file_context",
            json!({ "path": "src/checkout/shipping.ts", "startLine": 1, "endLine": 5 }),
        ),
        ("get_symbol_context", json!({ "symbol": "createSession" })),
        (
            "get_related_files",
            json!({ "path": "src/auth/session.ts" }),
        ),
        ("get_project_memory", json!({ "includeProposals": true })),
        ("get_project_decisions", json!({})),
        ("get_stale_context", json!({})),
        ("get_index_status", json!({ "project": pid })),
        ("refresh_project_index", json!({})),
        ("get_context_sources", json!({ "packId": pack_id })),
        (
            "build_context_pack",
            json!({ "task": "regras de dinheiro", "format": "json" }),
        ),
    ];
    assert_eq!(cases.len(), 13, "uma chamada por ferramenta");
    for (i, (tool, args)) in cases.iter().enumerate() {
        let r = call(&mut s, 200 + i as i64, tool, args.clone());
        assert_eq!(r["isError"], false, "{tool}: {r}");
        validate(&schema(tool), &r["structuredContent"], tool)
            .unwrap_or_else(|e| panic!("{tool} não segue o outputSchema: {e}"));
    }

    // Conteúdo concreto em algumas delas.
    let sym = call(
        &mut s,
        300,
        "get_symbol_context",
        json!({ "symbol": "createSession" }),
    );
    let def = &sym["structuredContent"]["definitions"][0];
    assert_eq!(def["relPath"], "src/auth/session.ts");
    assert!(def["text"].as_str().unwrap().contains("createSession"));
    assert!(
        sym["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .contains(&json!("src/server/routes/auth.routes.ts"))
    );
    let related = call(
        &mut s,
        301,
        "get_related_files",
        json!({ "path": "src/auth/session.ts" }),
    );
    let rel = related["structuredContent"]["related"].as_array().unwrap();
    assert!(
        rel.iter()
            .any(|r| r["relPath"] == "tests/auth/session.test.ts" && r["relation"] == "test")
    );
    assert!(rel.iter().any(|r| r["relation"] == "mentions_this"));
    let mem = call(&mut s, 302, "get_project_memory", json!({}));
    let confirmed = mem["structuredContent"]["memories"].as_array().unwrap();
    assert_eq!(confirmed.len(), 1, "por padrão só confirmadas");
    assert_eq!(confirmed[0]["status"], "confirmed");
    let dec = call(&mut s, 303, "get_project_decisions", json!({}));
    assert_eq!(
        dec["structuredContent"]["decisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        dec["structuredContent"]["proposals"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn scope_and_input_errors_are_tool_errors() {
    let env = setup();
    let mut s = Server::new(&env.core);
    let err = |s: &mut Server<'_>, id: i64, tool: &str, args: Value| -> String {
        let r = call(s, id, tool, args);
        assert_eq!(r["isError"], true, "{tool}: {r}");
        r["content"][0]["text"].as_str().unwrap().to_owned()
    };
    // Arquivo sensível: existe no projeto, mas nunca é lido.
    let msg = err(
        &mut s,
        1,
        "get_file_context",
        json!({ "path": ".env.production" }),
    );
    assert!(msg.contains("não encontrado"), "{msg}");
    assert!(!msg.contains("segredo"));
    let msg = err(
        &mut s,
        2,
        "get_file_context",
        json!({ "path": "../fora.txt" }),
    );
    assert!(
        msg.contains("não encontrado") || msg.contains("inválid"),
        "{msg}"
    );
    let msg = err(
        &mut s,
        3,
        "search_project",
        json!({ "query": "x", "extra": 1 }),
    );
    assert!(msg.contains("unknown argument"), "{msg}");
    let msg = err(
        &mut s,
        4,
        "search_project",
        json!({ "query": "x", "limit": 500 }),
    );
    assert!(msg.contains("between 1 and 50"), "{msg}");
    let msg = err(
        &mut s,
        5,
        "get_project_overview",
        json!({ "project": "outro" }),
    );
    assert!(msg.contains("não encontrado"), "{msg}");
    let msg = err(
        &mut s,
        6,
        "get_symbol_context",
        json!({ "symbol": "a b; drop" }),
    );
    assert!(msg.contains("identificador"), "{msg}");
    let msg = err(
        &mut s,
        7,
        "get_context_sources",
        json!({ "packId": "../../etc" }),
    );
    assert!(msg.contains("invalid packId"), "{msg}");
    let msg = err(
        &mut s,
        8,
        "get_project_memory",
        json!({ "kinds": ["segredo"] }),
    );
    assert!(msg.contains("unknown memory kind"), "{msg}");

    // Atualização do índice: limitada por frequência.
    let ok = call(&mut s, 9, "refresh_project_index", json!({}));
    assert_eq!(ok["isError"], false);
    let msg = err(&mut s, 10, "refresh_project_index", json!({}));
    assert!(msg.contains("try again"), "{msg}");

    // Com dois projetos, "project" passa a ser obrigatório.
    let other = env.root.parent().unwrap().join("outro");
    std::fs::create_dir_all(&other).unwrap();
    env.core.add_project(&other).unwrap();
    let msg = err(&mut s, 11, "search_project", json!({ "query": "frete" }));
    assert!(msg.contains("several projects"), "{msg}");
}

#[test]
fn tool_calls_are_audited_without_arguments() {
    let env = setup();
    let mut s = Server::new(&env.core);
    call(
        &mut s,
        1,
        "search_project",
        json!({ "query": "consulta secreta xyz" }),
    );
    call(
        &mut s,
        2,
        "get_file_context",
        json!({ "path": "nao/existe.ts" }),
    );
    let ops = env
        .core
        .list_operations(&OperationQuery {
            limit: Some(50),
            kind_prefix: Some("mcp.".into()),
            status: None,
        })
        .unwrap();
    let tools: Vec<&Value> = ops
        .iter()
        .filter(|o| o.kind == "mcp.tool")
        .filter_map(|o| o.detail.as_ref())
        .collect();
    assert_eq!(tools.len(), 2);
    assert!(
        tools
            .iter()
            .any(|d| d["tool"] == "search_project" && d["isError"] == false)
    );
    assert!(
        tools
            .iter()
            .any(|d| d["tool"] == "get_file_context" && d["isError"] == true)
    );
    let all = serde_json::to_string(&ops).unwrap();
    assert!(
        !all.contains("consulta secreta"),
        "argumentos não vão para o histórico"
    );
    assert_eq!(ops.iter().filter(|o| o.kind == "mcp.session").count(), 1);
}
