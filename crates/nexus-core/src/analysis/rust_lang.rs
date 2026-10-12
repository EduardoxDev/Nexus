//! Rust.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, RUST};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces,
};

pub struct Rust;

const QUALIFIERS: &[&str] = &["default", "async", "unsafe", "extern"];

fn visibility(line: &str) -> (&str, bool) {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("pub") {
        if let Some(r) = rest.strip_prefix('(') {
            // pub(crate), pub(super), pub(in path): visível só dentro do crate.
            let close = r.find(')').map_or(r.len(), |i| i + 1);
            return (&r[close..], false);
        }
        if rest.starts_with(char::is_whitespace) {
            return (rest, true);
        }
    }
    (t, false)
}

/// Tipo de um `impl`: `impl<T> Trait for Tipo<T>` → `Tipo`.
fn impl_target(rest: &str) -> Option<String> {
    let mut r = rest.trim_start();
    if r.starts_with('<') {
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in r.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        r = r[end..].trim_start();
    }
    let head = r.split(['{', ' ']).collect::<Vec<_>>();
    let target = match r.find(" for ") {
        Some(i) => r[i + 5..].trim_start(),
        None => head.first().copied().unwrap_or(""),
    };
    let target = target
        .trim_start_matches(['&', '*'])
        .trim_start_matches("mut ")
        .trim_start_matches("dyn ");
    let name = target.rsplit("::").next().unwrap_or(target);
    ident(name).map(str::to_owned)
}

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let (rest, exported) = visibility(line);
    let (rest, _) = strip_words(rest, QUALIFIERS);
    // `extern "C" fn` deixa a ABI entre aspas (mascarada como espaços).
    let rest = rest.trim_start_matches(['"', ' ']);
    let kw = ident(rest)?;
    let after = &rest[kw.len()..];
    let in_container = matches!(scope, Scope::In(_));
    let (kind, opens) = match kw {
        "fn" => (
            if in_container {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            },
            Block::Other,
        ),
        "const" | "static" if in_container => {
            let (q, _) = strip_words(after, QUALIFIERS);
            let f = q.strip_prefix("fn ")?;
            return Some(Decl {
                name: ident(f)?.to_owned(),
                kind: SymbolKind::Method,
                exported,
                opens: Block::Other,
            });
        }
        _ if in_container => return None,
        "struct" | "union" => (SymbolKind::Struct, Block::Other),
        "enum" => (SymbolKind::Enum, Block::Other),
        "trait" => {
            let name = ident(after)?;
            return Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Trait,
                exported,
                opens: Block::Container(name.to_owned()),
            });
        }
        "type" => (SymbolKind::Type, Block::Other),
        "const" | "static" => {
            // `const fn` / `const unsafe fn`: é função.
            let (q, _) = strip_words(after, QUALIFIERS);
            if let Some(f) = q.strip_prefix("fn ") {
                let name = ident(f)?;
                return Some(Decl {
                    name: name.to_owned(),
                    kind: if in_container {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    },
                    exported,
                    opens: Block::Other,
                });
            }
            if in_container {
                return None;
            }
            let a = after.trim_start().strip_prefix("mut ").unwrap_or(after);
            let name = ident(a)?;
            if name == "_" {
                return None;
            }
            return Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Const,
                exported,
                opens: Block::Other,
            });
        }
        // `mod x;` é import de arquivo, não declaração com corpo.
        "mod" if after.trim_end().ends_with(';') => return None,
        "mod" => (SymbolKind::Module, Block::Namespace),
        "macro_rules" => {
            let name = ident(after.trim_start().trim_start_matches('!'))?;
            return Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Macro,
                exported,
                opens: Block::Other,
            });
        }
        "impl" => {
            // `impl` não é um símbolo: só abre um contêiner de métodos.
            let target = impl_target(after)?;
            return Some(Decl {
                name: String::new(),
                kind: SymbolKind::Module,
                exported: false,
                opens: Block::Container(target),
            });
        }
        _ => return None,
    };
    let name = ident(after)?;
    Some(Decl {
        name: name.to_owned(),
        kind,
        exported,
        opens,
    })
}

