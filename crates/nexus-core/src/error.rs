//! Erros do core e o envelope serializável entregue a adaptadores (desktop, CLI, MCP).

use serde::Serialize;
use ts_rs::TS;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Código estável de erro, usado em contratos externos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    Storage,
    Io,
    Conflict,
    Cancelled,
    IncompatibleSchema,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::NotFound => "not_found",
            Self::Storage => "storage",
            Self::Io => "io",
            Self::Conflict => "conflict",
            Self::Cancelled => "cancelled",
            Self::IncompatibleSchema => "incompatible_schema",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("entrada inválida: {0}")]
    InvalidInput(String),

    #[error("não encontrado: {0}")]
    NotFound(String),

    #[error("erro no banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("erro de armazenamento: {0}")]
    Storage(String),

    #[error("erro de E/S ({context}): {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "o banco está na versão de schema {found}, mas esta versão do NEXUS conhece até a {supported}"
    )]
    IncompatibleSchema { found: i64, supported: i64 },

    #[error("conflito: {0}")]
    Conflict(String),

    #[error("operação cancelada")]
    Cancelled,

    #[error("erro interno: {0}")]
    Internal(String),

    /// Erro ocorrido dentro de uma operação registrada; carrega o `op_id` para rastreio.
    #[error("{source}")]
    InOperation {
        op_id: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Sqlite(_) | Self::Storage(_) => ErrorCode::Storage,
            Self::Io { .. } => ErrorCode::Io,
            Self::IncompatibleSchema { .. } => ErrorCode::IncompatibleSchema,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Internal(_) => ErrorCode::Internal,
            Self::InOperation { source, .. } => source.code(),
        }
    }

    pub fn op_id(&self) -> Option<&str> {
        match self {
            Self::InOperation { op_id, .. } => Some(op_id),
            _ => None,
        }
    }

    /// Mensagem segura para exibir ao usuário: sem SQL, sem caminhos internos de código.
    pub fn user_message(&self) -> String {
        match self {
            Self::Sqlite(_) => {
                "Falha ao acessar o banco local. Veja o Diagnóstico para detalhes.".to_owned()
            }
            Self::InOperation { source, .. } => source.user_message(),
            other => other.to_string(),
        }
    }

    pub fn to_info(&self) -> ErrorInfo {
        ErrorInfo {
            code: self.code(),
            message: self.user_message(),
            op_id: self.op_id().map(str::to_owned),
        }
    }
}

/// Envelope de erro devolvido aos adaptadores.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInfo {
    pub code: ErrorCode,
    pub message: String,
    pub op_id: Option<String>,
}

impl From<Error> for ErrorInfo {
    fn from(err: Error) -> Self {
        err.to_info()
    }
}

/// Converte um lock envenenado em erro interno em vez de propagar panics.
pub(crate) fn poisoned<T>(_: std::sync::PoisonError<T>) -> Error {
    Error::Internal("estado interno inconsistente após uma falha anterior".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_operation_delegates_code_and_message() {
        let err = Error::InOperation {
            op_id: "op-1".into(),
            source: Box::new(Error::InvalidInput("x".into())),
        };
        assert_eq!(err.code(), ErrorCode::InvalidInput);
        let info = err.to_info();
        assert_eq!(info.op_id.as_deref(), Some("op-1"));
        assert_eq!(info.message, "entrada inválida: x");
    }

    #[test]
    fn sqlite_errors_do_not_leak_sql_to_users() {
        let err = Error::Sqlite(rusqlite::Error::InvalidQuery);
        assert_eq!(err.code(), ErrorCode::Storage);
        assert!(!err.user_message().contains("Query"));
    }
}
