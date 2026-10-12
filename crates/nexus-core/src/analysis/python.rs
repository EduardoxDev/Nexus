//! Python: blocos por indentação.

use super::lexer::{self, PYTHON};
use super::{Analyzer, FileAnalysis, Import, ImportKind, Symbol, SymbolKind, ident};

pub struct Python;

fn indent_of(line: &str) -> usize {
    line.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

impl Analyzer for Python {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, PYTHON);
        let lines: Vec<&str> = m.bare.lines().collect();
        let mut symbols: Vec<Symbol> = Vec::new();
        // (indentação da classe, nome)
        let mut classes: Vec<(usize, String)> = Vec::new();
        // (indentação, índice do símbolo) dos blocos abertos, para calcular `end_line`.
        let mut open: Vec<(usize, usize)> = Vec::new();
        // Linhas de continuação (dentro de parênteses) não começam declarações.
        let mut paren_depth: i64 = 0;

        for (i, raw) in lines.iter().enumerate() {
            let line_no = i as i64 + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let indent = indent_of(raw);
            if paren_depth == 0 {
                while open.last().is_some_and(|(ind, _)| indent <= *ind) {
                    open.pop();
                }
                while classes.last().is_some_and(|(ind, _)| indent <= *ind) {
                    classes.pop();
                }
                for (_, idx) in &open {
                    if let Some(s) = symbols.get_mut(*idx) {
                        s.end_line = line_no;
                    }
                }
                let body = trimmed.strip_prefix("async ").unwrap_or(trimmed);
                // Dentro de uma função (não de classe), declarações são locais: ignoradas.
                let in_function = open.iter().any(|(ind, idx)| {
                    symbols.get(*idx).is_some_and(|s| {
                        matches!(s.kind, SymbolKind::Function | SymbolKind::Method) && *ind < indent
                    })
                });
                let container = classes
                    .last()
                    .filter(|(ind, _)| *ind < indent)
                    .map(|(_, n)| n.clone());
                let decl = if let Some(rest) = body.strip_prefix("def ") {
                    ident(rest).map(|n| {
                        (
                            n,
                            if container.is_some() {
                                SymbolKind::Method
                            } else {
                                SymbolKind::Function
                            },
                        )
                    })
                } else if let Some(rest) = body.strip_prefix("class ") {
                    ident(rest).map(|n| (n, SymbolKind::Class))
                } else if indent == 0 {
                    // Constante de módulo: NOME_MAIUSCULO = …
                    ident(body)
                        .filter(|n| {
                            n.len() > 1
                                && n.chars().all(|c| {
                                    c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'
                                })
                                && body[n.len()..].trim_start().starts_with(['=', ':'])
                                && !body[n.len()..].trim_start().starts_with("==")
                        })
                        .map(|n| (n, SymbolKind::Const))
                } else {
                    None
                };
                if let Some((name, kind)) = decl
                    && !in_function
                {
                    symbols.push(Symbol {
                        name: name.to_owned(),
                        kind,
                        line: line_no,
                        end_line: line_no,
                        exported: !name.starts_with('_'),
                        container: if kind == SymbolKind::Method {
                            container
                        } else {
                            None
                        },
                    });
                    if matches!(
                        kind,
                        SymbolKind::Function | SymbolKind::Method | SymbolKind::Class
                    ) {
                        open.push((indent, symbols.len() - 1));
                    }
                    if kind == SymbolKind::Class {
                        classes.push((indent, name.to_owned()));
                    }
                }
            } else {
                for (_, idx) in &open {
                    if let Some(s) = symbols.get_mut(*idx) {
                        s.end_line = line_no;
                    }
                }
            }
            for c in trimmed.chars() {
                match c {
                    '(' | '[' | '{' => paren_depth += 1,
                    ')' | ']' | '}' => paren_depth = (paren_depth - 1).max(0),
                    _ => {}
                }
            }
        }

        let mut imports = Vec::new();
        for (i, line) in m.code.lines().enumerate() {
            let line_no = i as i64 + 1;
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("import ") {
                for part in rest.split(',') {
                    let module = part.split_whitespace().next().unwrap_or("");
                    if !module.is_empty()
                        && module
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
                    {
                        imports.push(Import {
                            specifier: module.to_owned(),
                            line: line_no,
                            kind: ImportKind::Import,
                        });
                    }
                }
            } else if let Some(rest) = t.strip_prefix("from ")
                && let Some((module, _)) = rest.split_once(" import")
            {
                let module = module.trim();
                if !module.is_empty()
                    && module
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
                {
                    // `from . import x` importa o submódulo `x`: registra `.x` quando possível.
                    let names = rest.split_once(" import").map_or("", |(_, n)| n);
                    if module.chars().all(|c| c == '.') {
                        for n in names
                            .trim()
                            .trim_start_matches('(')
                            .split(',')
                            .filter_map(|p| p.split_whitespace().next())
                            .filter(|p| p.chars().all(|c| c.is_alphanumeric() || c == '_'))
                        {
                            imports.push(Import {
                                specifier: format!("{module}{n}"),
                                line: line_no,
                                kind: ImportKind::Import,
                            });
                        }
                    } else {
                        imports.push(Import {
                            specifier: module.to_owned(),
                            line: line_no,
                            kind: ImportKind::Import,
                        });
                    }
                }
            }
        }
        FileAnalysis { symbols, imports }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_methods_functions_and_constants() {
        let src = r#""""Docstring with def fake(): and class Fake:"""
import os, sys as system
from app.payments import gateway
from . import models, utils
from ..core.config import (
    Settings,
)

MAX_RETRIES = 3
DEBUG: bool = False
lower_case = 1

class PaymentService(Base):
    """class Inner: not real"""
    rate = 0.1

    def __init__(self, gw):
        self.gw = gw

    async def charge(self, amount):
        def helper():
            return 1
        return helper()

    class Meta:
        def describe(self):
            pass

def _private(x,
             y):
    return x + y
"#;
        let a = Python.analyze(src);
        let got: Vec<(&str, SymbolKind, i64, Option<&str>)> = a
            .symbols
            .iter()
            .map(|s| (s.name.as_str(), s.kind, s.line, s.container.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("MAX_RETRIES", SymbolKind::Const, 9, None),
                ("DEBUG", SymbolKind::Const, 10, None),
                ("PaymentService", SymbolKind::Class, 13, None),
                ("__init__", SymbolKind::Method, 17, Some("PaymentService")),
                ("charge", SymbolKind::Method, 20, Some("PaymentService")),
                ("Meta", SymbolKind::Class, 25, None),
                ("describe", SymbolKind::Method, 26, Some("Meta")),
                ("_private", SymbolKind::Function, 29, None),
            ]
        );
        let private = a.symbols.last().unwrap();
        assert!(!private.exported);
        assert_eq!(private.end_line, 31);
        assert_eq!(a.symbols[2].end_line, 27);
        let imports: Vec<(&str, i64)> = a
            .imports
            .iter()
            .map(|i| (i.specifier.as_str(), i.line))
            .collect();
        assert_eq!(
            imports,
            vec![
                ("os", 2),
                ("sys", 2),
                ("app.payments", 3),
                (".models", 4),
                (".utils", 4),
                ("..core.config", 5),
            ]
        );
    }
}