static USE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?s)(?:^|[\s;{}])(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").ok());
static MOD: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_]\w*)\s*;").ok());

/// Normaliza espaços: colapsa e remove os que cercam `::`, `{`, `}` e `,`.
fn tidy(path: &str) -> String {
    let collapsed = path.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(collapsed.len());
    let chars: Vec<char> = collapsed.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' {
            let prev = i.checked_sub(1).and_then(|j| chars.get(j)).copied();
            let next = chars.get(i + 1).copied();
            let sep = |x: Option<char>| matches!(x, Some(':' | '{' | '}' | ','));
            if sep(prev) || sep(next) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Expande grupos: `crate::a::{b, c::{D, E}}` → `crate::a::b`, `crate::a::c::D`, `crate::a::c::E`.
fn expand_use(path: &str) -> Vec<String> {
    expand(&tidy(path))
}

fn expand(path: &str) -> Vec<String> {
    let Some(open) = path.find('{') else {
        let leaf = path.split(" as ").next().unwrap_or(path);
        return vec![leaf.trim_end_matches("::*").to_owned()];
    };
    let prefix = &path[..open];
    let inner = path[open + 1..]
        .strip_suffix('}')
        .unwrap_or(&path[open + 1..]);
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .flat_map(|p| {
            if p == "self" {
                vec![prefix.trim_end_matches("::").to_owned()]
            } else {
                expand(&format!("{prefix}{p}"))
            }
        })
        .collect()
}

/// Reescreve caminhos `super::`/`self::` usados dentro de `depth` módulos inline para o ponto
/// de vista do arquivo: o que não sai do arquivo vira `self::…` (o próprio arquivo).
fn adjust_for_inline(spec: &str, depth: usize) -> String {
    if depth == 0 {
        return spec.to_owned();
    }
    let segs: Vec<&str> = spec.split("::").collect();
    match segs.first().copied() {
        Some("super") => {
            let supers = segs.iter().take_while(|s| **s == "super").count();
            let rest = &segs[supers..];
            if supers > depth {
                let mut out = vec!["super"; supers - depth];
                out.extend_from_slice(rest);
                out.join("::")
            } else if supers < depth || rest.is_empty() {
                // Continua dentro de um módulo inline: é o próprio arquivo.
                "self".to_owned()
            } else {
                format!("self::{}", rest.join("::"))
            }
        }
        Some("self") => "self".to_owned(),
        _ => spec.to_owned(),
    }
}

impl Analyzer for Rust {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, RUST);
        let mut symbols = walk_braces(&m.bare, classify);
        symbols.retain(|s| !s.name.is_empty());
        let mut imports = Vec::new();
        if let Some(re) = USE.as_ref() {
            for caps in re.captures_iter(&m.bare) {
                let Some(path) = caps.get(1) else { continue };
                let line = lexer::line_of(&m.bare, path.start());
                // Dentro de `mod tests { … }` inline, `super::` aponta para o próprio arquivo.
                let depth = symbols
                    .iter()
                    .filter(|s| s.kind == SymbolKind::Module && s.line < line && line <= s.end_line)
                    .count();
                for spec in expand_use(path.as_str()) {
                    imports.push(Import {
                        specifier: adjust_for_inline(&spec, depth),
                        line,
                        kind: ImportKind::Import,
                    });
                }
            }
        }
        if let Some(re) = MOD.as_ref() {
            for caps in re.captures_iter(&m.bare) {
                let Some(name) = caps.get(1) else { continue };
                imports.push(Import {
                    specifier: name.as_str().to_owned(),
                    line: lexer::line_of(&m.bare, name.start()),
                    kind: ImportKind::Module,
                });
            }
        }
        imports.sort_by_key(|i| (i.line, i.specifier.clone()));
        FileAnalysis { symbols, imports }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_impls_and_traits() {
        let src = r#"//! Docs: fn fake() {}
use std::collections::HashMap;
use crate::parser::{Token, lexer::{self, Lexer}};
pub mod config;
mod inline {
    pub fn helper() {}
}

/// Doc comment struct Fake
pub struct Parser<'a> {
    input: &'a str,
}

pub(crate) enum Kind { A, B }

pub trait Visit {
    fn visit(&self);
    fn default_visit(&self) {}
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let s = "fn not_real() {";
        Self { input }
    }

    async fn parse(&mut self)
        -> Result<(), String>
    {
        fn local() {}
        Ok(())
    }
}

