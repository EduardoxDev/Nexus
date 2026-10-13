//! Java.
//!
//! `package a.b;` vira um símbolo `module` (usado também para resolver imports). Tipos
//! (class/interface/enum/record/@interface) abrem contêineres; dentro deles contam métodos,
//! construtores e campos `public`. Exportado = `public`.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, C_LIKE};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces,
};

pub struct Java;

const MODIFIERS: &[&str] = &[
    "public",
    "protected",
    "private",
    "static",
    "final",
    "abstract",
    "sealed",
    "non-sealed",
    "strictfp",
    "synchronized",
    "native",
    "transient",
    "volatile",
    "default",
];

/// Palavras que começam instruções, nunca declarações de membro.
const NOT_TYPES: &[&str] = &[
    "return", "new", "throw", "if", "else", "for", "while", "do", "switch", "case", "try", "catch",
    "finally", "assert", "break", "continue", "yield", "this", "super",
];

/// Remove anotações do começo da linha (`@Override`, `@Size(max = 3)`).
fn strip_annotations(mut s: &str) -> &str {
    loop {
        let t = s.trim_start();
        let Some(rest) = t.strip_prefix('@') else {
            return t;
        };
        if rest.starts_with("interface") {
            return t;
        }
        let name_len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .unwrap_or(rest.len());
        let mut after = &rest[name_len..];
        if after.trim_start().starts_with('(') {
            let open = after.find('(').unwrap_or(0);
            let mut depth = 0;
            let mut end = after.len();
            for (i, c) in after[open..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = open + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            after = &after[end..];
        }
        s = after;
    }
}

fn type_kind(kw: &str) -> Option<SymbolKind> {
    Some(match kw {
        "class" | "record" => SymbolKind::Class,
        "interface" | "@interface" => SymbolKind::Interface,
        "enum" => SymbolKind::Enum,
        _ => return None,
    })
}

/// `Tipo<A, B>[] nome(` / `nome(` (construtor) / `Tipo nome =|;` (campo).
static MEMBER: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^(?:<[^>]*>\s*)?(?:([\w.$]+(?:\s*<[^;=(){}]*>)?(?:\s*\[\s*\])*)\s+)?([A-Za-z_$][\w$]*)\s*([(=;])")
        .ok()
});

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let line = strip_annotations(line);
    let (rest, mods) = strip_words(line, MODIFIERS);
    let public = mods.contains(&"public");
    // Métodos genéricos começam por `<T>`: não há palavra-chave a ler.
    let kw = if rest.starts_with("@interface") {
        "@interface"
    } else {
        ident(rest).unwrap_or("")
    };
    if let Some(kind) = type_kind(kw) {
        let name = ident(&rest[kw.len()..])?;
        return Some(Decl {
            name: name.to_owned(),
            kind,
            exported: public,
            opens: Block::Container(name.to_owned()),
        });
    }
    match scope {
        Scope::Top => {
            if kw == "package" {
                let name: String = rest[kw.len()..]
                    .trim()
                    .trim_end_matches(';')
                    .split_whitespace()
                    .collect();
                return (!name.is_empty()).then_some(Decl {
                    name,
                    kind: SymbolKind::Module,
                    exported: true,
                    opens: Block::Other,
                });
            }
            None
        }
        Scope::In(container) => {
            if NOT_TYPES.contains(&kw) {
                return None;
            }
            let caps = MEMBER.as_ref()?.captures(rest)?;
            let ty = caps.get(1).map(|m| m.as_str());
            let name = caps.get(2)?.as_str();
            let opener = caps.get(3)?.as_str();
            if opener == "(" {
                // Método (com tipo de retorno) ou construtor (nome = contêiner, sem tipo).
                if ty.is_none() && name != container {
                    return None;
                }
                if ty.is_some_and(|t| NOT_TYPES.contains(&t)) {
                    return None;
                }
                return Some(Decl {
                    name: name.to_owned(),
                    kind: SymbolKind::Method,
                    exported: public,
                    opens: Block::Other,
                });
            }
            // Campo: só os públicos (constante quando `static final`).
            if !public || ty.is_none() {
                return None;
            }
            let constant = mods.contains(&"static") && mods.contains(&"final");
            Some(Decl {
                name: name.to_owned(),
                kind: if constant {
                    SymbolKind::Const
                } else {
                    SymbolKind::Variable
                },
                exported: true,
                opens: Block::Other,
            })
        }
    }
}

