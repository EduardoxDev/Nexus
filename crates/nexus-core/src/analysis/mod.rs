//! Análise de código por linguagem: declarações (símbolos) e imports, atrás de um trait comum
//! com cobertura declarada (ADR-0012).
//!
//! Os analisadores são **léxicos**: o texto passa por um lexer que mascara comentários e
//! strings (`lexer`), e declarações são reconhecidas linha a linha com uma pilha de blocos
//! (chaves) ou de indentação (Python). Não há AST: a precisão é medida contra respostas
//! conferidas à mão (`tests/analysis.rs`) para decidir se um parser real se justifica.

mod csharp;
pub mod go;
pub mod graph;
mod java;
pub mod lexer;
pub mod map;
mod python;
pub mod query;
pub mod resolve;
mod rust_lang;
mod typescript;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Versão dos analisadores. Mudou ⇒ conteúdos são reanalisados (via `INDEXER_VERSION`).
/// 2: Go, Java e C# (v0.2).
pub const ANALYZER_VERSION: i64 = 2;
/// Máximo de símbolos e imports guardados por arquivo (arquivos gerados enormes).
const MAX_ITEMS: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    Const,
    Variable,
    Module,
    Macro,
}

impl SymbolKind {
    pub const ALL: [Self; 12] = [
        Self::Function,
        Self::Method,
        Self::Class,
        Self::Struct,
        Self::Enum,
        Self::Interface,
        Self::Trait,
        Self::Type,
        Self::Const,
        Self::Variable,
        Self::Module,
        Self::Macro,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Const => "const",
            Self::Variable => "variable",
            Self::Module => "module",
            Self::Macro => "macro",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// Rótulo pt-BR para motivos e saídas.
    pub fn label(self) -> &'static str {
        match self {
            Self::Function => "função",
            Self::Method => "método",
            Self::Class => "classe",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Type => "tipo",
            Self::Const => "constante",
            Self::Variable => "variável",
            Self::Module => "módulo",
            Self::Macro => "macro",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Linha da declaração (1-based).
    pub line: i64,
    /// Última linha do corpo (igual a `line` para declarações sem bloco).
    pub end_line: i64,
    /// Exportado/público (`export`, `pub`, nome Python sem `_`).
    pub exported: bool,
    /// Classe, `impl` ou trait que contém o método.
    pub container: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    /// `import … from`, `from … import`, `use`.
    Import,
    /// `export … from` (reexportação).
    Reexport,
    /// `require("…")`.
    Require,
    /// `import("…")` dinâmico.
    Dynamic,
    /// `mod x;` (Rust: arquivo de módulo).
    Module,
}

impl ImportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Reexport => "reexport",
            Self::Require => "require",
            Self::Dynamic => "dynamic",
            Self::Module => "module",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "reexport" => Self::Reexport,
            "require" => Self::Require,
            "dynamic" => Self::Dynamic,
            "module" => Self::Module,
            _ => Self::Import,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    /// Como escrito (`./session`, `app.payments.gateway`, `crate::parser::Token`).
    pub specifier: String,
    pub line: i64,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileAnalysis {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
}

/// Um analisador de linguagem.
pub trait Analyzer: Sync {
    fn analyze(&self, text: &str) -> FileAnalysis;
}

/// O que cada analisador cobre — exibido na interface e na documentação.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub language: String,
    pub symbols: Vec<String>,
    pub imports: Vec<String>,
    pub not_covered: Vec<String>,
}

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| (*x).to_owned()).collect()
}

