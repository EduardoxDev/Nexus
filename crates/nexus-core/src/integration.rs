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

/// Como a configuração de um agente é aplicada pela pessoa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SetupKind {
    /// Comando para rodar uma vez num terminal.
    Command,
    /// Trecho JSON para colar no arquivo de configuração do agente.
    Json,
}

/// Configuração pronta de um agente (cliente MCP), com os caminhos desta instalação.
/// O NEXUS só mostra; quem aplica é a pessoa (o app não escreve em arquivos de outros programas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClientSetup {
    pub id: String,
    pub name: String,
    pub kind: SetupKind,
    pub text: String,
    /// Onde rodar ou colar.
    pub where_hint: String,
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
    /// Agentes suportados (vazio quando a CLI não foi encontrada).
    pub clients: Vec<ClientSetup>,
    /// Revisão moderna e revisões legadas aceitas.
    pub protocol_versions: Vec<String>,
    pub tools: Vec<McpToolInfo>,
}
