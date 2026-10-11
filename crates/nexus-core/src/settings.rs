//! Configurações persistidas. Somente chaves da allowlist, cada uma com validação própria.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::time_util::now_ms;

/// Chaves aceitas. Qualquer outra chave é rejeitada na desserialização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SettingKey {
    #[serde(rename = "ui.theme")]
    UiTheme,
    #[serde(rename = "ui.reduced_motion")]
    UiReducedMotion,
    #[serde(rename = "ui.sidebar_collapsed")]
    UiSidebarCollapsed,
}

impl SettingKey {
    pub const ALL: [Self; 3] = [
        Self::UiTheme,
        Self::UiReducedMotion,
        Self::UiSidebarCollapsed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UiTheme => "ui.theme",
            Self::UiReducedMotion => "ui.reduced_motion",
            Self::UiSidebarCollapsed => "ui.sidebar_collapsed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    System,
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MotionPreference {
    #[default]
    System,
    On,
    Off,
}

/// Configurações efetivas (padrões aplicados às chaves ausentes).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: ThemePreference,
    pub reduced_motion: MotionPreference,
    pub sidebar_collapsed: bool,
}

impl Settings {
    fn apply(&mut self, key: SettingKey, value: serde_json::Value) -> Result<()> {
        let invalid = |e: serde_json::Error| {
            Error::InvalidInput(format!("valor inválido para {}: {e}", key.as_str()))
        };
        match key {
            SettingKey::UiTheme => self.theme = serde_json::from_value(value).map_err(invalid)?,
            SettingKey::UiReducedMotion => {
                self.reduced_motion = serde_json::from_value(value).map_err(invalid)?;
            }
            SettingKey::UiSidebarCollapsed => {
                self.sidebar_collapsed = serde_json::from_value(value).map_err(invalid)?;
            }
        }
        Ok(())
    }
}

/// Lê as configurações. Valores corrompidos no banco são ignorados (padrão) e registrados no log.
pub fn load(conn: &Connection) -> Result<Settings> {
    let mut settings = Settings::default();
    let mut stmt = conn.prepare("SELECT key, value_json FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (key, raw) = row?;
        let Some(key) = SettingKey::parse(&key) else {
            tracing::warn!(key, "chave de configuração desconhecida ignorada");
            continue;
        };
        let applied = serde_json::from_str(&raw)
            .map_err(|e| Error::InvalidInput(e.to_string()))
            .and_then(|v| settings.apply(key, v));
        if let Err(e) = applied {
            tracing::warn!(key = key.as_str(), error = %e, "configuração inválida ignorada");
        }
    }
    Ok(settings)
}

/// Valida e grava uma configuração; devolve as configurações efetivas.
pub fn set(conn: &Connection, key: SettingKey, value: serde_json::Value) -> Result<Settings> {
    // Valida antes de gravar.
    Settings::default().apply(key, value.clone())?;
    conn.execute(
        "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![key.as_str(), value.to_string(), now_ms()],
    )?;
    load(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_keys_are_rejected_by_deserialization() {
        let r: std::result::Result<SettingKey, _> = serde_json::from_str("\"fs.root\"");
        assert!(r.is_err());
        let k: SettingKey = serde_json::from_str("\"ui.theme\"").unwrap();
        assert_eq!(k, SettingKey::UiTheme);
    }

    #[test]
    fn values_are_validated_per_key() {
        let mut s = Settings::default();
        assert!(
            s.apply(SettingKey::UiTheme, serde_json::json!("neon"))
                .is_err()
        );
        assert!(
            s.apply(SettingKey::UiSidebarCollapsed, serde_json::json!("yes"))
                .is_err()
        );
        s.apply(SettingKey::UiTheme, serde_json::json!("light"))
            .unwrap();
        assert_eq!(s.theme, ThemePreference::Light);
    }
}
