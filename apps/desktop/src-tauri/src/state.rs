//! Estado compartilhado do app: o core (ou o motivo de não ter aberto).

use std::sync::Arc;

use nexus_core::diagnostics::StartupStatus;
use nexus_core::{Core, Error, ErrorCode, ErrorInfo};

pub struct AppState {
    core: Result<Arc<Core>, ErrorInfo>,
    startup: StartupStatus,
}

impl AppState {
    pub fn new(core: Result<Core, (Option<String>, Error)>) -> Self {
        match core {
            Ok(core) => {
                let startup = StartupStatus::from_core(&core);
                Self {
                    core: Ok(Arc::new(core)),
                    startup,
                }
            }
            Err((data_dir, err)) => {
                tracing::error!(error = %err, "o core não pôde ser aberto");
                Self {
                    startup: StartupStatus::failed(data_dir, &err),
                    core: Err(err.to_info()),
                }
            }
        }
    }

    pub fn startup(&self) -> &StartupStatus {
        &self.startup
    }

    pub fn core(&self) -> Result<Arc<Core>, ErrorInfo> {
        self.core.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_core_is_reported_and_commands_are_refused() {
        let err = Error::IncompatibleSchema {
            found: 9,
            supported: 1,
        };
        let state = AppState::new(Err((Some("C:\\dados".into()), err)));
        let startup = state.startup();
        assert!(!startup.ready);
        assert_eq!(
            startup.error.as_ref().map(|e| e.code),
            Some(ErrorCode::IncompatibleSchema)
        );
        let refused = tauri::async_runtime::block_on(blocking(&state, |core| core.settings()));
        assert_eq!(refused.unwrap_err().code, ErrorCode::IncompatibleSchema);
    }

    #[test]
    fn errors_from_core_keep_their_code_and_op_id() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(nexus_core::DataPaths::at(dir.path())).unwrap();
        let state = AppState::new(Ok(core));
        let result = tauri::async_runtime::block_on(blocking(&state, |core| {
            core.set_setting(
                nexus_core::settings::SettingKey::UiTheme,
                serde_json::json!("neon"),
            )
        }));
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert!(err.op_id.is_some());
    }

    #[test]
    fn unknown_setting_keys_are_rejected_at_the_ipc_boundary() {
        let parsed: Result<nexus_core::settings::SettingKey, _> =
            serde_json::from_value(serde_json::json!("fs.scope"));
        assert!(parsed.is_err());
    }
}

/// Executa uma chamada ao core fora da thread do webview.
pub async fn blocking<T, F>(state: &AppState, f: F) -> Result<T, ErrorInfo>
where
    T: Send + 'static,
    F: FnOnce(&Core) -> nexus_core::Result<T> + Send + 'static,
{
    let core = state.core()?;
    tauri::async_runtime::spawn_blocking(move || f(&core))
        .await
        .map_err(|e| ErrorInfo {
            code: ErrorCode::Internal,
            message: format!("tarefa interna falhou: {e}"),
            op_id: None,
        })?
        .map_err(|e| {
            tracing::warn!(code = e.code().as_str(), error = %e, "comando falhou");
            e.to_info()
        })
}
