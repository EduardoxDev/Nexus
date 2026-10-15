//! Ruby.
//!
//! Sem chaves: blocos abrem com `class`, `module`, `def`, `do` e as estruturas de controle
//! no início de uma instrução (`if`, `unless`, `while`, `until`, `case`, `begin`, `for`) e
//! fecham com `end`. Modificadores (`x if y`) não abrem blocos; `def x = 1` (sem `end`)
//! também não. `class`/`module` são contêineres (nome = último trecho de `A::B`); `def`
//! dentro deles é método (`def self.x` também). Constantes (`NOME = …`) no nível superior ou
//! em classes/módulos. Exportado = não `private`/`protected` (linha solta ou `private def`).
//! Imports: `require_relative` (arquivo) e `require` (pastas locais, senão gem).

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, RUBY};
use super::{Analyzer, FileAnalysis, Import, ImportKind, Symbol, SymbolKind, ident};

pub struct Ruby;

/// Bloco aberto.
struct Frame {
    /// Classe ou módulo (contêiner de métodos e constantes).
    container: Option<String>,
    /// Corpo de método, `do`, `if`…: declarações dentro são ignoradas.
    other: bool,
    /// Visibilidade atual dentro do contêiner (`private`/`protected` soltos).
    public: bool,
    /// Símbolo aberto pelo bloco (para marcar a última linha).
    symbol: Option<usize>,
}

const OPENERS: &[&str] = &["if", "unless", "while", "until", "case", "begin", "for"];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '?' || c == '!'
}

