//! Integrações: dados para conectar agentes ao servidor MCP (`nexus mcp serve`).

use std::path::PathBuf;

use nexus_core::ErrorInfo;
use nexus_core::integration::{ClientSetup, IntegrationInfo, McpToolInfo, SetupKind};
use tauri::State;

use crate::state::AppState;

/// A CLI é procurada ao lado do executável do app (instalação e builds de desenvolvimento).
fn cli_next_to_app() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) { "nexus.exe" } else { "nexus" };
    let candidate = exe.parent()?.join(name);
    candidate.is_file().then_some(candidate)
}

/// Aspas para o shell do usuário (PowerShell/cmd/bash aceitam aspas duplas em caminhos).
fn quote(s: &str) -> String {
    if s.contains([' ', '(', ')', '&']) {
        format!("\"{s}\"")
    } else {
        s.to_owned()
    }
}

fn shell_line(prefix: &str, cli: &str, args: &[String]) -> String {
    let args = args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ");
    format!("{prefix} -- {} {args}", quote(cli))
}

fn json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// Onde fica o arquivo de configuração do Claude Desktop neste sistema.
fn claude_desktop_config_path() -> &'static str {
    if cfg!(windows) {
        r"%APPDATA%\Claude\claude_desktop_config.json"
    } else if cfg!(target_os = "macos") {
        "~/Library/Application Support/Claude/claude_desktop_config.json"
    } else {
        "~/.config/Claude/claude_desktop_config.json"
    }
}

/// Configuração pronta de cada agente. Formatos conferidos na documentação oficial em
/// 2026-10-13 (Claude Code, Codex, Cursor, VS Code, Claude Desktop); ver docs/mcp.md.
pub fn client_setups(cli: &str, args: &[String]) -> Vec<ClientSetup> {
    let setup = |id: &str, name: &str, kind, text: String, hint: String| ClientSetup {
        id: id.to_owned(),
        name: name.to_owned(),
        kind,
        text,
        where_hint: hint,
    };
    let server = serde_json::json!({ "command": cli, "args": args });
    let typed = serde_json::json!({ "type": "stdio", "command": cli, "args": args });
    vec![
        setup(
            "claude-code",
            "Claude Code",
            SetupKind::Command,
            shell_line("claude mcp add nexus", cli, args),
            "Rode uma vez num terminal. O Claude Code inicia o servidor quando precisa e pede aprovação para as ferramentas no primeiro uso.".into(),
        ),
        setup(
            "codex",
            "Codex",
            SetupKind::Command,
            shell_line("codex mcp add nexus", cli, args),
            "Rode uma vez num terminal; o Codex grava em ~/.codex/config.toml (confira com codex mcp list).".into(),
        ),
        setup(
            "cursor",
            "Cursor",
            SetupKind::Json,
            json(&serde_json::json!({ "mcpServers": { "nexus": typed } })),
            "Cole em ~/.cursor/mcp.json (todos os projetos) ou em .cursor/mcp.json (só um projeto). Se o arquivo já tem servidores, junte a entrada \"nexus\" às existentes.".into(),
        ),
        setup(
            "vscode",
            "VS Code",
            SetupKind::Json,
            json(&serde_json::json!({ "servers": { "nexus": typed } })),
            "Cole em .vscode/mcp.json no projeto, ou abra \"MCP: Open User Configuration\" para todos. Use no chat do Copilot em modo agente.".into(),
        ),
        setup(
            "claude-desktop",
            "Claude Desktop",
            SetupKind::Json,
            json(&serde_json::json!({ "mcpServers": { "nexus": server } })),
            format!(
                "Cole em {} (Configurações → Desenvolvedor → Editar configuração) e reinicie o Claude Desktop.",
                claude_desktop_config_path()
            ),
        ),
    ]
}

pub fn integration_info(data_dir: &str, cli: Option<PathBuf>) -> IntegrationInfo {
    let serve_args = vec![
        "--data-dir".to_owned(),
        data_dir.to_owned(),
        "mcp".to_owned(),
        "serve".to_owned(),
    ];
    let cli_path = cli.map(|p| p.display().to_string());
    let claude_command = cli_path.as_ref().map(|cli| {
        format!(
            "claude mcp add nexus -- {} {}",
            quote(cli),
            serve_args
                .iter()
                .map(|a| quote(a))
                .collect::<Vec<_>>()
                .join(" ")
        )
    });
    let clients = cli_path
        .as_deref()
        .map(|cli| client_setups(cli, &serve_args))
        .unwrap_or_default();
    IntegrationInfo {
        clients,
        cli_path,
        data_dir: data_dir.to_owned(),
        serve_args,
        claude_command,
        protocol_versions: nexus_mcp::protocol::supported_versions()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        tools: nexus_mcp::tools::TOOLS
            .iter()
            .map(|t| McpToolInfo {
                name: t.name.to_owned(),
                title: t.title.to_owned(),
                description: t.description.to_owned(),
                read_only: t.read_only,
            })
            .collect(),
    }
}

#[tauri::command]
pub fn get_integration_info(state: State<'_, AppState>) -> Result<IntegrationInfo, ErrorInfo> {
    let core = state.core()?;
    let dir = core.paths().root().display().to_string();
    Ok(integration_info(&dir, cli_next_to_app()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_command_quotes_paths_with_spaces() {
        let info = integration_info(
            r"C:\Users\Ana Maria\AppData\Local\Nexus",
            Some(PathBuf::from(r"C:\Program Files\NEXUS\nexus.exe")),
        );
        assert_eq!(
            info.claude_command.as_deref(),
            Some(
                r#"claude mcp add nexus -- "C:\Program Files\NEXUS\nexus.exe" --data-dir "C:\Users\Ana Maria\AppData\Local\Nexus" mcp serve"#
            )
        );
        assert_eq!(info.tools.len(), 14);
        assert_eq!(info.protocol_versions[0], "2026-07-28");
        assert!(integration_info("x", None).claude_command.is_none());
    }

    #[test]
    fn every_client_gets_the_cli_and_args_with_spaces_intact() {
        let cli = r"C:\Program Files\NEXUS\nexus.exe";
        let data = r"C:\Users\Ana Maria\AppData\Local\Nexus";
        let info = integration_info(data, Some(PathBuf::from(cli)));
        let ids: Vec<_> = info.clients.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            ["claude-code", "codex", "cursor", "vscode", "claude-desktop"]
        );
        for client in &info.clients {
            match client.kind {
                SetupKind::Command => {
                    assert!(
                        client
                            .text
                            .ends_with(&format!(r#"-- "{cli}" --data-dir "{data}" mcp serve"#)),
                        "{}",
                        client.text
                    );
                }
                SetupKind::Json => {
                    let v: serde_json::Value = serde_json::from_str(&client.text).unwrap();
                    let root = if client.id == "vscode" {
                        "servers"
                    } else {
                        "mcpServers"
                    };
                    let nexus = &v[root]["nexus"];
                    assert_eq!(nexus["command"], cli, "{}", client.id);
                    assert_eq!(
                        nexus["args"],
                        serde_json::json!(["--data-dir", data, "mcp", "serve"])
                    );
                    // Cursor e VS Code pedem o tipo do transporte; o Claude Desktop não usa.
                    let typed = client.id != "claude-desktop";
                    assert_eq!(nexus["type"] == "stdio", typed, "{}", client.id);
                }
            }
            assert!(!client.where_hint.is_empty());
        }
        assert_eq!(
            info.clients[1].text.split(" -- ").next(),
            Some("codex mcp add nexus")
        );
        assert!(integration_info("x", None).clients.is_empty());
    }
}