pub fn coverage() -> Vec<Coverage> {
    vec![
        Coverage {
            language: "typescript / tsx / javascript / jsx".into(),
            symbols: s(&[
                "function (inclusive async/generator)",
                "class e métodos",
                "interface, type, enum, namespace",
                "const/let/var de nível superior (arrow functions viram função)",
            ]),
            imports: s(&[
                "import … from",
                "import \"x\"",
                "export … from",
                "require(\"x\")",
                "import(\"x\")",
                "aliases de tsconfig (paths/baseUrl)",
            ]),
            not_covered: s(&[
                "objetos e propriedades",
                "funções aninhadas",
                "imports montados dinamicamente",
                "declarações em literais de regex com aspas",
            ]),
        },
        Coverage {
            language: "python".into(),
            symbols: s(&[
                "def / async def",
                "class e métodos",
                "constantes MAIÚSCULAS de módulo",
            ]),
            imports: s(&[
                "import a.b",
                "from a.b import c",
                "imports relativos (from . import x)",
            ]),
            not_covered: s(&[
                "atributos de instância",
                "importlib/__import__",
                "variáveis minúsculas de módulo",
            ]),
        },
        Coverage {
            language: "go".into(),
            symbols: s(&[
                "func e métodos (contêiner = tipo do receptor)",
                "type struct/interface/alias, também em grupos type ( )",
                "const e var de nível superior, também em grupos",
            ]),
            imports: s(&[
                "import \"x\" e grupos import ( )",
                "apelidos (x, _, .)",
                "pacotes do go.mod → todos os arquivos .go da pasta (sem _test.go)",
            ]),
            not_covered: s(&[
                "campos de struct e métodos de interface",
                "funções anônimas",
                "build tags e arquivos gerados por go:generate",
            ]),
        },
        Coverage {
            language: "java".into(),
            symbols: s(&[
                "package (como módulo)",
                "class, interface, enum, record, @interface (também aninhados)",
                "métodos, construtores e campos public (static final = constante)",
            ]),
            imports: s(&[
                "import a.b.C",
                "import a.b.* (o pacote inteiro)",
                "import static a.b.C.m (depende de a.b.C)",
            ]),
            not_covered: s(&[
                "campos não públicos e constantes de enum",
                "classes anônimas e lambdas",
                "text blocks (\"\"\") com chaves desbalanceadas",
            ]),
        },
        Coverage {
            language: "c#".into(),
            symbols: s(&[
                "namespace (com bloco ou file-scoped) como módulo",
                "class, interface, struct, record, enum (também aninhados)",
                "métodos, construtores, propriedades e campos public (const = constante)",
            ]),
            imports: s(&[
                "using X.Y (arquivos que declaram o namespace)",
                "using static X.Y.Tipo e using A = X.Y.Tipo",
                "global using",
            ]),
            not_covered: s(&[
                "membros não públicos (exceto métodos)",
                "eventos, indexadores e operadores",
                "strings verbatim (@\"\") com chaves desbalanceadas",
            ]),
        },
        Coverage {
            language: "rust".into(),
            symbols: s(&[
                "fn (pub, async, const, unsafe)",
                "struct, enum, trait, type, union, const, static",
                "mod inline, macro_rules!",
                "métodos em impl e trait",
            ]),
            imports: s(&[
                "use (com grupos { })",
                "mod x; (arquivo do módulo)",
                "crate::, self::, super::",
            ]),
            not_covered: s(&[
                "itens gerados por macros",
                "re-exports com glob (use x::*) resolvidos só até o módulo",
            ]),
        },
    ]
}

/// Analisador para a linguagem detectada pelo `classify`, se houver.
pub fn analyzer_for(language: Option<&str>) -> Option<&'static dyn Analyzer> {
    match language? {
        "typescript" | "tsx" | "javascript" | "jsx" => Some(&typescript::TypeScript),
        "python" => Some(&python::Python),
        "rust" => Some(&rust_lang::Rust),
        "go" => Some(&go::Go),
        "java" => Some(&java::Java),
        "csharp" => Some(&csharp::CSharp),
        _ => None,
    }
}

pub fn analyze(language: Option<&str>, text: &str) -> Option<FileAnalysis> {
    let mut a = analyzer_for(language)?.analyze(text);
    a.symbols.truncate(MAX_ITEMS);
    a.imports.truncate(MAX_ITEMS);
    Some(a)
}

/// Nome normalizado para buscas (minúsculas, sem `_`): `validate_session` ≈ `validateSession`.
pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '$')
        .flat_map(char::to_lowercase)
        .collect()
}

// ---- Percorredor de blocos (linguagens com chaves) ---------------------------------------

/// Tipo do bloco aberto por uma declaração.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    /// Contém métodos (classe, impl, trait); o nome é o contêiner.
    Container(String),
    /// Agrupa declarações de nível superior (namespace, mod inline).
    Namespace,
    /// Qualquer outro (corpo de função, objeto, `if`…): declarações dentro são ignoradas.
    Other,
}

/// Declaração reconhecida numa linha.
pub(crate) struct Decl {
    pub name: String,
    pub kind: SymbolKind,
    pub exported: bool,
    /// Bloco que a declaração abre, se tiver corpo.
    pub opens: Block,
}

