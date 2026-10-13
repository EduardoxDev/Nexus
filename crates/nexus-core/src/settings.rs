//! Configurações persistidas. Somente chaves da allowlist, cada uma com validação própria.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::time_util::now_ms;
use crate::updates::UpdateCheck;

/// Chaves aceitas. Qualquer outra chave é rejeitada na desserialização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SettingKey {
    #[serde(rename = "ui.theme")]
    UiTheme,
    #[serde(rename = "ui.reduced_motion")]
    UiReducedMotion,
    #[serde(rename = "ui.sidebar_collapsed")]
    UiSidebarCollapsed,
    #[serde(rename = "indexing.threads")]
    IndexingThreads,
    #[serde(rename = "indexing.max_file_size_kb")]
    MaxFileSizeKb,
    #[serde(rename = "indexing.watch")]
    WatchProjects,
    #[serde(rename = "updates.check")]
    UpdatesCheck,
    #[serde(rename = "updates.last_check_ms")]
    UpdatesLastCheckMs,
}

impl SettingKey {
    pub const ALL: [Self; 8] = [
        Self::UiTheme,
        Self::UiReducedMotion,
        Self::UiSidebarCollapsed,
        Self::IndexingThreads,
        Self::MaxFileSizeKb,
        Self::WatchProjects,
        Self::UpdatesCheck,
        Self::UpdatesLastCheckMs,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UiTheme => "ui.theme",
            Self::UiReducedMotion => "ui.reduced_motion",
            Self::UiSidebarCollapsed => "ui.sidebar_collapsed",
            Self::IndexingThreads => "indexing.threads",
            Self::MaxFileSizeKb => "indexing.max_file_size_kb",
            Self::WatchProjects => "indexing.watch",
            Self::UpdatesCheck => "updates.check",
            Self::UpdatesLastCheckMs => "updates.last_check_ms",
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

/// Limites aceitos para as configurações numéricas.
pub const MAX_INDEXING_THREADS: u32 = 16;
pub const MIN_FILE_SIZE_KB: u32 = 16;
pub const MAX_FILE_SIZE_KB: u32 = 10 * 1024;

/// Configurações efetivas (padrões aplicados às chaves ausentes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: ThemePreference,
    pub reduced_motion: MotionPreference,
    pub sidebar_collapsed: bool,
    /// Threads de indexação; 0 = automático (metade dos núcleos).
    pub indexing_threads: u32,
    /// Arquivos maiores que isso não são indexados.
    pub max_file_size_kb: u32,
    /// Reindexar automaticamente quando arquivos dos projetos mudam.
    pub watch_projects: bool,
    /// Consultar o GitHub em busca de versões novas (`Ask` até a pessoa responder).
    pub update_check: UpdateCheck,
    /// Última verificação automática (ms desde a época); 0 = nunca.
    pub update_last_check_ms: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            reduced_motion: MotionPreference::default(),
            sidebar_collapsed: false,
            indexing_threads: 0,
            max_file_size_kb: 1024,
            watch_projects: true,
            update_check: UpdateCheck::default(),
            update_last_check_ms: 0,
        }
    }
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
            SettingKey::IndexingThreads => {
                let n: u32 = serde_json::from_value(value).map_err(invalid)?;
                if n > MAX_INDEXING_THREADS {
                    return Err(Error::InvalidInput(format!(
                        "indexing.threads deve estar entre 0 (automático) e {MAX_INDEXING_THREADS}"
                    )));
                }
                self.indexing_threads = n;
            }
            SettingKey::MaxFileSizeKb => {
                let n: u32 = serde_json::from_value(value).map_err(invalid)?;
                if !(MIN_FILE_SIZE_KB..=MAX_FILE_SIZE_KB).contains(&n) {
                    return Err(Error::InvalidInput(format!(
                        "indexing.max_file_size_kb deve estar entre {MIN_FILE_SIZE_KB} e {MAX_FILE_SIZE_KB}"
                    )));
                }
                self.max_file_size_kb = n;
            }
            SettingKey::WatchProjects => {
                self.watch_projects = serde_json::from_value(value).map_err(invalid)?;
            }
            SettingKey::UpdatesCheck => {
                self.update_check = serde_json::from_value(value).map_err(invalid)?;
            }
            SettingKey::UpdatesLastCheckMs => {
                let ms: i64 = serde_json::from_value(value).map_err(invalid)?;
                if ms < 0 {
                    return Err(Error::InvalidInput(
                        "updates.last_check_ms não pode ser negativo".into(),
                    ));
                }
                self.update_last_check_ms = ms;
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
        assert!(
            s.apply(SettingKey::IndexingThreads, serde_json::json!(17))
                .is_err()
        );
        assert!(
            s.apply(SettingKey::IndexingThreads, serde_json::json!(-1))
                .is_err()
        );
        assert!(
            s.apply(SettingKey::MaxFileSizeKb, serde_json::json!(8))
                .is_err()
        );
        s.apply(SettingKey::MaxFileSizeKb, serde_json::json!(2048))
            .unwrap();
        assert_eq!(s.max_file_size_kb, 2048);
    }
}
