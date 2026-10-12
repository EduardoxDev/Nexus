//! JSON-RPC 2.0 e constantes do MCP.
//!
//! O servidor é *dual-era* (especificação 2026-07-28, "Versioning and Compatibility"):
//! - **moderno** (`2026-07-28`): sem handshake; cada requisição traz a versão em
//!   `_meta["io.modelcontextprotocol/protocolVersion"]`; `server/discover` anuncia versões;
//! - **legado** (`2025-11-25` e anteriores): handshake `initialize` que vale para o processo.

use serde_json::{Value, json};

/// Revisão moderna implementada.
pub const MODERN_VERSION: &str = "2026-07-28";
/// Revisões legadas aceitas no `initialize`, da mais nova para a mais antiga.
pub const LEGACY_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];

pub const META_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
pub const META_CLIENT_INFO: &str = "io.modelcontextprotocol/clientInfo";
pub const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
/// `UnsupportedProtocolVersionError` (faixa reservada ao MCP desde 2026-07-28).
pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// Todas as versões suportadas, para `server/discover` e erros de versão.
pub fn supported_versions() -> Vec<&'static str> {
    std::iter::once(MODERN_VERSION)
        .chain(LEGACY_VERSIONS.iter().copied())
        .collect()
}

/// Erro JSON-RPC (nível de protocolo).
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn unsupported_version(requested: Option<&str>) -> Self {
        Self::new(UNSUPPORTED_PROTOCOL_VERSION, "Unsupported protocol version").with_data(json!({
            "supported": supported_versions(),
            "requested": requested,
        }))
    }

    pub fn to_json(&self) -> Value {
        let mut e = json!({ "code": self.code, "message": self.message });
        if let Some(d) = &self.data {
            e["data"] = d.clone();
        }
        e
    }
}

pub fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error_response(id: &Value, error: &RpcError) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": error.to_json() })
}
