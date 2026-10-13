//! Go.
//!
//! Exportado = nome com inicial maiúscula (regra da linguagem). Métodos (`func (r *T) X()`)
//! ficam no nível superior do arquivo; o contêiner vem do tipo do receptor.

use std::cell::Cell;
use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, C_LIKE};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, walk_braces,
};

pub struct Go;

fn exported(name: &str) -> bool {
    name.starts_with(|c: char| c.is_uppercase())
}

/// Grupo aberto por `type (`, `const (` ou `var (`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    None,
    Type,
    Const,
    Var,
}

/// Receptor de um método: `(r *Tipo[T])` → `Tipo`.
fn receiver(after_func: &str) -> Option<(String, &str)> {
    let r = after_func.trim_start().strip_prefix('(')?;
    let close = r.find(')')?;
    let inside = &r[..close];
    let ty = inside.split_whitespace().last()?.trim_start_matches('*');
    let ty = ty.split('[').next()?;
    Some((ident(ty)?.to_owned(), &r[close + 1..]))
}

fn type_decl(rest: &str) -> Option<Decl> {
    let name = ident(rest)?;
    let after = rest.trim_start()[name.len()..].trim_start();
    // Parâmetros de tipo: `type Pilha[T any] struct`.
    let after = if after.starts_with('[') {
        after
            .find(']')
            .map_or(after, |i| after[i + 1..].trim_start())
    } else {
        after
    };
    let kind = if after.starts_with("struct") {
        SymbolKind::Struct
    } else if after.starts_with("interface") {
        SymbolKind::Interface
    } else {
        SymbolKind::Type
    };
    Some(Decl {
        name: name.to_owned(),
        kind,
        exported: exported(name),
        opens: Block::Other,
    })
}

fn value_decl(rest: &str, kind: SymbolKind) -> Option<Decl> {
    let name = ident(rest)?;
    if name == "_" {
        return None;
    }
    Some(Decl {
        name: name.to_owned(),
        kind,
        exported: exported(name),
        opens: Block::Other,
    })
}

fn analyze_symbols(bare: &str) -> Vec<super::Symbol> {
    let group = Cell::new(Group::None);
    let mut symbols = walk_braces(bare, |line, scope| {
        if scope != Scope::Top {
            return None;
        }
        let line = line.trim_end();
        match group.get() {
            Group::None => {}
            g => {
                if line.starts_with(')') {
                    group.set(Group::None);
                    return None;
                }
                return match g {
                    Group::Type => type_decl(line),
                    Group::Const => value_decl(line, SymbolKind::Const),
                    Group::Var => value_decl(line, SymbolKind::Variable),
                    Group::None => None,
                };
            }
        }
        let kw = ident(line)?;
        let rest = &line[kw.len()..];
        match kw {
            "func" => {
                if let Some((_, after)) = receiver(rest) {
                    let name = ident(after)?;
                    return Some(Decl {
                        name: name.to_owned(),
                        kind: SymbolKind::Method,
                        exported: exported(name),
                        opens: Block::Other,
                    });
                }
                let name = ident(rest)?;
                Some(Decl {
                    name: name.to_owned(),
                    kind: SymbolKind::Function,
                    exported: exported(name),
                    opens: Block::Other,
                })
            }
            "type" | "const" | "var" => {
                if rest.trim_start().starts_with('(') {
                    group.set(match kw {
                        "type" => Group::Type,
                        "const" => Group::Const,
                        _ => Group::Var,
                    });
                    return None;
                }
                match kw {
                    "type" => type_decl(rest),
                    "const" => value_decl(rest, SymbolKind::Const),
                    _ => value_decl(rest, SymbolKind::Variable),
                }
            }
            _ => None,
        }
    });
    // O contêiner de um método é o tipo do receptor (não um bloco envolvente).
    let lines: Vec<&str> = bare.lines().collect();
    for sym in &mut symbols {
        if sym.kind == SymbolKind::Method
            && let Some(line) = lines.get((sym.line - 1) as usize)
            && let Some(rest) = line.trim_start().strip_prefix("func")
        {
            sym.container = receiver(rest).map(|(ty, _)| ty);
        }
    }
    symbols
}

/// `"caminho"` (ou `` `caminho` ``) com apelido opcional (`x`, `_`, `.`) antes.
static IMPORT_SPEC: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"^\s*(?:[A-Za-z_]\w*\s+|[._]\s+)?["`]([^"`]+)["`]"#).ok());

fn analyze_imports(code: &str) -> Vec<Import> {
    let Some(spec) = IMPORT_SPEC.as_ref() else {
        return Vec::new();
    };
    let mut imports = Vec::new();
    let mut in_group = false;
    for (i, line) in code.lines().enumerate() {
        let t = line.trim();
        let candidate = if in_group {
            if t.starts_with(')') {
                in_group = false;
                continue;
            }
            t
        } else if let Some(rest) = t.strip_prefix("import") {
            let rest = rest.trim_start();
            if let Some(after) = rest.strip_prefix('(') {
                in_group = true;
                after
            } else {
                rest
            }
        } else {
            continue;
        };
        if let Some(caps) = spec.captures(candidate)
            && let Some(path) = caps.get(1)
        {
            imports.push(Import {
                specifier: path.as_str().to_owned(),
                line: i as i64 + 1,
                kind: ImportKind::Import,
            });
        }
    }
    imports
}

impl Analyzer for Go {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, C_LIKE);
        FileAnalysis {
            symbols: analyze_symbols(&m.bare),
            imports: analyze_imports(&m.code),
        }
    }
}

/// `module caminho` do go.mod.
pub fn module_path(go_mod: &str) -> Option<String> {
    go_mod.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("module")?;
        let path = rest.trim().trim_matches('"');
        (!path.is_empty() && rest.starts_with(char::is_whitespace)).then(|| path.to_owned())
    })
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
    fn functions_methods_types_and_groups() {
        let src = r#"// Package auth: func Fake() {}
package auth

import (
	"context"
	stdlog "log"
	_ "embed"
	"github.com/acme/app/internal/store"
)

import "fmt"

const MaxTries = 3

var (
	ErrDenied = fmt.Errorf("negado: func x()")
	cache     map[string]int
)

type (
	Token string
	Pilha[T any] struct {
		itens []T
	}
)

// Service autentica.
type Service struct {
	store *store.Store
}

type Checker interface {
	Check(ctx context.Context) error
}

func New(s *store.Store) *Service {
	inner := func() {}
	_ = inner
	return &Service{store: s}
}

func (s *Service) Login(ctx context.Context, user string) error {
	return nil
}

func (p Pilha[T]) tamanho() int { return len(p.itens) }
"#;
        let a = Go.analyze(src);
        assert_eq!(
            names(&a),
            [
                "MaxTries const L13 pub",
                "ErrDenied variable L16 pub",
                "cache variable L17",
                "Token type L21 pub",
                "Pilha struct L22 pub",
                "Service struct L28 pub",
                "Checker interface L32 pub",
                "New function L36 pub",
                "Login method L42 in Service pub",
                "tamanho method L46 in Pilha",
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
                (5, "context"),
                (6, "log"),
                (7, "embed"),
                (8, "github.com/acme/app/internal/store"),
                (11, "fmt"),
            ]
        );
    }

    #[test]
    fn module_path_from_go_mod() {
        assert_eq!(
            module_path("// c\nmodule github.com/acme/app\n\ngo 1.24\n").as_deref(),
            Some("github.com/acme/app")
        );
        assert_eq!(module_path("go 1.24\n"), None);
    }
}
