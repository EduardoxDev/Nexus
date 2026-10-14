//! PHP.
//!
//! `namespace A\B;` (ou com bloco) vira um símbolo `module` e resolve imports, junto com o
//! PSR-4 do `composer.json`. Tipos (`class`, `interface`, `trait`, `enum`) abrem contêineres;
//! neles contam métodos, constantes e propriedades `public`. No nível superior: `function` e
//! `const`. Exportado = não `private`/`protected`. Imports: `use` (também em grupo, `function`
//! e `const`) e `require`/`include` com caminho literal (`__DIR__ . '/x.php'`).

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, PHP};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces,
};

pub struct Php;

const MODIFIERS: &[&str] = &[
    "public",
    "protected",
    "private",
    "static",
    "abstract",
    "final",
    "readonly",
    "var",
];

/// Remove atributos do começo da linha (`#[Route('/x')]`).
fn strip_attributes(mut s: &str) -> &str {
    loop {
        let t = s.trim_start();
        let Some(rest) = t.strip_prefix("#[") else {
            return t;
        };
        let mut depth = 1;
        let mut end = rest.len();
        for (i, c) in rest.char_indices() {
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
        s = &rest[end..];
    }
}

/// Nome qualificado PHP (`App\Models\User`, sem a barra inicial).
fn qualified(s: &str) -> Option<String> {
    let s = s.trim_start().trim_start_matches('\\');
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\\'))
        .unwrap_or(s.len());
    let name = s[..end].trim_end_matches('\\');
    (!name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit())).then(|| name.to_owned())
}

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let line = strip_attributes(line);
    let (rest, mods) = strip_words(line, MODIFIERS);
    let hidden = mods.contains(&"private") || mods.contains(&"protected");
    let kw = ident(rest).unwrap_or("");
    let after = &rest[kw.len()..];
    let type_kind = match kw {
        "class" => Some(SymbolKind::Class),
        "interface" => Some(SymbolKind::Interface),
        "trait" => Some(SymbolKind::Trait),
        "enum" => Some(SymbolKind::Enum),
        _ => None,
    };
    if let Some(kind) = type_kind {
        let name = ident(after)?;
        return Some(Decl {
            name: name.to_owned(),
            kind,
            exported: true,
            opens: Block::Container(name.to_owned()),
        });
    }
    match kw {
        "function" => {
            let name = ident(after.trim_start().trim_start_matches('&'))?;
            Some(Decl {
                name: name.to_owned(),
                kind: match scope {
                    Scope::Top => SymbolKind::Function,
                    Scope::In(_) => SymbolKind::Method,
                },
                exported: !hidden,
                opens: Block::Other,
            })
        }
        "const" => {
            // `const X = 1;` ou `const int X = 1;` (PHP 8.3, com tipo).
            let names: Vec<&str> = after.split('=').next()?.split_whitespace().collect();
            let name = ident(names.last()?)?;
            Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Const,
                exported: !hidden,
                opens: Block::Other,
            })
        }
        "namespace" if scope == Scope::Top => {
            let name = qualified(after)?;
            Some(Decl {
                name,
                kind: SymbolKind::Module,
                exported: true,
                opens: Block::Namespace,
            })
        }
        _ => {
            // Propriedade pública: `public ?int $total = 0;` / `public $x;`.
            if !matches!(scope, Scope::In(_)) || !mods.contains(&"public") {
                return None;
            }
            let dollar = rest.find('$')?;
            let before = &rest[..dollar];
            if before.contains(['(', '=']) {
                return None;
            }
            let name = ident(&rest[dollar + 1..])?;
            Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Variable,
                exported: true,
                opens: Block::Other,
            })
        }
    }
}

/// `use` de nível superior (fora de classes e funções; dentro de `namespace X { }` vale).
/// Uma linha `use` dentro de uma classe é o uso de uma trait, não um import.
fn use_statements(bare: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    // true = bloco de namespace.
    let mut stack: Vec<bool> = Vec::new();
    let mut offset = 0;
    for line in bare.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let top = stack.iter().all(|ns| *ns);
        if top
            && let Some(rest) = trimmed.strip_prefix("use")
            && rest.starts_with(char::is_whitespace)
        {
            let start = offset + (line.len() - trimmed.len());
            let end = bare[start..].find(';').map_or(bare.len(), |e| start + e);
            out.push((start, bare[start + 3..end].to_owned()));
        }
        let opens_namespace = trimmed.starts_with("namespace");
        let mut first = true;
        for ch in line.chars() {
            match ch {
                '{' => {
                    stack.push(opens_namespace && first);
                    first = false;
                }
                '}' => {
                    stack.pop();
                }
                _ => {}
            }
        }
        offset += line.len();
    }
    out
}

