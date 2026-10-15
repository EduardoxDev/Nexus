//! C e C++.
//!
//! O pré-processador é lido à parte: `#define NOME` vira macro e `#include` vira import (`"x.h"`
//! procura o arquivo; `<x>` é externo, a menos que exista nas pastas `include/` do projeto).
//! As linhas de pré-processador são apagadas antes de contar chaves. Declarações: funções
//! (definições e protótipos, com o tipo de retorno na mesma linha), `struct`/`class`/`union`/
//! `enum`, `namespace` (contém declarações de nível superior), `typedef`/`using` (tipo) e
//! `constexpr`/`const` de nível superior. Métodos dentro de classes e definições fora delas
//! (`void Conta::depositar(…)`) ficam com o contêiner. Exportado = função sem `static`.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, C_LIKE};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, Symbol, SymbolKind, ident,
    strip_words, walk_braces,
};

pub struct CCpp;

const MODIFIERS: &[&str] = &[
    "static",
    "inline",
    "extern",
    "virtual",
    "explicit",
    "friend",
    "constexpr",
    "consteval",
    "constinit",
    "const",
    "volatile",
    "unsigned",
    "signed",
    "register",
    "thread_local",
    "mutable",
    "_Noreturn",
    "noreturn",
    "__inline",
    "__forceinline",
];

/// Nunca são nomes de função (`if (`, `return f(`, `sizeof(`…).
const NOT_NAMES: &[&str] = &[
    "if",
    "while",
    "for",
    "switch",
    "return",
    "sizeof",
    "catch",
    "else",
    "do",
    "case",
    "new",
    "delete",
    "throw",
    "static_assert",
    "decltype",
    "alignof",
    "typeid",
    "alignas",
    "defined",
    "noexcept",
    "requires",
    "co_return",
    "co_await",
    "co_yield",
    "goto",
];

