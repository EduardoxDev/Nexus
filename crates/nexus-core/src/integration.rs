//! Contrato da tela de Integrações: como conectar agentes ao servidor MCP do NEXUS.
//! Só DTOs; o desktop preenche a partir do crate `nexus-mcp`.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    /// `false` = grava dados locais do NEXUS (nunca arquivos do projeto).
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationInfo {
    /// Caminho da CLI `nexus` encontrada ao lado do app (ou `None`).
    pub cli_path: Option<String>,
    pub data_dir: String,
    /// Argumentos do servidor (`--data-dir … mcp serve`).
    pub serve_args: Vec<String>,
    /// Comando pronto para registrar no Claude Code (quando a CLI foi encontrada).
    pub claude_command: Option<String>,
    /// Revisão moderna e revisões legadas aceitas.
    pub protocol_versions: Vec<String>,
    pub tools: Vec<McpToolInfo>,
}
