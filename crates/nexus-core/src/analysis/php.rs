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
        // `namespace { … }`: bloco do namespace global; o conteúdo continua no nível superior.
        "namespace" if scope == Scope::Top && after.trim_start().starts_with('{') => Some(Decl {
            name: String::new(),
            kind: SymbolKind::Module,
            exported: true,
            opens: Block::Namespace,
        }),
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

/// Tira `function`/`const` do começo (com qualquer espaço ou quebra de linha depois).
fn strip_use_kind(s: &str) -> &str {
    let s = s.trim_start();
    for kw in ["function", "const"] {
        if let Some(rest) = s.strip_prefix(kw)
            && rest.starts_with(char::is_whitespace)
        {
            return rest.trim_start();
        }
    }
    s
}

/// Expande `A\B\{C, D as E}` e `A, B as C` em nomes completos.
fn expand_use(body: &str) -> Vec<String> {
    let body = strip_use_kind(body.trim()).trim();
    // O nome é a primeira palavra; `as Apelido` (com qualquer espaço) fica de fora.
    let without_alias = |s: &str| -> Option<String> {
        let name = s.split_whitespace().next()?;
        qualified(name).filter(|n| n.len() == name.trim_start_matches('\\').len())
    };
    if let Some(open) = body.find('{') {
        let prefix = body[..open].trim().trim_start_matches('\\');
        let inner = body[open + 1..].trim_end().trim_end_matches('}');
        return inner
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .filter_map(|p| without_alias(strip_use_kind(p)).map(|n| format!("{prefix}{n}")))
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

/// Divide nas vírgulas de nível superior (fora de `()`, `[]` e `{}`), até o `;`.
fn split_top_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            ';' if depth == 0 => {
                parts.push(&s[start..i]);
                return parts;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Declaradores além do primeiro numa mesma linha: `public $a, $b;` e `const A = 1, B = 2;`.
fn more_declarators(line: &str, kind: SymbolKind) -> Vec<String> {
    let line = strip_attributes(line);
    let (rest, _) = strip_words(line, MODIFIERS);
    let list = match kind {
        SymbolKind::Const => rest.trim_start().strip_prefix("const").unwrap_or(""),
        _ => rest.find('$').map_or("", |d| &rest[d..]),
    };
    split_top_commas(list)
        .into_iter()
        .skip(1)
        .filter_map(|part| {
            let decl = part.split('=').next()?.trim();
            let name = match kind {
                SymbolKind::Const => decl.split_whitespace().last()?,
                _ => decl.strip_prefix('$')?,
            };
            ident(name)
                .filter(|n| n.len() == name.len())
                .map(str::to_owned)
        })
        .collect()
}

/// Parâmetros promovidos a propriedade no construtor (`public function __construct(public
/// string $nome)`): só os públicos.
static PROMOTED: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:public(?:\s+readonly)?|readonly\s+public)\s+(?:[?\w\\|&]+\s+)?\$([A-Za-z_]\w*)",
    )
    .ok()
});

/// Fecha o parêntese que abre em `open` (offset em `text`).
fn closing_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

impl Analyzer for Php {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, PHP);
        let mut symbols = walk_braces(&m.bare, classify);
        let lines: Vec<&str> = m.bare.lines().collect();
        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(m.bare.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let mut extra = Vec::new();
        for s in &symbols {
            let Some(line) = usize::try_from(s.line - 1).ok().and_then(|i| lines.get(i)) else {
                continue;
            };
            if matches!(s.kind, SymbolKind::Const | SymbolKind::Variable) {
                for name in more_declarators(line, s.kind) {
                    extra.push(super::Symbol { name, ..s.clone() });
                }
            }
            if s.name == "__construct"
                && let Some(container) = &s.container
                && let Some(start) = usize::try_from(s.line - 1)
                    .ok()
                    .and_then(|i| line_starts.get(i).copied())
                && let Some(open) = m.bare[start..].find('(').map(|o| start + o)
                && let Some(close) = closing_paren(&m.bare, open)
                && let Some(re) = PROMOTED.as_ref()
            {
                for caps in re.captures_iter(&m.bare[open..close]) {
                    let Some(name) = caps.get(1) else { continue };
                    let line_no = lexer::line_of(&m.bare, open + name.start());
                    extra.push(super::Symbol {
                        name: name.as_str().to_owned(),
                        kind: SymbolKind::Variable,
                        line: line_no,
                        end_line: line_no,
                        exported: true,
                        container: Some(container.clone()),
                    });
                }
            }
        }
        // Parâmetros promovidos em linhas próprias já vieram do percorredor.
        for e in extra {
            if !symbols
                .iter()
                .any(|s| s.name == e.name && s.line == e.line && s.kind == e.kind)
            {
                symbols.push(e);
            }
        }
        symbols.sort_by_key(|s| s.line);
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
                // A palavra-chave tem de estar no código, não dentro de uma string.
                let Some(whole) = caps.get(0) else { continue };
                let keyword_len = whole
                    .as_str()
                    .find(|c: char| !c.is_alphanumeric() && c != '_');
                let in_code = keyword_len.is_some_and(|n| {
                    m.bare.get(whole.start()..whole.start() + n) == Some(&whole.as_str()[..n])
                });
                if !in_code {
                    continue;
                }
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
    fn promoted_params_multiple_declarators_global_namespace_and_tricky_imports() {
        let src = r#"<?php
namespace {
    use	function App\Support\money;
    use const
        App\Support\RATE;

    class Order
    {
        public $a, $b = [1, 2], $c;
        public const X = 1, Y = 2;

        public function __construct(
            public readonly int $id,
            private string $secret,
            public ?string $note = null,
        ) {}
    }

    $help = "require 'nada.php';";
    require 'boot.php';
}
"#;
        let a = Php.analyze(src);
        assert_eq!(
            names(&a),
            [
                "Order class L7 pub",
                "a variable L9 in Order pub",
                "b variable L9 in Order pub",
                "c variable L9 in Order pub",
                "X const L10 in Order pub",
                "Y const L10 in Order pub",
                "__construct method L12 in Order pub",
                "id variable L13 in Order pub",
                "note variable L15 in Order pub",
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
                (3, "App\\Support\\money"),
                (4, "App\\Support\\RATE"),
                (20, "boot.php"),
            ]
        );
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
