//! C#.
//!
//! `namespace X.Y` (com bloco ou file-scoped `namespace X.Y;`) vira um símbolo `module`, usado
//! também para resolver `using`. Tipos (class/interface/struct/record/enum) abrem contêineres;
//! dentro deles contam métodos, construtores e propriedades/campos `public` (propriedade =
//! `variable`, `const` = constante). Exportado = `public`.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, C_LIKE};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces,
};

pub struct CSharp;

const MODIFIERS: &[&str] = &[
    "public",
    "protected",
    "private",
    "internal",
    "file",
    "static",
    "readonly",
    "sealed",
    "abstract",
    "virtual",
    "override",
    "partial",
    "async",
    "extern",
    "unsafe",
    "new",
    "required",
    "volatile",
    "ref",
];

const NOT_TYPES: &[&str] = &[
    "return", "throw", "if", "else", "for", "foreach", "while", "do", "switch", "case", "try",
    "catch", "finally", "using", "lock", "yield", "await", "var", "this", "base", "get", "set",
    "init", "add", "remove", "where", "operator", "implicit", "explicit", "event", "delegate",
];

/// Remove atributos do começo da linha (`[Fact]`, `[Route("x")]`).
fn strip_attributes(mut s: &str) -> &str {
    loop {
        let t = s.trim_start();
        if !t.starts_with('[') {
            return t;
        }
        let mut depth = 0;
        let mut end = t.len();
        for (i, c) in t.char_indices() {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        s = &t[end..];
    }
}

fn type_kind(kw: &str, after: &str) -> Option<SymbolKind> {
    Some(match kw {
        "class" => SymbolKind::Class,
        "interface" => SymbolKind::Interface,
        "struct" => SymbolKind::Struct,
        "enum" => SymbolKind::Enum,
        // `record`, `record class`, `record struct`.
        "record" if after.trim_start().starts_with("struct") => SymbolKind::Struct,
        "record" => SymbolKind::Class,
        _ => return None,
    })
}

/// `Tipo<A>[]? Nome(` / `Nome(` (construtor) / `Tipo Nome {|=>|=|;` (propriedade/campo).
static MEMBER: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^(?:([\w.]+(?:\s*<[^;=(){}]*>)?\??(?:\s*\[[\s,]*\])*\??)\s+)?([A-Za-z_@][\w]*)\s*(?:<[^;=(){}]*>\s*)?(\(|\{|=>|=|;|$)")
        .ok()
});

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let line = strip_attributes(line);
    let (rest, mods) = strip_words(line, MODIFIERS);
    let public = mods.contains(&"public");
    let kw = ident(rest).unwrap_or("");
    let after_kw = &rest[kw.len()..];
    if let Some(kind) = type_kind(kw, after_kw) {
        let after = after_kw.trim_start();
        let after = after
            .strip_prefix("class")
            .or_else(|| after.strip_prefix("struct"))
            .filter(|_| kw == "record")
            .unwrap_or(after);
        let name = ident(after)?;
        return Some(Decl {
            name: name.to_owned(),
            kind,
            exported: public,
            opens: Block::Container(name.to_owned()),
        });
    }
    if kw == "namespace" {
        let name: String = after_kw
            .trim()
            .trim_end_matches(['{', ';'])
            .split_whitespace()
            .collect();
        return (!name.is_empty()).then_some(Decl {
            name,
            kind: SymbolKind::Module,
            exported: true,
            opens: Block::Namespace,
        });
    }
    let Scope::In(container) = scope else {
        return None;
    };
    if NOT_TYPES.contains(&kw) {
        return None;
    }
    let const_field = kw == "const";
    let rest = if const_field {
        after_kw.trim_start()
    } else {
        rest
    };
    let caps = MEMBER.as_ref()?.captures(rest)?;
    let ty = caps.get(1).map(|m| m.as_str());
    let name = caps.get(2)?.as_str();
    let opener = caps.get(3)?.as_str();
    if ty.is_some_and(|t| NOT_TYPES.contains(&t)) {
        return None;
    }
    if opener == "(" {
        if ty.is_none() && name != container {
            return None;
        }
        return Some(Decl {
            name: name.to_owned(),
            kind: SymbolKind::Method,
            exported: public,
            opens: Block::Other,
        });
    }
    // Propriedades e campos: só os públicos e com tipo.
    if !public || ty.is_none() {
        return None;
    }
    Some(Decl {
        name: name.to_owned(),
        kind: if const_field {
            SymbolKind::Const
        } else {
            SymbolKind::Variable
        },
        exported: true,
        opens: Block::Other,
    })
}

/// `using X.Y;`, `using static X.Y.Z;`, `using A = X.Y;`, `global using X;`.
static USING: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:global\s+)?using\s+(?:static\s+)?(?:[A-Za-z_]\w*\s*=\s*)?([A-Za-z_][\w]*(?:\s*\.\s*[A-Za-z_][\w]*)*)\s*;")
        .ok()
});

impl Analyzer for CSharp {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, C_LIKE);
        let symbols = walk_braces(&m.bare, classify);
        let mut imports = Vec::new();
        if let Some(re) = USING.as_ref() {
            for caps in re.captures_iter(&m.bare) {
                let Some(path) = caps.get(1) else { continue };
                imports.push(Import {
                    specifier: path.as_str().split_whitespace().collect(),
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
    fn namespaces_types_members_and_usings() {
        let src = r#"using System;
using System.Collections.Generic;
using static Acme.Util.Strings;
using Store = Acme.Data.Store;
global using Acme.Common;

namespace Acme.Billing
{
    /// <summary>class Fake {}</summary>
    [Serializable]
    public sealed class InvoiceService : IBilling
    {
        public const int MaxLines = 50;
        private readonly Store _store;
        public string Currency { get; set; } = "BRL";
        public int Count => _items.Count;

        public InvoiceService(Store store)
        {
            _store = store;
        }

        public async Task<List<Invoice>> PendingAsync(int limit)
        {
            using var scope = _store.Begin();
            var s = "public void Fake() {";
            return await _store.Find(limit);
        }

        private static T? Pick<T>(IReadOnlyList<T> items) => items[0];
    }

    public interface IBilling
    {
        Task<List<Invoice>> PendingAsync(int limit);
    }

    public record struct Line(string Sku, int Qty);

    internal enum Status { Open, Paid }
}
"#;
        let a = CSharp.analyze(src);
        assert_eq!(
            names(&a),
            [
                "Acme.Billing module L7 pub",
                "InvoiceService class L11 pub",
                "MaxLines const L13 in InvoiceService pub",
                "Currency variable L15 in InvoiceService pub",
                "Count variable L16 in InvoiceService pub",
                "InvoiceService method L18 in InvoiceService pub",
                "PendingAsync method L23 in InvoiceService pub",
                "Pick method L30 in InvoiceService",
                "IBilling interface L33 pub",
                "PendingAsync method L35 in IBilling",
                "Line struct L38 pub",
                "Status enum L40",
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
                (1, "System"),
                (2, "System.Collections.Generic"),
                (3, "Acme.Util.Strings"),
                (4, "Acme.Data.Store"),
                (5, "Acme.Common"),
            ]
        );
    }

    #[test]
    fn file_scoped_namespace() {
        let a = CSharp.analyze(
            "namespace Acme.Api;\n\npublic class Health\n{\n    public bool Ok() => true;\n}\n",
        );
        assert_eq!(
            names(&a),
            [
                "Acme.Api module L1 pub",
                "Health class L3 pub",
                "Ok method L5 in Health pub",
            ]
        );
    }
}