/// Palavras da linha com a posição em que começam (strings e comentários já mascarados).
fn words(line: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in line.char_indices() {
        match (is_word_char(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, &line[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &line[s..]));
    }
    out
}

/// A palavra está no início de uma instrução (começo da linha, depois de `;`, `=` ou `(`)?
/// Assim `x = if y` abre bloco e `return if y` não.
fn statement_start(line: &str, at: usize) -> bool {
    let before = line[..at].trim_end();
    before.is_empty() || before.ends_with([';', '=', '(', '|', '&', ','])
}

/// Último trecho de `A::B::C`.
fn last_segment(s: &str) -> Option<&str> {
    let name = s.trim();
    let end = name
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(name.len());
    let full = &name[..end];
    let last = full.rsplit("::").next()?;
    ident(last).filter(|l| l.len() == last.len())
}

/// Operadores que podem ser nomes de método (o mais longo primeiro).
const OPERATORS: &[&str] = &[
    "[]=", "[]", "<=>", "===", "==", "=~", "!=", "!~", "<<", ">>", "<=", ">=", "**", "+@", "-@",
    "+", "-", "*", "/", "%", "<", ">", "!", "~", "&", "|", "^",
];

/// Nome de um método: palavra (`pago?`, `salvar!`), setter colado ao `=` (`valor=`) ou
/// operador (`[]`, `+`, `<=>`). O texto começa logo depois de `def`.
fn def_name(after: &str) -> Option<&str> {
    let a = after.trim_start();
    let a = a.strip_prefix("self.").unwrap_or(a);
    if let Some(op) = OPERATORS.iter().find(|op| a.starts_with(**op)) {
        return Some(op);
    }
    let end = a.find(|c: char| !is_word_char(c)).unwrap_or(a.len());
    if end == 0 {
        return None;
    }
    // `def valor=(v)`: o `=` colado ao nome faz parte dele (`def x = 1` tem espaço).
    if a[end..].starts_with('=') && !a[end..].starts_with("==") && !a[end..].starts_with("=~") {
        return Some(&a[..=end]);
    }
    Some(&a[..end])
}

/// `def nome(args) = expr` não tem `end`. O texto começa logo depois de `def`. Um setter
/// (`def total=(v)`) tem `end`.
fn endless_def(after: &str) -> bool {
    let rest = after.trim_start();
    let rest = rest.strip_prefix("self.").unwrap_or(rest);
    let Some(name) = def_name(rest) else {
        return false;
    };
    // Setter (`valor=`): tem `end`.
    if name.ends_with('=') && name.len() > 1 && !OPERATORS.contains(&name) {
        return false;
    }
    let mut tail = &rest[name.len()..];
    let trimmed = tail.trim_start();
    if trimmed.starts_with('(') {
        let mut depth = 0;
        let mut end = trimmed.len();
        for (i, c) in trimmed.char_indices() {
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
        tail = &trimmed[end..];
    }
    let t = tail.trim_start();
    t.starts_with('=') && !t.starts_with("==")
}

fn walk(bare: &str) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    for (i, line) in bare.lines().enumerate() {
        let line_no = i as i64 + 1;
        let toks = words(line);
        let in_other = stack.iter().any(|f| f.other);
        let container = stack.iter().rev().find_map(|f| f.container.clone());
        let trimmed = line.trim();
        // Declarações (só fora de corpos de método/blocos).
        if !in_other {
            let (rest, private_prefix) = match trimmed
                .strip_prefix("private ")
                .or_else(|| trimmed.strip_prefix("protected "))
            {
                Some(r) => (r.trim_start(), true),
                None => (trimmed, false),
            };
            // `private` / `protected` / `public` soltos mudam a visibilidade do contêiner.
            if matches!(trimmed, "private" | "protected" | "public")
                && let Some(frame) = stack.last_mut()
            {
                frame.public = trimmed == "public";
            }
            let public = !private_prefix && stack.last().is_none_or(|f| f.public);
            if let Some(after) = rest.strip_prefix("def ") {
                if let Some(name) = def_name(after) {
                    symbols.push(Symbol {
                        name: name.to_owned(),
                        kind: if container.is_some() {
                            SymbolKind::Method
                        } else {
                            SymbolKind::Function
                        },
                        line: line_no,
                        end_line: line_no,
                        exported: public,
                        container: container.clone(),
                    });
                }
            } else if let Some(after) = rest
                .strip_prefix("class ")
                .or_else(|| rest.strip_prefix("module "))
                && !after.trim_start().starts_with("<<")
                && let Some(name) = last_segment(after)
            {
                symbols.push(Symbol {
                    name: name.to_owned(),
                    kind: if rest.starts_with("class") {
                        SymbolKind::Class
                    } else {
                        SymbolKind::Module
                    },
                    line: line_no,
                    end_line: line_no,
                    exported: true,
                    container: container.clone(),
                });
            } else if let Some(name) = ident(rest)
                && name.starts_with(|c: char| c.is_ascii_uppercase())
                && rest[name.len()..].trim_start().starts_with('=')
                && !rest[name.len()..].trim_start().starts_with("==")
            {
                symbols.push(Symbol {
                    name: name.to_owned(),
                    kind: SymbolKind::Const,
                    line: line_no,
                    end_line: line_no,
                    exported: true,
                    container: container.clone(),
                });
            }
        }
        // Blocos: aberturas e `end`, na ordem em que aparecem na linha.
        let loop_kw = toks
            .first()
            .is_some_and(|(_, w)| matches!(*w, "while" | "until" | "for"));
        for (k, &(at, w)) in toks.iter().enumerate() {
            // `x.end`, `:end` ou `end_date` não são palavras-chave.
            let prev = line[..at].chars().next_back();
            if matches!(prev, Some('.' | ':' | '@' | '$')) {
                continue;
            }
            match w {
                "end" => {
                    if let Some(frame) = stack.pop()
                        && let Some(idx) = frame.symbol
                        && let Some(sym) = symbols.get_mut(idx)
                    {
                        sym.end_line = line_no;
                    }
                }
                "class" | "module" if statement_start(line, at) => {
                    let after = &line[at + w.len()..];
                    let singleton = after.trim_start().starts_with("<<");
                    let declared = symbols.iter().rposition(|s| {
                        s.line == line_no
                            && matches!(s.kind, SymbolKind::Class | SymbolKind::Module)
                    });
                    let name = if singleton {
                        // `class << self`: métodos continuam no contêiner de fora.
                        container.clone()
                    } else {
                        last_segment(after).map(str::to_owned)
                    };
                    stack.push(Frame {
                        container: name,
                        other: in_other,
                        public: true,
                        symbol: declared,
                    });
                }
                "def" => {
                    if !endless_def(&line[at + w.len()..]) {
                        stack.push(Frame {
                            container: None,
                            other: true,
                            public: true,
                            symbol: symbols.iter().rposition(|s| s.line == line_no),
                        });
                    }
                }
                "do" if !(loop_kw && k > 0) => stack.push(Frame {
                    container: None,
                    other: true,
                    public: true,
                    symbol: None,
                }),
                w if OPENERS.contains(&w) && statement_start(line, at) => stack.push(Frame {
                    container: None,
                    other: true,
                    public: true,
                    symbol: None,
                }),
                _ => {}
            }
        }
    }
    symbols
}

static REQUIRE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?m)^[ \t]*(require_relative|require)[ \t]*\(?[ \t]*(?:'([^'\n]+)'|"([^"\n]+)")"#)
        .ok()
});

