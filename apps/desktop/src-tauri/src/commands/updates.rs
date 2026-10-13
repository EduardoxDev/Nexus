//! Atualizações: a única conexão de rede do app, e só com permissão (ver docs/privacy.md).
//! A consulta e a verificação da assinatura (minisign) são feitas aqui pelo plugin; o webview
//! não recebe nenhuma permissão do plugin nem acesso à rede.

use nexus_core::ops::kinds;
use nexus_core::settings::SettingKey;
use nexus_core::time_util::now_ms;
use nexus_core::updates::{UpdateInfo, UpdateStatus, should_check_automatically};
use nexus_core::{Error, ErrorInfo};
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use crate::state::{AppState, blocking};

fn unreachable(e: impl std::fmt::Display) -> ErrorInfo {
    ErrorInfo::from(Error::Internal(format!(
        "não foi possível consultar as atualizações no GitHub: {e}"
    )))
}

fn record(state: &AppState, kind: &'static str, detail: serde_json::Value) {
    if let Ok(core) = state.core()
        && let Err(e) = core.record_event(kind, detail)
    {
        tracing::warn!(error = %e, kind, "não foi possível registrar a operação");
    }
}

/// `manual` = botão "Verificar agora" (vale mesmo com a preferência desligada).
/// Automática (ao abrir): só com a preferência ligada e no máximo uma vez a cada 24 h.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    state: State<'_, AppState>,
    manual: bool,
) -> Result<UpdateStatus, ErrorInfo> {
    if !manual {
        let settings = blocking(&state, |core| core.settings()).await?;
        if !should_check_automatically(
            settings.update_check,
            settings.update_last_check_ms,
            now_ms(),
        ) {
            return Ok(UpdateStatus::Skipped);
        }
        // Marca antes de consultar: sem rede, não tenta de novo a cada abertura.
        blocking(&state, |core| {
            core.set_setting(SettingKey::UpdatesLastCheckMs, serde_json::json!(now_ms()))
        })
        .await?;
    }
    let found = app
        .updater()
        .map_err(unreachable)?
        .check()
        .await
        .map_err(unreachable);
    let status = match found {
        Ok(Some(update)) => UpdateStatus::Available(UpdateInfo {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        }),
        Ok(None) => UpdateStatus::UpToDate {
            current_version: app.package_info().version.to_string(),
        },
        Err(e) => {
            record(
                &state,
                kinds::UPDATE_CHECK,
                serde_json::json!({ "manual": manual, "result": "error" }),
            );
            return Err(e);
        }
    };
    let result = match &status {
        UpdateStatus::Available(info) => info.version.clone(),
        _ => "up_to_date".to_owned(),
    };
    record(
        &state,
        kinds::UPDATE_CHECK,
        serde_json::json!({ "manual": manual, "result": result }),
    );
    Ok(status)
}

/// Baixa, confere a assinatura, instala e reinicia o app. Só roda por clique da pessoa.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), ErrorInfo> {
    let update = app
        .updater()
        .map_err(unreachable)?
        .check()
        .await
        .map_err(unreachable)?
        .ok_or_else(|| {
            ErrorInfo::from(Error::NotFound("não há versão nova para instalar".into()))
        })?;
    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| {
            ErrorInfo::from(Error::Internal(format!(
                "a atualização para {version} falhou (nada foi alterado): {e}"
            )))
        })?;
    record(
        &state,
        kinds::UPDATE_INSTALL,
        serde_json::json!({ "version": version }),
    );
    app.restart();
}
