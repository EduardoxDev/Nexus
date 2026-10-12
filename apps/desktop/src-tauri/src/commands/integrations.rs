//! Integrações: dados para conectar agentes ao servidor MCP (`nexus mcp serve`).

use std::path::PathBuf;

use nexus_core::ErrorInfo;
use nexus_core::integration::{IntegrationInfo, McpToolInfo};
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
    IntegrationInfo {
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
        assert_eq!(info.tools.len(), 13);
        assert_eq!(info.protocol_versions[0], "2026-07-28");
        assert!(integration_info("x", None).claude_command.is_none());
    }
}
