//! Garante que a superfície exposta ao webview é exatamente a declarada:
//! comandos em `build.rs` ⇔ permissões em `capabilities/default.json` ⇔ `generate_handler!`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn declared_commands() -> BTreeSet<String> {
    let build = std::fs::read_to_string(manifest_dir().join("build.rs")).unwrap();
    let start = build.find("const COMMANDS").unwrap();
    let end = start + build[start..].find("];").unwrap();
    build[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn capabilities_allow_exactly_the_declared_commands() {
    let raw = std::fs::read_to_string(manifest_dir().join("capabilities/default.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let permissions: Vec<&str> = json["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().expect("permissões são strings simples"))
        .collect();

    let allowed: BTreeSet<String> = permissions
        .iter()
        .filter_map(|p| p.strip_prefix("allow-"))
        .map(|p| p.replace('-', "_"))
        .collect();
    assert_eq!(allowed, declared_commands());

    // Nenhum plugin com acesso amplo pode ser exposto ao webview.
    for p in &permissions {
        for forbidden in ["fs:", "shell:", "http:", "process:", "opener:"] {
            assert!(!p.starts_with(forbidden), "permissão proibida: {p}");
        }
    }
    let core: Vec<&&str> = permissions
        .iter()
        .filter(|p| p.starts_with("core:"))
        .collect();
    assert_eq!(
        core,
        vec![&"core:event:default"],
        "apenas eventos do core são permitidos"
    );
}

#[test]
fn every_declared_command_is_registered_in_the_handler() {
    let lib = std::fs::read_to_string(manifest_dir().join("src/lib.rs")).unwrap();
    let start = lib.find("generate_handler![").unwrap();
    let end = start + lib[start..].find("])").unwrap();
    let registered: BTreeSet<String> = lib[start..end]
        .split(|c: char| c == ',' || c.is_whitespace() || c == '[')
        .filter_map(|s| s.rsplit("::").next())
        .filter(|s| !s.is_empty() && *s != "generate_handler!")
        .map(str::to_owned)
        .collect();
    assert_eq!(registered, declared_commands());
}

#[test]
fn global_tauri_api_is_disabled_and_csp_is_strict() {
    let raw = std::fs::read_to_string(manifest_dir().join("tauri.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(conf["app"]["withGlobalTauri"], false);
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(!csp.contains("unsafe-eval"));
    // Trusted Types: nenhuma atribuição de HTML/script por string (innerHTML etc.) é aceita.
    assert!(csp.contains("require-trusted-types-for 'script'"));
    assert!(csp.contains("trusted-types 'none'"));
    assert!(csp.contains("object-src 'none'"));
    assert!(csp.contains("frame-ancestors 'none'"));
    assert!(!csp.contains("http://") || csp.contains("http://ipc.localhost"));
    for part in csp.split(';') {
        assert!(
            !part.contains("https:"),
            "CSP não pode liberar origens remotas: {part}"
        );
        assert!(!part.contains('*'), "CSP não pode usar curingas: {part}");
    }
}
