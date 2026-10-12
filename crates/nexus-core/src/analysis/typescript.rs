//! TypeScript, TSX, JavaScript e JSX.

use std::sync::LazyLock;

use regex::Regex;

use super::lexer::{self, C_LIKE};
use super::{
    Analyzer, Block, Decl, FileAnalysis, Import, ImportKind, Scope, SymbolKind, ident, strip_words,
    walk_braces,
};

pub struct TypeScript;

const MODIFIERS: &[&str] = &["export", "default", "declare", "abstract", "async"];
const MEMBER_MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "readonly",
    "async",
    "override",
    "abstract",
    "declare",
];
const NOT_METHODS: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "return",
    "catch",
    "constructor",
    "super",
    "new",
    "function",
    "throw",
    "else",
    "await",
    "typeof",
    "delete",
    "void",
];

fn classify(line: &str, scope: Scope<'_>) -> Option<Decl> {
    match scope {
        Scope::Top => top_level(line),
        Scope::In(_) => member(line),
    }
}

fn top_level(line: &str) -> Option<Decl> {
    let (rest, mods) = strip_words(line, MODIFIERS);
    let exported = mods.contains(&"export");
    let (kw, after) = {
        let kw = ident(rest)?;
        (kw, &rest[kw.len()..])
    };
    let (kind, after) = match kw {
        "function" => (
            SymbolKind::Function,
            after.trim_start().trim_start_matches('*'),
        ),
        "class" => (SymbolKind::Class, after),
        "interface" => (SymbolKind::Interface, after),
        "type" => (SymbolKind::Type, after),
        "enum" => (SymbolKind::Enum, after),
        "namespace" | "module" => (SymbolKind::Module, after),
        "const" | "let" | "var" => {
            let name = ident(after)?;
            let value = after
                .split_once('=')
                .map(|(_, v)| v.trim_start())
                .unwrap_or("");
            let is_fn = value.starts_with("async ")
                || value.starts_with("function")
                || value.starts_with('(') && (value.contains("=>") || value.ends_with('('))
                || ARROW_SINGLE.as_ref().is_some_and(|re| re.is_match(value));
            let kind = if is_fn {
                SymbolKind::Function
            } else if kw == "const" {
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
        _ => return None,
    };
    let name = ident(after)?;
    // `type X = …` precisa do `=`; `module "x"` (declaração de ambiente) não é capturado.
    if kind == SymbolKind::Type && !after.contains('=') {
        return None;
    }
    let opens = match kind {
        SymbolKind::Class => Block::Container(name.to_owned()),
        SymbolKind::Module => Block::Namespace,
        _ => Block::Other,
    };
    Some(Decl {
        name: name.to_owned(),
        kind,
        exported,
        opens,
    })
}

static ARROW_SINGLE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_$][\w$]*\s*=>").ok());

fn member(line: &str) -> Option<Decl> {
    let (rest, _) = strip_words(line, MEMBER_MODIFIERS);
    // `get nome()`/`set nome(v)` — mas `get(id)` é um método chamado "get".
    let rest = ["get ", "set "]
        .iter()
        .find_map(|p| rest.strip_prefix(p).filter(|r| ident(r).is_some()))
        .unwrap_or(rest);
    let rest = rest.trim_start_matches('*').trim_start_matches('#');
    let name = ident(rest)?;
    if NOT_METHODS.contains(&name) {
        return None;
    }
    let after = rest[name.len()..].trim_start();
    let after = after.strip_prefix('?').unwrap_or(after).trim_start();
    // `nome(…)`, `nome<T>(…)` ou `nome = (…) =>`.
    let is_method = after.starts_with('(')
        || (after.starts_with('<') && after.contains('('))
        || (after.starts_with('=') && after.contains("=>"));
    is_method.then(|| Decl {
        name: name.to_owned(),
        kind: SymbolKind::Method,
        exported: !name.starts_with('_'),
        opens: Block::Other,
    })
}

static IMPORT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?s)\bimport\s+(?:type\s+)?(?:[^'";]*?\bfrom\s*)?["']([^"'\n]+)["']"#).ok()
});
static REEXPORT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?s)\bexport\s+(?:type\s+)?(?:\*(?:\s+as\s+[\w$]+)?|\{[^}]*\})\s*from\s*["']([^"'\n]+)["']"#).ok()
});
static REQUIRE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"\brequire\s*\(\s*["']([^"'\n]+)["']\s*\)"#).ok());
static DYNAMIC: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"\bimport\s*\(\s*["']([^"'\n]+)["']\s*\)"#).ok());

impl Analyzer for TypeScript {
    fn analyze(&self, text: &str) -> FileAnalysis {
        let m = lexer::mask(text, C_LIKE);
        let symbols = walk_braces(&m.bare, classify);
        let mut imports = Vec::new();
        for (re, kind) in [
            (&IMPORT, ImportKind::Import),
            (&REEXPORT, ImportKind::Reexport),
            (&REQUIRE, ImportKind::Require),
            (&DYNAMIC, ImportKind::Dynamic),
        ] {
            let Some(re) = re.as_ref() else { continue };
            for caps in re.captures_iter(&m.code) {
                let (Some(all), Some(spec)) = (caps.get(0), caps.get(1)) else {
                    continue;
                };
                // A palavra-chave precisa estar fora de strings (em `bare` ela continua lá).
                if m.bare.get(all.start()..all.start() + 6)
                    != m.code.get(all.start()..all.start() + 6)
                {
                    continue;
                }
                imports.push(Import {
                    specifier: spec.as_str().to_owned(),
                    line: lexer::line_of(&m.code, all.start()),
                    kind,
                });
            }
        }
        imports.sort_by_key(|i| (i.line, i.specifier.clone()));
        imports.dedup_by(|a, b| a.line == b.line && a.specifier == b.specifier);
        FileAnalysis { symbols, imports }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Symbol;

    fn names(symbols: &[Symbol]) -> Vec<(String, SymbolKind, i64, Option<String>)> {
        symbols
            .iter()
            .map(|s| (s.name.clone(), s.kind, s.line, s.container.clone()))
            .collect()
    }

    #[test]
    fn declarations_methods_and_scopes() {
        let src = r#"import { a, b } from "./a";
// export function commented() {}
export async function load(id: string): Promise<void> {
  const inner = () => 1;
  function nested() {}
}
export default class Store<T> extends Base {
  private items: T[] = [];
  constructor(x: number) { super(); }
  async get(id: string) {
    if (id) { return null; }
  }
  static create<T>(): Store<T> { return new Store(); }
  handle = (e: Event) => { this.x = e; };
}
export interface Props { name: string }
type Mode = "a" | "b";
export enum Color { Red, Green }
export const MAX = 10;
export const compute = (x: number) =>
  x * 2;
const s = "function fake() {}";
namespace NS {
  export function inside() {}
}
"#;
        let a = TypeScript.analyze(src);
        let n = names(&a.symbols);
        let expected: Vec<(String, SymbolKind, i64, Option<String>)> = vec![
            ("load".into(), SymbolKind::Function, 3, None),
            ("Store".into(), SymbolKind::Class, 7, None),
            ("get".into(), SymbolKind::Method, 10, Some("Store".into())),
            (
                "create".into(),
                SymbolKind::Method,
                13,
                Some("Store".into()),
            ),
            (
                "handle".into(),
                SymbolKind::Method,
                14,
                Some("Store".into()),
            ),
            ("Props".into(), SymbolKind::Interface, 16, None),
            ("Mode".into(), SymbolKind::Type, 17, None),
            ("Color".into(), SymbolKind::Enum, 18, None),
            ("MAX".into(), SymbolKind::Const, 19, None),
            ("compute".into(), SymbolKind::Function, 20, None),
            ("s".into(), SymbolKind::Const, 22, None),
            ("NS".into(), SymbolKind::Module, 23, None),
            ("inside".into(), SymbolKind::Function, 24, None),
        ];
        assert_eq!(n, expected);
        let load = &a.symbols[0];
        assert!(load.exported);
        assert_eq!(load.end_line, 6);
        let store = &a.symbols[1];
        assert_eq!(store.end_line, 15);
        assert!(
            !a.symbols
                .iter()
                .find(|s| s.name == "Mode")
                .is_some_and(|s| s.exported)
        );
    }

    #[test]
    fn imports_of_every_form() {
        let src = r#"import React from "react";
import type { A } from './types';
import {
  x,
  y,
} from "../utils/money";
import "./styles.css";
export { z } from "./z";
export * from "./all";
const fs = require("node:fs");
const lazy = () => import("./lazy");
const msg = "import nope from 'nope'";
// import commented from "commented";
"#;
        let a = TypeScript.analyze(src);
        let got: Vec<(&str, i64, ImportKind)> = a
            .imports
            .iter()
            .map(|i| (i.specifier.as_str(), i.line, i.kind))
            .collect();
        assert_eq!(
            got,
            vec![
                ("react", 1, ImportKind::Import),
                ("./types", 2, ImportKind::Import),
                ("../utils/money", 3, ImportKind::Import),
                ("./styles.css", 7, ImportKind::Import),
                ("./z", 8, ImportKind::Reexport),
                ("./all", 9, ImportKind::Reexport),
                ("node:fs", 10, ImportKind::Require),
                ("./lazy", 11, ImportKind::Dynamic),
            ]
        );
    }
}