/// Remove `template <…>` do começo da linha.
fn strip_template(s: &str) -> &str {
    let t = s.trim_start();
    let Some(rest) = t.strip_prefix("template") else {
        return t;
    };
    let rest = rest.trim_start();
    if !rest.starts_with('<') {
        return t;
    }
    let mut depth = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return rest[i + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    t
}

/// `tipo [ptr/ref] nome(` — o tipo pode ter `::`, `<…>`, `*`, `&`. Nome pode ser `A::b`,
/// `~A` ou `A::~A`.
static FUNCTION: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"^((?:[A-Za-z_][\w:]*(?:\s*<[^;(){}=]*>)?[\s*&]+)*?)((?:[A-Za-z_]\w*::)*~?[A-Za-z_]\w*)\s*\(",
    )
    .ok()
});

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    let line = strip_template(line);
    // `extern "C" {` (a string já está mascarada): agrupa, sem símbolo.
    if line.starts_with("extern") && line.contains('"') && line.trim_end().ends_with('{') {
        return Some(Decl {
            name: String::new(),
            kind: SymbolKind::Module,
            exported: true,
            opens: Block::Namespace,
        });
    }
    let (rest, mods) = strip_words(line, MODIFIERS);
    let kw = ident(rest).unwrap_or("");
    let after = &rest[kw.len()..];
    match kw {
        "namespace" => {
            let name = after.trim().trim_end_matches('{').trim();
            let name = if name.is_empty() {
                String::new()
            } else {
                ident(name)?.to_owned()
            };
            return Some(Decl {
                name,
                kind: SymbolKind::Module,
                exported: true,
                opens: Block::Namespace,
            });
        }
        "struct" | "class" | "union" | "enum" => {
            // `enum class X`, `struct X : Base {`; `struct X;` (só declaração) não conta.
            let after = after
                .trim_start()
                .strip_prefix("class ")
                .or_else(|| after.trim_start().strip_prefix("struct "))
                .unwrap_or(after);
            let (after, _) = strip_words(after, &["alignas", "final"]);
            let name = ident(after)?;
            let tail = after[name.len()..].trim_start();
            if tail.starts_with(';') || tail.starts_with('*') || tail.starts_with('&') {
                return None;
            }
            // `struct Ponto p = {…};` é uma variável.
            if ident(tail).is_some() {
                return None;
            }
            let kind = match kw {
                "class" => SymbolKind::Class,
                "enum" => SymbolKind::Enum,
                _ => SymbolKind::Struct,
            };
            return Some(Decl {
                name: name.to_owned(),
                kind,
                exported: true,
                opens: Block::Container(name.to_owned()),
            });
        }
        "typedef" => {
            // `typedef struct Nó {` abre a struct; `typedef X Y;` é um tipo.
            let inner = after.trim_start();
            for tag in ["struct", "union", "enum"] {
                if let Some(t) = inner.strip_prefix(tag)
                    && let Some(name) = ident(t)
                    && t[t.find(name).unwrap_or(0) + name.len()..]
                        .trim_start()
                        .starts_with('{')
                {
                    return Some(Decl {
                        name: name.to_owned(),
                        kind: if tag == "enum" {
                            SymbolKind::Enum
                        } else {
                            SymbolKind::Struct
                        },
                        exported: true,
                        opens: Block::Container(name.to_owned()),
                    });
                }
            }
            if inner.contains(['(', '{']) || !inner.trim_end().ends_with(';') {
                return None;
            }
            let body = inner.trim_end().trim_end_matches(';');
            let name = body
                .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()?;
            return ident(name).map(|n| Decl {
                name: n.to_owned(),
                kind: SymbolKind::Type,
                exported: true,
                opens: Block::Other,
            });
        }
        "using" => {
            // `using Nome = Tipo;` (não `using namespace std;`).
            let name = ident(after)?;
            if name == "namespace"
                || !after[after.find(name)? + name.len()..]
                    .trim_start()
                    .starts_with('=')
            {
                return None;
            }
            return Some(Decl {
                name: name.to_owned(),
                kind: SymbolKind::Type,
                exported: true,
                opens: Block::Other,
            });
        }
        "public" | "private" | "protected" => return None,
        _ => {}
    }
    // `const`/`constexpr` de nível superior que não é função: `constexpr int kMax = 100;`,
    // `const char *nome = "x";`.
    if scope == Scope::Top && (mods.contains(&"const") || mods.contains(&"constexpr")) {
        let decl_end = rest.find(['=', ';', '{', '[']).unwrap_or(rest.len());
        let head = &rest[..decl_end];
        // Parênteses ou vírgula = continuação de uma assinatura em várias linhas
        // (`const std::string& nome);`), não uma constante.
        if !head.contains(['(', ')', ',']) && decl_end < rest.len() {
            let name = head
                .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                .find(|w| !w.is_empty())?;
            return ident(name).filter(|n| n.len() == name.len()).map(|n| Decl {
                name: n.to_owned(),
                kind: SymbolKind::Const,
                exported: !mods.contains(&"static"),
                opens: Block::Other,
            });
        }
    }
    let caps = FUNCTION.as_ref()?.captures(rest)?;
    let ty = caps.get(1).map_or("", |m| m.as_str()).trim();
    let name = caps.get(2)?.as_str();
    let simple = name.rsplit("::").next().unwrap_or(name);
    if NOT_NAMES.contains(&simple)
        || ty.split_whitespace().any(|w| NOT_NAMES.contains(&w))
        || matches!(ty, "else" | "return")
    {
        return None;
    }
    // Sem tipo de retorno: só construtor/destrutor (`Conta(…)`, `~Conta()`, `Conta::Conta(…)`).
    if ty.is_empty() {
        let ctor = match scope {
            Scope::In(container) => simple.trim_start_matches('~') == container,
            Scope::Top => {
                name.contains("::") && {
                    let mut parts = name.rsplit("::");
                    let last = parts.next().unwrap_or("").trim_start_matches('~');
                    parts.next() == Some(last)
                }
            }
        };
        if !ctor {
            return None;
        }
    }
    // Chamada com resultado atribuído (`int x = f(1);`) não chega aqui (o `=` quebra o padrão).
    let kind = match scope {
        Scope::In(_) => SymbolKind::Method,
        Scope::Top => SymbolKind::Function,
    };
    Some(Decl {
        name: name.to_owned(),
        kind,
        exported: !mods.contains(&"static") || matches!(scope, Scope::In(_)),
        opens: Block::Other,
    })
}

