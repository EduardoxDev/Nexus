//! Detecção preventiva de arquivos sensíveis. Arquivos detectados são registrados como
//! `skipped_sensitive` **sem conteúdo** (nem hash): só o caminho e o motivo.
//! A detecção é conservadora e baseada em padrões conhecidos — não é um scanner de segredos completo.

use std::sync::LazyLock;

use regex::Regex;

/// Motivo legível da exclusão. Nunca contém o segredo em si.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveMatch {
    pub reason: String,
}

/// Arquivos de exemplo/modelo comuns que, por convenção, não contêm segredos reais.
/// Ainda passam pela verificação de conteúdo.
const TEMPLATE_SUFFIXES: &[&str] = &[".example", ".sample", ".template", ".dist", ".defaults"];

const SENSITIVE_NAMES: &[&str] = &[
    ".env",
    ".netrc",
    "_netrc",
    ".npmrc",
    ".pypirc",
    ".git-credentials",
    ".htpasswd",
    ".dockercfg",
    "credentials",
    "credentials.json",
    "credentials.yml",
    "credentials.yaml",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "terraform.tfstate",
    "terraform.tfstate.backup",
];

const SENSITIVE_EXTENSIONS: &[&str] = &[
    "pem", "key", "p12", "pfx", "jks", "keystore", "ppk", "kdbx", "tfvars", "asc", "gpg",
];

/// Verificação pelo caminho relativo (sem ler o arquivo).
pub fn check_name(rel_path: &str) -> Option<SensitiveMatch> {
    let name = rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path)
        .to_ascii_lowercase();
    if TEMPLATE_SUFFIXES.iter().any(|s| name.ends_with(s)) {
        return None;
    }
    let reason = |what: &str| {
        Some(SensitiveMatch {
            reason: format!("nome corresponde a padrão de credencial ({what})"),
        })
    };
    if SENSITIVE_NAMES.contains(&name.as_str()) {
        return reason(&name);
    }
    if name.starts_with(".env.") || name.ends_with(".env") {
        return reason(".env*");
    }
    if name.starts_with("secrets.") || name.starts_with("secret.") {
        return reason("secrets.*");
    }
    if name.starts_with("service-account") && name.ends_with(".json") {
        return reason("service-account*.json");
    }
    if let Some((_, ext)) = name.rsplit_once('.')
        && SENSITIVE_EXTENSIONS.contains(&ext)
    {
        return reason(&format!("*.{ext}"));
    }
    if rel_path.to_ascii_lowercase().contains(".ssh/") {
        return reason(".ssh/");
    }
    None
}

struct ContentRule {
    label: &'static str,
    regex: Regex,
}

static CONTENT_RULES: LazyLock<Vec<ContentRule>> = LazyLock::new(|| {
    let rules: [(&str, &str); 9] = [
        (
            "chave privada",
            r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY( BLOCK)?-----",
        ),
        ("chave de acesso AWS", r"\b(AKIA|ASIA)[0-9A-Z]{16}\b"),
        ("token do GitHub", r"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
        ("token do GitHub", r"\bgithub_pat_[A-Za-z0-9_]{22,}\b"),
        ("token do Slack", r"\bxox[abposr]-[A-Za-z0-9-]{10,}"),
        (
            "chave secreta do Stripe",
            r"\b[sr]k_live_[0-9A-Za-z]{20,}\b",
        ),
        ("chave de API do Google", r"\bAIza[0-9A-Za-z_\-]{35}\b"),
        ("chave de API da Anthropic", r"\bsk-ant-[A-Za-z0-9_\-]{20,}"),
        (
            "chave de API da OpenAI",
            r"\bsk-(proj-)?[A-Za-z0-9_\-]{40,}",
        ),
    ];
    rules
        .into_iter()
        .filter_map(|(label, pattern)| {
            Regex::new(pattern)
                .map(|regex| ContentRule { label, regex })
                .ok()
        })
        .collect()
});

/// Verificação do conteúdo textual. Reporta apenas o tipo e a linha, nunca o valor.
pub fn check_content(text: &str) -> Option<SensitiveMatch> {
    for (index, line) in text.lines().enumerate() {
        if line.len() > 4096 {
            continue; // linhas gigantes (minificados) raramente contêm segredos legíveis
        }
        for rule in CONTENT_RULES.iter() {
            if rule.regex.is_match(line) {
                return Some(SensitiveMatch {
                    reason: format!(
                        "conteúdo parece conter {} (linha {})",
                        rule.label,
                        index + 1
                    ),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_names() {
        for path in [
            ".env",
            "config/.env.production",
            "deploy/prod.env",
            "keys/server.pem",
            "certs/client.pfx",
            "home/.ssh/config",
            "id_rsa",
            ".npmrc",
            "infra/terraform.tfstate",
            "secrets.yaml",
            "gcp/service-account-prod.json",
        ] {
            assert!(check_name(path).is_some(), "{path} deveria ser sensível");
        }
        for path in [
            ".env.example",
            "src/env.ts",
            "docs/keys.md",
            "id_rsa.pub",
            "src/key.rs",
        ] {
            assert!(
                check_name(path).is_none(),
                "{path} não deveria ser sensível"
            );
        }
    }

    #[test]
    fn sensitive_content_reports_kind_and_line_without_value() {
        let key = format!("{}{}", "AKIA", "ABCDEFGHIJKLMNOP");
        let text = format!("region = 'us-east-1'\naws_key = '{key}'\n");
        let found = check_content(&text).expect("deve detectar");
        assert_eq!(
            found.reason,
            "conteúdo parece conter chave de acesso AWS (linha 2)"
        );
        assert!(!found.reason.contains(&key));

        let pk = format!("-----BEGIN {} PRIVATE KEY-----\nMIIE...", "RSA");
        assert!(check_content(&pk).is_some());
    }

    #[test]
    fn ordinary_code_is_not_flagged() {
        let code = "const apiKey = process.env.API_KEY;\nfn sk_live_mode() {}\nlet x = \"AKIA\";";
        assert!(check_content(code).is_none());
    }
}