impl Visit for Parser<'_> {
    fn visit(&self) {}
}

pub const MAX: usize = 3;
static mut COUNTER: u32 = 0;
pub type Res<T> = Result<T, String>;
macro_rules! check { () => {} }
"#;
        let a = Rust.analyze(src);
        let got: Vec<(&str, SymbolKind, i64, Option<&str>)> = a
            .symbols
            .iter()
            .map(|s| (s.name.as_str(), s.kind, s.line, s.container.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("inline", SymbolKind::Module, 5, None),
                ("helper", SymbolKind::Function, 6, None),
                ("Parser", SymbolKind::Struct, 10, None),
                ("Kind", SymbolKind::Enum, 14, None),
                ("Visit", SymbolKind::Trait, 16, None),
                ("visit", SymbolKind::Method, 17, Some("Visit")),
                ("default_visit", SymbolKind::Method, 18, Some("Visit")),
                ("new", SymbolKind::Method, 22, Some("Parser")),
                ("parse", SymbolKind::Method, 27, Some("Parser")),
                ("visit", SymbolKind::Method, 36, Some("Parser")),
                ("MAX", SymbolKind::Const, 39, None),
                ("COUNTER", SymbolKind::Const, 40, None),
                ("Res", SymbolKind::Type, 41, None),
                ("check", SymbolKind::Macro, 42, None),
            ]
        );
        assert!(a.symbols[2].exported);
        assert!(!a.symbols[3].exported, "pub(crate) não é público");
        let parse = a.symbols.iter().find(|s| s.name == "parse").unwrap();
        assert_eq!(parse.end_line, 32);
        let imports: Vec<(&str, i64, ImportKind)> = a
            .imports
            .iter()
            .map(|i| (i.specifier.as_str(), i.line, i.kind))
            .collect();
        assert_eq!(
            imports,
            vec![
                ("std::collections::HashMap", 2, ImportKind::Import),
                ("crate::parser::Token", 3, ImportKind::Import),
                ("crate::parser::lexer", 3, ImportKind::Import),
                ("crate::parser::lexer::Lexer", 3, ImportKind::Import),
                ("config", 4, ImportKind::Module),
            ]
        );
    }

    #[test]
    fn use_groups_and_aliases() {
        assert_eq!(
            expand_use("crate::a::{b as c, d::{self, E},\n  f::*}"),
            [
                "crate::a::b",
                "crate::a::d",
                "crate::a::d::E",
                "crate::a::f"
            ]
        );
        assert_eq!(expand_use("super::x::Y as Z"), ["super::x::Y"]);
    }

    #[test]
    fn const_fn_is_a_function() {
        let a = Rust.analyze(
            "pub const fn size() -> usize { 1 }\nimpl X {\n    const fn k() {}\n    const N: u8 = 1;\n}\n",
        );
        let got: Vec<(&str, SymbolKind)> = a
            .symbols
            .iter()
            .map(|s| (s.name.as_str(), s.kind))
            .collect();
        assert_eq!(
            got,
            [("size", SymbolKind::Function), ("k", SymbolKind::Method)]
        );
    }

    #[test]
    fn super_inside_inline_test_modules_points_to_the_file() {
        let src = "pub fn helper() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use super::super::other::X;\n}\n";
        let a = Rust.analyze(src);
        let specs: Vec<&str> = a.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(specs, ["self", "super::other::X"]);
        assert_eq!(adjust_for_inline("super::a::b", 2), "self");
        assert_eq!(adjust_for_inline("crate::x", 1), "crate::x");
    }

    #[test]
    fn impl_targets() {
        assert_eq!(
            impl_target("<T: Clone> From<T> for Wrapper<T> {").as_deref(),
            Some("Wrapper")
        );
        assert_eq!(impl_target(" crate::a::Store {").as_deref(), Some("Store"));
        assert_eq!(
            impl_target(" Display for &mut Thing {").as_deref(),
            Some("Thing")
        );
    }
}