static IMPORT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*import\s+(?:static\s+)?([A-Za-z_$][\w$]*(?:\s*\.\s*(?:[A-Za-z_$][\w$]*|\*))*)\s*;")
        .ok()
});

impl Analyzer for Java {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, C_LIKE);
        let symbols = walk_braces(&m.bare, classify);
        let mut imports = Vec::new();
        if let Some(re) = IMPORT.as_ref() {
            for caps in re.captures_iter(&m.bare) {
                let Some(path) = caps.get(1) else { continue };
                let static_import = caps.get(0).is_some_and(|c| c.as_str().contains("static"));
                let spec: String = path.as_str().split_whitespace().collect();
                imports.push(Import {
                    // `import static a.b.C.metodo;` depende da classe `a.b.C`.
                    specifier: if static_import {
                        spec.rsplit_once('.')
                            .map_or(spec.clone(), |(c, _)| c.to_owned())
                    } else {
                        spec
                    },
                    line: lexer::line_of(&m.bare, path.start()),
                    kind: ImportKind::Import,
                });
            }
        }
        FileAnalysis { symbols, imports }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(a: &FileAnalysis) -> Vec<String> {
        a.symbols
            .iter()
            .map(|s| {
                format!(
                    "{} {} L{}{}{}",
                    s.name,
                    s.kind.as_str(),
                    s.line,
                    s.container
                        .as_ref()
                        .map(|c| format!(" in {c}"))
                        .unwrap_or_default(),
                    if s.exported { " pub" } else { "" }
                )
            })
            .collect()
    }

    #[test]
    fn package_types_members_and_imports() {
        let src = r#"package com.acme.billing;

import java.util.List;
import java.util.Map.*;
import static com.acme.util.Strings.isBlank;
import com.acme.store.Repository;

/** Doc: class Fake {} */
@Service
public final class InvoiceService implements Billing {
    public static final int MAX_LINES = 50;
    private final Repository repo;
    public String currency = "BRL";

    public InvoiceService(Repository repo) {
        this.repo = repo;
    }

    @Override
    public List<Invoice> pending(int limit) {
        String s = "public void fake() {";
        return repo.find(limit);
    }

    private static <T> Map<String, T> index(List<T> items) {
        return null;
    }

    enum Status { OPEN, PAID }

    record Line(String sku, int qty) {}
}

interface Billing {
    List<Invoice> pending(int limit);
}
"#;
        let a = Java.analyze(src);
        assert_eq!(
            names(&a),
            [
                "com.acme.billing module L1 pub",
                "InvoiceService class L10 pub",
                "MAX_LINES const L11 in InvoiceService pub",
                "currency variable L13 in InvoiceService pub",
                "InvoiceService method L15 in InvoiceService pub",
                "pending method L20 in InvoiceService pub",
                "index method L25 in InvoiceService",
                "Status enum L29 in InvoiceService",
                "Line class L31 in InvoiceService",
                "Billing interface L34",
                "pending method L35 in Billing",
            ]
        );
        let specs: Vec<_> = a
            .imports
            .iter()
            .map(|i| (i.line, i.specifier.as_str()))
            .collect();
        assert_eq!(
            specs,
            [
                (3, "java.util.List"),
                (4, "java.util.Map.*"),
                (5, "com.acme.util.Strings"),
                (6, "com.acme.store.Repository"),
            ]
        );
    }
}
