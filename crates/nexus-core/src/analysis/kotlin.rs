//! Kotlin.
//!
//! `package a.b` vira um símbolo `module` (resolve imports, como em Java). Tipos (`class`,
//! `interface`, `object`, `enum class`…) abrem contêineres; neles contam `fun` (métodos) e
//! propriedades `val`/`var`. No nível superior: `fun` (inclusive de extensão: `fun String.x()`),
//! `val`/`var` (`const val` = constante) e `typealias`. Exportado = não `private`.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, KOTLIN};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces_with,
};

pub struct Kotlin;

const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "internal",
    "open",
    "final",
    "abstract",
    "sealed",
    "data",
    "inner",
    "annotation",
    "override",
    "lateinit",
    "const",
    "inline",
    "noinline",
    "crossinline",
    "infix",
    "operator",
    "suspend",
    "tailrec",
    "external",
    "expect",
    "actual",
    "value",
    "companion",
    "enum",
];

/// Remove anotações do começo da linha (`@JvmStatic`, `@Inject(x)`, `@file:JvmName("X")`).
fn strip_annotations(mut s: &str) -> &str {
    loop {
        let t = s.trim_start();
        let Some(rest) = t.strip_prefix('@') else {
            return t;
        };
        let name_len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == ':'))
            .unwrap_or(rest.len());
        let mut after = &rest[name_len..];
        if after.starts_with('(') {
            let mut depth = 0;
            let mut end = after.len();
            for (i, c) in after.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
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

/// Nome de uma `fun`: `fun <T> String.nome(` → `nome`.
fn fun_name(rest: &str) -> Option<&str> {
    let mut r = rest.trim_start();
    if r.starts_with('<') {
        let mut depth = 0;
        let mut end = r.len();
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
    // O `(` dos parâmetros fica fora de crases (um nome escapado pode ter parênteses).
    let mut in_ticks = false;
    let open = r.char_indices().find_map(|(i, c)| {
        if c == '`' {
            in_ticks = !in_ticks;
        }
        (c == '(' && !in_ticks).then_some(i)
    })?;
    let head = &r[..open];
    // Receptor de extensão (`String.`, `List<T>.`): o nome é o que vem depois do último ponto
    // fora de `<>`.
    let mut depth = 0;
    let mut cut = 0;
    for (i, c) in head.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            '.' if depth == 0 => cut = i + 1,
            _ => {}
        }
    }
    let tail = head[cut..].trim();
    // Nome escapado com crases (`` fun `when`() ``): o nome é o que está entre elas.
    if let Some(inner) = tail.strip_prefix('`') {
        return inner
            .strip_suffix('`')
            .filter(|n| !n.is_empty() && !n.contains('`'));
    }
    let name = ident(tail)?;
    (name.len() == tail.len()).then_some(name)
}

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let line = strip_annotations(line);
    let (rest, mods) = strip_words(line, MODIFIERS);
    let exported = !mods.contains(&"private");
    let kw = ident(rest).unwrap_or("");
    let after = &rest[kw.len()..];
    let type_decl = |kind: SymbolKind, name: &str| Decl {
        name: name.to_owned(),
        kind,
        exported,
        opens: Block::Container(name.to_owned()),
    };
    match kw {
        "class" | "interface" | "object" => {
            let kind = if mods.contains(&"enum") {
                SymbolKind::Enum
            } else if kw == "interface" {
                SymbolKind::Interface
            } else {
                SymbolKind::Class
            };
            // `companion object { … }` sem nome se chama `Companion`.
            let name = ident(after).unwrap_or(if mods.contains(&"companion") {
                "Companion"
            } else {
                ""
            });
            if name.is_empty() {
                return None;
            }
            return Some(type_decl(kind, name));
        }
        // `fun interface X` (interface funcional).
        "fun" if ident(after) == Some("interface") => {
            let name = ident(after.trim_start().strip_prefix("interface")?)?;
            return Some(type_decl(SymbolKind::Interface, name));
        }
        "fun" => {
            let name = fun_name(after)?;
            return Some(Decl {
                name: name.to_owned(),
                kind: match scope {
                    Scope::Top => SymbolKind::Function,
                    Scope::In(_) => SymbolKind::Method,
                },
                exported,
                opens: Block::Other,
            });
        }
        "val" | "var" => {
            // `val (a, b) = …` (desestruturação) não declara um nome só.
            let name = ident(after)?;
            let kind = if mods.contains(&"const") {
                SymbolKind::Const
            } else {
                SymbolKind::Variable
            };
            return Some(Decl {
                name: name.to_owned(),
                kind,
                exported,
                opens: Block::Other,
            });
        }
        "typealias" if scope == Scope::Top => {
            return Some(Decl {
                name: ident(after)?.to_owned(),
                kind: SymbolKind::Type,
                exported,
                opens: Block::Other,
            });
        }
        "package" if scope == Scope::Top => {
            let name: String = after.split_whitespace().collect();
            let name = name.trim_end_matches(';').to_owned();
            return (!name.is_empty()).then_some(Decl {
                name,
                kind: SymbolKind::Module,
                exported: true,
                opens: Block::Other,
            });
        }
        _ => {}
    }
    None
}