impl Analyzer for Ruby {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, RUBY);
        let symbols = walk(&m.bare);
        let mut imports = Vec::new();
        if let Some(re) = REQUIRE.as_ref() {
            for caps in re.captures_iter(&m.code) {
                let Some(path) = caps.get(2).or_else(|| caps.get(3)) else {
                    continue;
                };
                let relative = caps
                    .get(1)
                    .is_some_and(|k| k.as_str() == "require_relative");
                let p = path.as_str();
                let specifier = if relative && !p.starts_with('.') {
                    format!("./{p}")
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
                    "{} {} L{}-{}{}{}",
                    s.name,
                    s.kind.as_str(),
                    s.line,
                    s.end_line,
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
    fn operator_and_setter_methods_and_literals() {
        let src = "class Money\n  def [](i)\n    @v[i]\n  end\n\n  def +(other) = Money.new(v + other.v)\n\n  def <=>(o)\n    v <=> o.v\n  end\n\n  def value=(v)\n    @v = v\n  end\n\n  def fmt\n    x = %q{def fake; end}\n    y = /#/\n  end\n\n  def last; end\nend\n";
        let a = Ruby.analyze(src);
        let got: Vec<String> = a
            .symbols
            .iter()
            .map(|s| format!("{} L{}-{}", s.name, s.line, s.end_line))
            .collect();
        assert_eq!(
            got,
            [
                "Money L1-22",
                "[] L2-4",
                "+ L6-6",
                "<=> L8-10",
                "value= L12-14",
                "fmt L16-19",
                "last L21-21",
            ]
        );
    }

    #[test]
    fn classes_modules_methods_constants_and_requires() {
        let src = r#"require "json"
require_relative "money"
require 'billing/tax'

# class Fake; end
module Billing
  VERSION = "1.0"

  class Invoice < Base
    attr_reader :total

    def initialize(total)
      @total = total
      @label = "def fake; end"
    end

    def self.empty = new(0)

    def paid?
      return false if total > 0
      total.zero?
    end

    def lines
      items.each do |i|
        puts i unless i.nil?
      end
      x = if total then 1 else 2 end
    end

    private

    def round(v); v.round(2); end

    protected def audit
      while ready? do
        tick
      end
    end
  end

  class << self
    def configure
      yield config
    end
  end
end

def helper(x) = x * 2
"#;
        let a = Ruby.analyze(src);
        assert_eq!(
            names(&a),
            [
                "Billing module L6-47 pub",
                "VERSION const L7-7 in Billing pub",
                "Invoice class L9-40 in Billing pub",
                "initialize method L12-15 in Invoice pub",
                "empty method L17-17 in Invoice pub",
                "paid? method L19-22 in Invoice pub",
                "lines method L24-29 in Invoice pub",
                "round method L33-33 in Invoice",
                "audit method L35-39 in Invoice",
                "configure method L43-45 in Billing pub",
                "helper function L49-49 pub",
            ]
        );
        let specs: Vec<_> = a
            .imports
            .iter()
            .map(|i| (i.line, i.specifier.as_str()))
            .collect();
        assert_eq!(specs, [(1, "json"), (2, "./money"), (3, "billing/tax")]);
    }
}
