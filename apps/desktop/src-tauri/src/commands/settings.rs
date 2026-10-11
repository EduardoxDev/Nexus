use nexus_core::ErrorInfo;
use nexus_core::settings::{SettingKey, Settings};
use tauri::State;

use crate::state::{AppState, blocking};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, ErrorInfo> {
    blocking(&state, |core| core.settings()).await
}

/// `key` só aceita chaves da allowlist (a desserialização rejeita as demais);
/// `value` é validado por chave no core.
#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: SettingKey,
    value: serde_json::Value,
) -> Result<Settings, ErrorInfo> {
    blocking(&state, move |core| core.set_setting(key, value)).await
}