static IMPORT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*import\s+([\p{L}_][\p{L}\p{N}_]*(?:\.(?:[\p{L}_][\p{L}\p{N}_]*|\*))*)")
        .ok()
});

impl Analyzer for Kotlin {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, KOTLIN);
        let symbols = walk_braces_with(&m.bare, classify, true);
        let mut imports = Vec::new();
        if let Some(re) = IMPORT.as_ref() {
            for caps in re.captures_iter(&m.bare) {
                let Some(path) = caps.get(1) else { continue };
                imports.push(Import {
                    specifier: path.as_str().to_owned(),
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
    fn header_lambdas_default_params_backticks_and_raw_strings() {
        let src = r#"package com.acme.ui

import com.acme.ação.Botão

class Panel(val onClick: () -> Unit = {}) {
    fun render() {}
}

fun retry(
    times: Int = 3,
    delay: Long = 100,
) {
    println(times)
}

fun `when`() {}
fun `soma (a, b)`() {}

/* /* aninhado */ fun fake() {} */
val doc = """
fun alsoFake() {}
"""
"#;
        let a = Kotlin.analyze(src);
        let got: Vec<String> = a
            .symbols
            .iter()
            .map(|s| {
                format!(
                    "{} {} L{}-{}{}",
                    s.name,
                    s.kind.as_str(),
                    s.line,
                    s.end_line,
                    s.container
                        .as_ref()
                        .map(|c| format!(" in {c}"))
                        .unwrap_or_default()
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                "com.acme.ui module L1-1",
                "Panel class L5-7",
                "render method L6-6 in Panel",
                "retry function L9-14",
                "when function L16-16",
                "soma (a, b) function L17-17",
                "doc variable L20-20",
            ]
        );
        assert_eq!(a.imports[0].specifier, "com.acme.ação.Botão");
    }

    #[test]
    fn package_types_functions_and_imports() {
        let src = r#"@file:JvmName("Billing")
package com.acme.billing

import com.acme.store.Repository
import kotlinx.coroutines.*
import com.acme.util.slug as toSlug

const val MAX_LINES = 50
private val cache = mutableMapOf<String, Int>()
typealias Lines = List<Line>

/* class Fake {} */
@Service
data class Invoice(val id: String, val total: Long) {
    val label: String get() = "fun fake() {"
    fun isPaid(): Boolean = total == 0L

    companion object {
        fun empty() = Invoice("", 0)
    }
}

sealed interface Event
enum class Status { OPEN, PAID }
object Registry {
    private fun reset() {
        val local = 1
    }
}

fun String.slugify(): String = lowercase()
suspend fun <T> retry(times: Int, block: suspend () -> T): T {
    return block()
}
fun interface Handler { fun handle(e: Event) }
"#;
        let a = Kotlin.analyze(src);
        assert_eq!(
            names(&a),
            [
                "com.acme.billing module L2 pub",
                "MAX_LINES const L8 pub",
                "cache variable L9",
                "Lines type L10 pub",
                "Invoice class L14 pub",
                "label variable L15 in Invoice pub",
                "isPaid method L16 in Invoice pub",
                "Companion class L18 in Invoice pub",
                "empty method L19 in Companion pub",
                "Event interface L23 pub",
                "Status enum L24 pub",
                "Registry class L25 pub",
                "reset method L26 in Registry",
                "slugify function L31 pub",
                "retry function L32 pub",
                // Uma declaração por linha: o `fun handle` da mesma linha não entra.
                "Handler interface L35 pub",
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
                (4, "com.acme.store.Repository"),
                (5, "kotlinx.coroutines.*"),
                (6, "com.acme.util.slug"),
            ]
        );
    }
}
