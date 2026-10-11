//! Análise de consultas e tarefas: normalização, stopwords (pt/en), expansão PT→EN de termos
//! técnicos e detecção de intenção. Heurística documentada — não é processamento semântico.

use std::collections::BTreeSet;

use serde::Serialize;
use ts_rs::TS;

use crate::chunk::split_identifier;

/// Um conceito da consulta: o termo original e suas variantes (expansões e subpalavras).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Concept {
    pub term: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// A consulta pede testes.
    Tests,
    /// Documentação, decisões, convenções, arquitetura.
    Docs,
    /// Configuração e ambiente.
    Config,
    /// Pipelines de CI.
    Ci,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QueryAnalysis {
    pub concepts: Vec<Concept>,
    pub intents: Vec<Intent>,
    /// Caminhos ou nomes de arquivo citados literalmente (ex.: `src/auth/login.ts`).
    pub mentioned_paths: Vec<String>,
    /// Palavras descartadas por serem genéricas.
    pub ignored: Vec<String>,
}

impl QueryAnalysis {
    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty() && self.mentioned_paths.is_empty()
    }

    /// Todos os termos (originais + variantes), sem repetição.
    pub fn all_terms(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for c in &self.concepts {
            set.insert(c.term.clone());
            for v in &c.variants {
                set.insert(v.clone());
            }
        }
        set.into_iter().collect()
    }

    /// Só os termos digitados (para a busca simples de referência).
    pub fn original_terms(&self) -> Vec<String> {
        self.concepts.iter().map(|c| c.term.clone()).collect()
    }

    pub fn has(&self, intent: Intent) -> bool {
        self.intents.contains(&intent)
    }
}

/// Minúsculas e sem acentos (equivalente ao `remove_diacritics` do tokenizador FTS).
pub fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

const STOPWORDS: &[&str] = &[
    // português
    "a",
    "o",
    "as",
    "os",
    "de",
    "da",
    "do",
    "das",
    "dos",
    "e",
    "em",
    "no",
    "na",
    "nos",
    "nas",
    "um",
    "uma",
    "uns",
    "umas",
    "para",
    "pra",
    "por",
    "com",
    "sem",
    "que",
    "qual",
    "quais",
    "onde",
    "como",
    "quando",
    "ao",
    "aos",
    "se",
    "ser",
    "esta",
    "estao",
    "fica",
    "ficam",
    "isso",
    "isto",
    "este",
    "esse",
    "essa",
    "aquele",
    "todo",
    "toda",
    "todos",
    "mais",
    "menos",
    "ou",
    "mas",
    "eu",
    "ele",
    "ela",
    "seu",
    "sua",
    "meu",
    "minha",
    "faz",
    "fazem",
    "foi",
    "ha",
    "tem",
    "pelo",
    "pela",
    "sobre",
    "entre",
    "ate",
    "tambem",
    "ja",
    "nao",
    "sim",
    "ter",
    "vai",
    "deve",
    "devem",
    "pode",
    "podem",
    "existe",
    "existente",
    "existentes",
    "quero",
    "preciso",
    "arquivo",
    "arquivos",
    "codigo",
    "codigos",
    "projeto",
    "controla",
    "controlam",
    "implementa",
    "implementam",
    "implementado",
    "responsavel",
    "responsaveis",
    "usado",
    "usada",
    "usa",
    "lugar",
    "parte",
    "partes",
    "respeitando",
    "adicionar",
    "criar",
    "alterar",
    "mudar",
    "verificam",
    "verifica",
    "dependem",
    "depende",
    "trata",
    "tratam",
    "nova",
    "novo",
    // inglês
    "the",
    "an",
    "of",
    "to",
    "in",
    "on",
    "for",
    "with",
    "and",
    "or",
    "is",
    "are",
    "where",
    "what",
    "which",
    "how",
    "does",
    "do",
    "file",
    "files",
    "code",
    "that",
    "this",
    "it",
    "be",
    "by",
    "from",
    "at",
    "as",
    "add",
    "new",
    "into",
    "using",
    "use",
    "uses",
    "handles",
    "handle",
];

