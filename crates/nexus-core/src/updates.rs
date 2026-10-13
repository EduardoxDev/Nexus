//! Verificação de atualizações: preferência da pessoa e a regra de quando consultar.
//! A consulta em si (GitHub Releases, assinatura minisign) fica no desktop, via plugin do Tauri;
//! aqui ficam só a política e os contratos, para serem testados sem rede.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Se o app pode consultar o GitHub em busca de versões novas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdateCheck {
    /// Ainda não respondeu: o app pergunta e, até lá, não consulta nada.
    #[default]
    Ask,
    On,
    Off,
}

/// Intervalo mínimo entre verificações automáticas.
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

/// Verificação automática (ao abrir o app): só com `On` e no máximo uma vez a cada 24 h.
/// A verificação manual ("Verificar agora") não passa por aqui.
pub fn should_check_automatically(pref: UpdateCheck, last_check_ms: i64, now_ms: i64) -> bool {
    pref == UpdateCheck::On && (last_check_ms <= 0 || now_ms - last_check_ms >= CHECK_INTERVAL_MS)
}

/// Versão nova encontrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    /// Notas da release (texto do GitHub), quando houver.
    pub notes: Option<String>,
    /// Data de publicação (RFC 3339), quando houver.
    pub date: Option<String>,
}

/// Resultado de uma verificação.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum UpdateStatus {
    /// Não consultou (preferência desligada/sem resposta, ou já consultou nas últimas 24 h).
    Skipped,
    UpToDate {
        #[serde(rename = "currentVersion")]
        current_version: String,
    },
    Available(UpdateInfo),
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = CHECK_INTERVAL_MS;

    #[test]
    fn only_checks_automatically_when_allowed_and_once_a_day() {
        let now = 10 * DAY;
        assert!(!should_check_automatically(UpdateCheck::Ask, 0, now));
        assert!(!should_check_automatically(UpdateCheck::Off, 0, now));
        assert!(should_check_automatically(UpdateCheck::On, 0, now));
        assert!(!should_check_automatically(
            UpdateCheck::On,
            now - DAY + 1,
            now
        ));
        assert!(should_check_automatically(UpdateCheck::On, now - DAY, now));
    }

    #[test]
    fn status_serializes_with_a_tag() {
        let v = serde_json::to_value(UpdateStatus::UpToDate {
            current_version: "0.2.0".into(),
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "status": "upToDate", "currentVersion": "0.2.0" })
        );
        assert_eq!(
            serde_json::to_value(UpdateStatus::Skipped).unwrap(),
            serde_json::json!({ "status": "skipped" })
        );
    }
}