/// Expande `A\B\{C, D as E}` e `A, B as C` em nomes completos.
fn expand_use(body: &str) -> Vec<String> {
    let body = body.trim();
    let body = body
        .strip_prefix("function ")
        .or_else(|| body.strip_prefix("const "))
        .unwrap_or(body)
        .trim();
    let without_alias = |s: &str| -> Option<String> {
        let name = s.split(" as ").next()?.trim();
        qualified(name).filter(|n| n.len() == name.trim_start_matches('\\').len())
    };
    if let Some(open) = body.find('{') {
        let prefix = body[..open].trim().trim_start_matches('\\');
        let inner = body[open + 1..].trim_end().trim_end_matches('}');
        return inner
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .filter_map(|p| {
                let p = p.trim();
                let p = p
                    .strip_prefix("function ")
                    .or_else(|| p.strip_prefix("const "))
                    .unwrap_or(p);
                without_alias(p).map(|n| format!("{prefix}{n}"))
            })
            .collect();
    }
    body.split(',').filter_map(without_alias).collect()
}

static REQUIRE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r#"\b(?:require|require_once|include|include_once)\b\s*\(?\s*(__DIR__\s*\.\s*)?(?:'([^'\n]+)'|"([^"\n]+)")"#,
    )
    .ok()
});

impl Analyzer for Php {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, PHP);
        let symbols = walk_braces(&m.bare, classify);
        let mut imports = Vec::new();
        for (start, body) in use_statements(&m.bare) {
            let line = lexer::line_of(&m.bare, start);
            for spec in expand_use(&body) {
                imports.push(Import {
                    specifier: spec,
                    line,
                    kind: ImportKind::Import,
                });
            }
        }
        if let Some(re) = REQUIRE.as_ref() {
            for caps in re.captures_iter(&m.code) {
                let Some(path) = caps.get(2).or_else(|| caps.get(3)) else {
                    continue;
                };
                let p = path.as_str();
                // `__DIR__ . '/x.php'` é relativo ao arquivo.
                let specifier = if caps.get(1).is_some() {
                    format!("./{}", p.trim_start_matches('/'))
                } else {
                    p.to_owned()
                };
                imports.push(Import {
                    specifier,
                    line: lexer::line_of(&m.code, path.start()),
                    kind: ImportKind::Require,
                });
            }
        }
        imports.sort_by_key(|i| i.line);
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
    fn namespace_types_members_and_imports() {
        let src = r#"<?php
namespace App\Billing;

use App\Models\{Invoice, Line as InvoiceLine};
use function App\Support\money;
use Illuminate\Support\Str;
require_once __DIR__ . '/helpers.php';

// class Fake {}
const VERSION = '1.0';

#[Attribute]
final class InvoiceService implements Billing
{
    use LogsActivity;
    public const MAX_LINES = 50;
    private array $cache = [];
    public ?string $currency = 'BRL';

    public function __construct(private Repository $repo) {}

    public static function pending(int $limit): array
    {
        $s = "function fake() {";
        return [];
    }

    protected function audit(): void {}
}

interface Billing { }
trait LogsActivity { }
enum Status: string { case Open = 'open'; }

function format_money(int $cents): string
{
    return number_format($cents / 100, 2);
}
"#;
        let a = Php.analyze(src);
        assert_eq!(
            names(&a),
            [
                "App\\Billing module L2 pub",
                "VERSION const L10 pub",
                "InvoiceService class L13 pub",
                "MAX_LINES const L16 in InvoiceService pub",
                "currency variable L18 in InvoiceService pub",
                "__construct method L20 in InvoiceService pub",
                "pending method L22 in InvoiceService pub",
                "audit method L28 in InvoiceService",
                "Billing interface L31 pub",
                "LogsActivity trait L32 pub",
                "Status enum L33 pub",
                "format_money function L35 pub",
            ]
        );
        let specs: Vec<_> = a
            .imports
            .iter()
            .map(|i| (i.line, i.specifier.as_str(), i.kind))
            .collect();
        assert_eq!(
            specs,
            [
                (4, "App\\Models\\Invoice", ImportKind::Import),
                (4, "App\\Models\\Line", ImportKind::Import),
                (5, "App\\Support\\money", ImportKind::Import),
                (6, "Illuminate\\Support\\Str", ImportKind::Import),
                (7, "./helpers.php", ImportKind::Require),
            ]
        );
    }
}