/// `#define NOME` (macros) e `#include`, e o texto sem as linhas de pré-processador.
static DEFINE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^\s*#\s*define\s+([A-Za-z_]\w*)").ok());
static INCLUDE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?m)^[ \t]*#[ \t]*include[ \t]*([<"])([^>"\n]+)[>"]"#).ok());

/// Apaga diretivas de pré-processador (com continuações `\`) de `bare`, guardando as macros.
fn strip_preprocessor(bare: &str) -> (String, Vec<Symbol>) {
    let mut out = String::with_capacity(bare.len());
    let mut macros = Vec::new();
    let mut continued = false;
    for (i, line) in bare.split_inclusive('\n').enumerate() {
        let directive = continued || line.trim_start().starts_with('#');
        if directive {
            if !continued
                && let Some(caps) = DEFINE.as_ref().and_then(|re| re.captures(line))
                && let Some(name) = caps.get(1)
            {
                macros.push(Symbol {
                    name: name.as_str().to_owned(),
                    kind: SymbolKind::Macro,
                    line: i as i64 + 1,
                    end_line: i as i64 + 1,
                    exported: true,
                    container: None,
                });
            }
            continued = line.trim_end().ends_with('\\');
            out.extend(line.chars().map(|c| if c == '\n' { '\n' } else { ' ' }));
        } else {
            out.push_str(line);
        }
    }
    (out, macros)
}