/// Onde a linha está.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scope<'a> {
    Top,
    In(&'a str),
}

/// Percorre linhas de `bare` (comentários e strings já mascarados) com uma pilha de blocos.
/// `classify` reconhece declarações da linguagem conforme o escopo.
pub(crate) fn walk_braces(
    bare: &str,
    classify: impl Fn(&str, Scope<'_>) -> Option<Decl>,
) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = Vec::new();
    // (bloco, índice do símbolo que o abriu)
    let mut stack: Vec<(Block, Option<usize>)> = Vec::new();
    // Declaração cuja chave ainda não apareceu (assinatura em várias linhas).
    let mut pending: Option<(Block, usize)> = None;

    for (i, line) in bare.lines().enumerate() {
        let line_no = i as i64 + 1;
        let scope = match stack.last() {
            None => Some(Scope::Top),
            Some((Block::Namespace, _)) => Some(Scope::Top),
            Some((Block::Container(name), _)) => Some(Scope::In(name.as_str())),
            Some((Block::Other, _)) => None,
        };
        let decl = scope.and_then(|sc| classify(line.trim_start(), sc));
        if let Some(d) = decl {
            let container = match stack.last() {
                Some((Block::Container(n), _)) => Some(n.clone()),
                _ => None,
            };
            symbols.push(Symbol {
                name: d.name,
                kind: d.kind,
                line: line_no,
                end_line: line_no,
                exported: d.exported,
                container,
            });
            pending = Some((d.opens, symbols.len() - 1));
        }
        for ch in line.chars() {
            match ch {
                '{' => match pending.take() {
                    Some((block, idx)) => stack.push((block, Some(idx))),
                    None => stack.push((Block::Other, None)),
                },
                '}' => {
                    if let Some((_, Some(idx))) = stack.pop()
                        && let Some(sym) = symbols.get_mut(idx)
                    {
                        sym.end_line = line_no;
                    }
                }
                ';' => {
                    // Declaração sem corpo (`fn x();`, `type X = Y;`).
                    pending = None;
                }
                _ => {}
            }
        }
        // Valores (`const x = 1` sem `;`) não "esperam" a próxima chave, a menos que a linha
        // continue (`=`, `(`, `,`, `=>`…).
        if let Some((_, idx)) = &pending
            && symbols.get(*idx).is_some_and(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Const | SymbolKind::Variable | SymbolKind::Type
                )
            })
            && !line
                .trim_end()
                .ends_with(['=', '(', ',', '>', '<', '[', '|', '&', ':'])
        {
            pending = None;
        }
    }
    symbols
}

/// Primeiro identificador do texto.
pub(crate) fn ident(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_' || *c == '$'))
        .map_or(s.len(), |(i, _)| i);
    let id = &s[..end];
    (!id.is_empty() && !id.starts_with(|c: char| c.is_ascii_digit())).then_some(id)
}

/// Remove prefixos (palavras-chave) repetidos do início da linha.
pub(crate) fn strip_words<'a>(mut s: &'a str, words: &[&str]) -> (&'a str, Vec<&'a str>) {
    let mut found = Vec::new();
    loop {
        let t = s.trim_start();
        let Some(w) = words.iter().find(|w| {
            t.starts_with(**w)
                && t[w.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
        }) else {
            return (t, found);
        };
        found.push(&t[..w.len()]);
        s = &t[w.len()..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize_across_conventions() {
        assert_eq!(
            normalize_name("validate_session"),
            normalize_name("validateSession")
        );
        assert_eq!(normalize_name("MAX_RETRIES"), "maxretries");
    }

    #[test]
    fn helpers() {
        assert_eq!(ident("  foo_bar(x)"), Some("foo_bar"));
        assert_eq!(ident("1abc"), None);
        let (rest, found) = strip_words(
            "export default async function x()",
            &["export", "default", "async"],
        );
        assert_eq!(found, ["export", "default", "async"]);
        assert_eq!(rest, "function x()");
        // "exported" não é a palavra "export".
        assert_eq!(strip_words("exported = 1", &["export"]).1.len(), 0);
    }

    #[test]
    fn unknown_languages_are_not_analyzed() {
        assert!(analyze(Some("kotlin"), "fun x() {}").is_none());
        assert!(analyze(None, "x").is_none());
        assert_eq!(coverage().len(), 6);
    }
}