/// Dicionário PT→EN (chaves sem acento). Expande o vocabulário de quem pergunta em português
/// para os identificadores, geralmente em inglês.
const EXPANSIONS: &[(&str, &[&str])] = &[
    (
        "autenticacao",
        &["auth", "authentication", "login", "session"],
    ),
    ("autenticar", &["auth", "login", "authenticate"]),
    ("login", &["login", "signin", "auth"]),
    ("sessao", &["session"]),
    ("sessoes", &["session"]),
    ("senha", &["password"]),
    ("senhas", &["password"]),
    ("usuario", &["user"]),
    ("usuarios", &["user", "users"]),
    ("pagamento", &["payment", "charge", "billing"]),
    ("pagamentos", &["payment", "payments", "charge", "billing"]),
    ("cobranca", &["charge", "billing", "invoice"]),
    ("cobrar", &["charge"]),
    ("fatura", &["invoice", "billing"]),
    ("carrinho", &["cart"]),
    ("pedido", &["order"]),
    ("pedidos", &["order", "orders"]),
    ("teste", &["test", "spec"]),
    ("testes", &["test", "tests", "spec"]),
    ("configuracao", &["config", "configuration", "settings"]),
    ("configuracoes", &["config", "settings"]),
    ("banco", &["database", "db"]),
    ("dados", &["data"]),
    ("erro", &["error"]),
    ("erros", &["error", "errors"]),
    ("permissao", &["permission", "role"]),
    ("permissoes", &["permission", "roles"]),
    ("rota", &["route", "router"]),
    ("rotas", &["route", "routes", "router"]),
    ("requisicao", &["request"]),
    ("resposta", &["response"]),
    ("fila", &["queue"]),
    ("notificacao", &["notification", "notify"]),
    ("notificacoes", &["notification", "notify"]),
    ("busca", &["search", "query"]),
    ("pesquisa", &["search"]),
    ("decisao", &["decision", "adr"]),
    ("decisoes", &["decision", "adr"]),
    ("decidida", &["decision", "adr"]),
    ("decidido", &["decision", "adr"]),
    ("convencao", &["convention", "style"]),
    ("convencoes", &["convention", "style"]),
    ("dependencia", &["dependency"]),
    ("dependencias", &["dependency", "dependencies"]),
    ("seguranca", &["security"]),
    ("chave", &["key"]),
    ("ambiente", &["env", "environment"]),
    ("tentativa", &["retry", "attempt"]),
    ("tentativas", &["retry", "attempts"]),
    ("repetir", &["retry"]),
    ("desconto", &["discount", "coupon"]),
    ("cupom", &["coupon", "discount"]),
    ("preco", &["price"]),
    ("precos", &["price", "prices"]),
    ("imposto", &["tax"]),
    ("frete", &["shipping"]),
    ("envio", &["shipping", "send"]),
    ("estoque", &["inventory", "stock"]),
    ("analisador", &["parser", "parse"]),
    ("interpretar", &["parse", "parser"]),
    ("duplicada", &["duplicate"]),
    ("duplicadas", &["duplicate"]),
    ("comentario", &["comment"]),
    ("comentarios", &["comment", "comments"]),
    ("migracao", &["migration"]),
    ("migracoes", &["migration", "migrations"]),
    ("componente", &["component"]),
    ("componentes", &["component", "components"]),
    ("botao", &["button"]),
    ("tela", &["screen", "page", "view"]),
    ("servico", &["service"]),
    ("servicos", &["service", "services"]),
    ("cliente", &["client", "customer"]),
    ("clientes", &["client", "customers"]),
    ("assinatura", &["subscription", "signature"]),
    ("plano", &["plan"]),
    ("planos", &["plan", "plans"]),
    ("email", &["email", "mail"]),
    ("registro", &["log", "register"]),
    ("logs", &["log", "logging"]),
    ("valor", &["value", "amount"]),
    ("total", &["total", "amount"]),
    ("expiracao", &["expires", "expiry", "ttl"]),
    ("expirada", &["expired", "expires"]),
    ("validar", &["validate", "validation"]),
    ("validacao", &["validate", "validation"]),
    ("api", &["api", "route", "handler"]),
    ("tabela", &["table"]),
    ("tabelas", &["table", "tables"]),
    ("dinheiro", &["money", "cents", "price"]),
    ("formatar", &["format"]),
    ("formatacao", &["format", "formatting"]),
    ("exibicao", &["display", "format"]),
    ("exibir", &["display", "show"]),
    ("salvar", &["save", "store", "insert"]),
    ("gravar", &["save", "write", "insert"]),
    ("tempo", &["time", "ttl", "duration", "timeout"]),
    ("limite", &["limit"]),
    ("formulario", &["form"]),
    ("gancho", &["hook"]),
    ("estruturado", &["structured"]),
    ("geracao", &["generate", "generation"]),
    ("numero", &["number"]),
    ("enviar", &["send"]),
    ("confirmacao", &["confirmation", "confirm"]),
    ("calculo", &["calculate", "cost", "compute"]),
    ("visao", &["overview"]),
    ("geral", &["overview", "general"]),
    ("google", &["google", "oauth"]),
    ("social", &["oauth", "provider"]),
];

fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

fn expansions(word: &str) -> &'static [&'static str] {
    EXPANSIONS
        .iter()
        .find(|(k, _)| *k == word)
        .map_or(&[][..], |(_, v)| *v)
}

/// Sufixos flexionais comuns em português (particípios, gerúndios, plurais, infinitivos).
const PT_SUFFIXES: &[&str] = &[
    "acoes", "acao", "adas", "ados", "idas", "idos", "ada", "ado", "ida", "ido", "ando", "endo",
    "indo", "amos", "emos", "ar", "er", "ir", "am", "em", "ou", "as", "os", "es", "a", "o", "s",
];

/// Expansões via radical quando a palavra flexionada não está no dicionário
/// (`validada` → `valid` → entradas `validar`/`validacao`). Radical mínimo de 4 letras.
fn stem_expansions(word: &str) -> Vec<&'static str> {
    for suffix in PT_SUFFIXES {
        let Some(stem) = word.strip_suffix(suffix) else {
            continue;
        };
        if stem.chars().count() < 4 {
            continue;
        }
        let found: Vec<&'static str> = EXPANSIONS
            .iter()
            .filter(|(k, _)| k.starts_with(stem))
            .flat_map(|(_, v)| v.iter().copied())
            .collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

fn intent_of(word: &str) -> Option<Intent> {
    Some(match word {
        "teste" | "testes" | "test" | "tests" | "spec" | "specs" | "cobertura" | "coverage"
        | "testado" | "testada" | "testados" => Intent::Tests,
        "decisao" | "decisoes" | "decidida" | "decidido" | "adr" | "adrs" | "documentacao"
        | "docs" | "doc" | "convencao" | "convencoes" | "convention" | "conventions"
        | "arquitetura" | "architecture" | "readme" | "decision" | "decisions" | "porque"
        | "why" => Intent::Docs,
        "config" | "configuracao" | "configuracoes" | "settings" | "ambiente" | "env"
        | "variavel" | "variaveis" | "configuration" => Intent::Config,
        "ci" | "pipeline" | "pipelines" | "workflow" | "workflows" => Intent::Ci,
        _ => return None,
    })
}

fn looks_like_path(token: &str) -> bool {
    token.contains('/')
        || token.rsplit_once('.').is_some_and(|(stem, ext)| {
            !stem.is_empty()
                && (1..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

/// Analisa a consulta. Termos com menos de 2 caracteres são ignorados.
pub fn analyze(text: &str) -> QueryAnalysis {
    let mut concepts: Vec<Concept> = Vec::new();
    let mut intents = BTreeSet::new();
    let mut mentioned_paths = Vec::new();
    let mut ignored = Vec::new();

    for raw in text.split(|c: char| {
        c.is_whitespace() || matches!(c, ',' | ';' | '?' | '!' | '"' | '(' | ')' | '`' | '\'')
    }) {
        let raw = raw.trim_matches(|c: char| matches!(c, '.' | ':'));
        if raw.is_empty() {
            continue;
        }
        if looks_like_path(raw) {
            mentioned_paths.push(raw.replace('\\', "/"));
        }
        for piece in raw.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if piece.is_empty() {
                continue;
            }
            let word = normalize(piece);
            if let Some(intent) = intent_of(&word) {
                intents.insert(intent);
            }
            if word.chars().count() < 2 || is_stopword(&word) {
                if !word.is_empty() {
                    ignored.push(word);
                }
                continue;
            }
            if concepts.iter().any(|c| c.term == word) {
                continue;
            }
            let mut variants = BTreeSet::new();
            let direct = expansions(&word);
            let expanded: Vec<&str> = if direct.is_empty() {
                stem_expansions(&word)
            } else {
                direct.to_vec()
            };
            for e in expanded {
                if e != word {
                    variants.insert(e.to_owned());
                }
            }
            // Identificadores compostos: `validateSession` também busca `validate` e `session`.
            let parts = split_identifier(piece);
            if parts.len() > 1 {
                for p in parts.into_iter().filter(|p| p.len() >= 3) {
                    if p != word {
                        variants.insert(p);
                    }
                }
            }
            // Plural simples em português/inglês.
            if word.len() > 4
                && let Some(singular) = word.strip_suffix('s')
            {
                variants.insert(singular.to_owned());
            }
            concepts.push(Concept {
                term: word,
                variants: variants.into_iter().collect(),
            });
        }
    }
    QueryAnalysis {
        concepts,
        intents: intents.into_iter().collect(),
        mentioned_paths,
        ignored,
    }
}

/// Escapa um termo para a sintaxe de consulta do FTS5 (sempre entre aspas).
fn fts_term(term: &str) -> Option<String> {
    let clean: String = term
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if clean.is_empty() {
        return None;
    }
    // Prefixo para termos de 4+ caracteres (`auth` encontra `authentication`).
    Some(if clean.chars().count() >= 4 {
        format!("\"{clean}\"*")
    } else {
        format!("\"{clean}\"")
    })
}

/// Expressão `MATCH` com os termos unidos por OR. `None` se não houver termo utilizável.
pub fn fts_expression(terms: &[String]) -> Option<String> {
    let parts: Vec<String> = terms.iter().filter_map(|t| fts_term(t)).collect();
    (!parts.is_empty()).then(|| parts.join(" OR "))
}

/// Forma que casou no texto, para rótulos honestos: `termo` ou `termo→variante`.
pub fn matched_form(concept: &Concept, normalized_text: &str) -> Option<String> {
    if normalized_text.contains(&concept.term) {
        return Some(concept.term.clone());
    }
    concept
        .variants
        .iter()
        .find(|v| v.len() >= 3 && normalized_text.contains(v.as_str()))
        .map(|v| format!("{}→{v}", concept.term))
}

/// O texto contém alguma variante do conceito? (comparação normalizada, por substring)
pub fn concept_in(concept: &Concept, normalized_text: &str) -> bool {
    normalized_text.contains(&concept.term)
        || concept
            .variants
            .iter()
            .any(|v| v.len() >= 3 && normalized_text.contains(v.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_stopwords_and_expands_portuguese_terms() {
        let a = analyze("Onde fica a autenticação?");
        assert_eq!(a.concepts.len(), 1);
        assert_eq!(a.concepts[0].term, "autenticacao");
        assert!(a.concepts[0].variants.contains(&"auth".to_owned()));
        assert!(a.ignored.contains(&"onde".to_owned()));
    }

    #[test]
    fn detects_intents_and_paths() {
        let a =
            analyze("Quais testes verificam o serviço de pagamentos em app/payments/service.py?");
        assert!(a.has(Intent::Tests));
        assert_eq!(a.mentioned_paths, ["app/payments/service.py"]);
        assert!(a.concepts.iter().any(|c| c.term == "pagamentos"));
        let b = analyze("Onde foi decidida esta convenção?");
        assert!(b.has(Intent::Docs));
    }

    #[test]
    fn identifiers_add_subwords() {
        let a = analyze("validateSession");
        let c = &a.concepts[0];
        assert_eq!(c.term, "validatesession");
        assert!(c.variants.contains(&"validate".to_owned()));
        assert!(c.variants.contains(&"session".to_owned()));
    }

    #[test]
    fn fts_expression_is_quoted_and_safe() {
        let expr = fts_expression(&["auth".into(), "db".into(), "x\" OR 1".into()]).unwrap();
        assert_eq!(expr, "\"auth\"* OR \"db\" OR \"xOR1\"*");
        assert!(fts_expression(&["***".into()]).is_none());
    }

    #[test]
    fn inflected_portuguese_words_use_the_stem() {
        let a = analyze("Onde é validada a sessão?");
        let validada = a.concepts.iter().find(|c| c.term == "validada").unwrap();
        assert!(validada.variants.contains(&"validate".to_owned()));
        let b = analyze("pedidos salvos");
        assert!(
            b.concepts
                .iter()
                .any(|c| c.variants.contains(&"save".to_owned()))
        );
        // Radicais curtos demais não expandem.
        assert!(analyze("uso").concepts[0].variants.is_empty());
    }

    #[test]
    fn only_stopwords_is_empty() {
        assert!(analyze("onde fica o que").is_empty());
    }

    #[test]
    fn normalization_matches_fts_diacritics() {
        assert_eq!(normalize("Configuração ÚNICA"), "configuracao unica");
    }
}