/// `typedef struct { … } Ponto;`: o nome (`Ponto`) aparece no fechamento. Também
/// `typedef struct No { … } NoAlias;` quando o apelido difere da tag.
fn typedef_aliases(bare: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    // (profundidade em que o typedef abriu, nome da tag)
    let mut open: Vec<(usize, Option<String>)> = Vec::new();
    for (i, line) in bare.lines().enumerate() {
        let trimmed = line.trim_start();
        let typedef_tag = trimmed.strip_prefix("typedef").and_then(|t| {
            let t = t.trim_start();
            ["struct", "union", "enum"]
                .iter()
                .find_map(|k| t.strip_prefix(k))
                .filter(|after| after.contains('{'))
                .map(|after| ident(after).map(str::to_owned))
        });
        for (k, c) in line.char_indices() {
            match c {
                '{' => {
                    if let Some(tag) = &typedef_tag
                        && open.last().is_none_or(|(d, _)| *d != depth)
                        && line[..k].trim_start().starts_with("typedef")
                    {
                        open.push((depth, tag.clone()));
                    }
                    depth += 1;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if open.last().is_some_and(|(d, _)| *d == depth)
                        && let Some((_, tag)) = open.pop()
                    {
                        let rest = &line[k + 1..];
                        let names = rest.split(';').next().unwrap_or("");
                        let alias = names
                            .split(',')
                            .map(str::trim)
                            .find(|n| !n.starts_with('*'))
                            .and_then(ident);
                        if let Some(alias) = alias
                            && tag.as_deref() != Some(alias)
                        {
                            out.push(Symbol {
                                name: alias.to_owned(),
                                kind: SymbolKind::Type,
                                line: i as i64 + 1,
                                end_line: i as i64 + 1,
                                exported: true,
                                container: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

impl Analyzer for CCpp {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, C_LIKE);
        let (bare, macros) = strip_preprocessor(&m.bare);
        let mut symbols = walk_braces(&bare, classify);
        // Tipos do próprio arquivo: `Conta::depositar` é método se `Conta` é uma classe/struct
        // daqui ou tem nome de tipo (maiúscula); `net::conectar` é função do namespace `net`.
        let types: std::collections::HashSet<String> = symbols
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class | SymbolKind::Struct))
            .map(|s| s.name.clone())
            .collect();
        for s in &mut symbols {
            if matches!(s.kind, SymbolKind::Function | SymbolKind::Method)
                && let Some((qualifier, name)) = s.name.rsplit_once("::")
            {
                let owner = qualifier
                    .rsplit("::")
                    .next()
                    .unwrap_or(qualifier)
                    .to_owned();
                let is_type =
                    types.contains(&owner) || owner.starts_with(|c: char| c.is_ascii_uppercase());
                s.name = name.to_owned();
                if is_type {
                    s.container = Some(owner);
                    s.kind = SymbolKind::Method;
                }
            }
        }
        symbols.extend(typedef_aliases(&bare));
        symbols.extend(macros);
        symbols.sort_by_key(|s| s.line);
        let mut imports = Vec::new();
        if let Some(re) = INCLUDE.as_ref() {
            for caps in re.captures_iter(&m.code) {
                let (Some(open), Some(path)) = (caps.get(1), caps.get(2)) else {
                    continue;
                };
                imports.push(Import {
                    // `<x>` fica com os sinais: a resolução sabe que é do sistema.
                    specifier: if open.as_str() == "<" {
                        format!("<{}>", path.as_str())
                    } else {
                        path.as_str().to_owned()
                    },
                    line: lexer::line_of(&m.code, path.start()),
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
    fn c_functions_structs_macros_and_includes() {
        let src = r#"#include <stdio.h>
#include "buffer.h"
#define MAX_LEN 64
#define SQUARE(x) \
    ((x) * (x))

typedef struct Node {
    int value;
    struct Node *next;
} Node;

typedef unsigned long size_type;
struct Point;

static int helper(int a);

/* int fake(void) { */
int buffer_len(const Buffer *b) {
    if (b == NULL) {
        return 0;
    }
    int n = count(b);
    return printf("%d", n);
}

static void reset(Buffer *b)
{
    b->len = 0;
}
"#;
        let a = CCpp.analyze(src);
        assert_eq!(
            names(&a),
            [
                "MAX_LEN macro L3 pub",
                "SQUARE macro L4 pub",
                "Node struct L7 pub",
                "size_type type L12 pub",
                "helper function L15",
                "buffer_len function L18 pub",
                "reset function L26",
            ]
        );
        let specs: Vec<_> = a
            .imports
            .iter()
            .map(|i| (i.line, i.specifier.as_str()))
            .collect();
        assert_eq!(specs, [(1, "<stdio.h>"), (2, "buffer.h")]);
    }

    #[test]
    fn typedef_aliases_constants_and_namespace_functions() {
        let src = r#"typedef struct {
    int x, y;
} Point;

typedef struct Node {
    struct Node *next;
} NodeT, *NodePtr;

static const int kLimit = 10;
const char *app_name = "nexus";
int open_file(const char *path,
              const char *mode);

namespace net {
int connect(int port);
}

int net::connect(int port) { return port; }
"#;
        let a = CCpp.analyze(src);
        assert_eq!(
            names(&a),
            [
                "Point type L3 pub",
                "Node struct L5 pub",
                "NodeT type L7 pub",
                "kLimit const L9",
                "app_name const L10 pub",
                "open_file function L11 pub",
                "net module L14 pub",
                "connect function L15 pub",
                "connect function L18 pub",
            ]
        );
    }

    #[test]
    fn cpp_namespaces_classes_and_out_of_line_methods() {
        let src = r#"#include <vector>
#include "account.hpp"

namespace bank {

class Account : public Base {
public:
    explicit Account(int id);
    ~Account();
    virtual int balance() const;
    template <typename T> void apply(T op);
private:
    std::vector<int> history_;
};

enum class Kind { Checking, Savings };
using Ledger = std::vector<Account>;
constexpr int kMaxAccounts = 100;

Account::Account(int id) : id_(id) {}

int Account::balance() const {
    for (int x : history_) { total += x; }
    return total;
}

}  // namespace bank

extern "C" {
int bank_open(int id);
}
"#;
        let a = CCpp.analyze(src);
        assert_eq!(
            names(&a),
            [
                "bank module L4 pub",
                "Account class L6 pub",
                "Account method L8 in Account pub",
                "~Account method L9 in Account pub",
                "balance method L10 in Account pub",
                "apply method L11 in Account pub",
                "Kind enum L16 pub",
                "Ledger type L17 pub",
                "kMaxAccounts const L18 pub",
                "Account method L20 in Account pub",
                "balance method L22 in Account pub",
                "bank_open function L30 pub",
            ]
        );
    }
}
