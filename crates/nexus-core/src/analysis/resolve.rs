//! Resolução de imports para arquivos do projeto.
//!
//! - TS/JS: caminhos relativos (com extensões e `index.*`), `.js` → `.ts` (ESM), aliases de
//!   `tsconfig.json` (`compilerOptions.paths` e `baseUrl`, o tsconfig mais próximo).
//! - Python: imports relativos por pacote; absolutos pela raiz ou por sufixo único.
//! - Rust: `mod x;`, `crate::`, `self::`, `super::` e o nome do próprio crate (Cargo.toml),
//!   pelo maior prefixo do caminho que corresponde a um arquivo de módulo.
//!
//! O que não resolve para um arquivo vira dependência **externa** (pacote/crate) ou fica
//! **não resolvido** (caminho relativo que não existe) — nunca é inventado.

use std::collections::{HashMap, HashSet};

use super::{Import, ImportKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    File(String),
    External(String),
    Unresolved,
}

/// Aliases de um tsconfig.
#[derive(Debug, Clone, Default)]
pub struct TsConfig {
    /// Pasta do tsconfig (relativa à raiz, `""` = raiz).
    pub dir: String,
    pub base_url: Option<String>,
    /// (padrão, destinos), ex.: ("@/*", ["./src/*"]).
    pub paths: Vec<(String, Vec<String>)>,
}

/// Remove comentários `//` e `/* */` (tsconfig aceita) e vírgulas finais antes de `}`/`]`.
fn strip_jsonc(text: &str) -> String {
    let m = super::lexer::mask(text, super::lexer::C_LIKE);
    let mut out = String::with_capacity(m.code.len());
    let chars: Vec<char> = m.code.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == ',' {
            let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

impl TsConfig {
    pub fn parse(dir: &str, text: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(&strip_jsonc(text)).ok()?;
        let opts = v.get("compilerOptions")?;
        let base_url = opts
            .get("baseUrl")
            .and_then(|b| b.as_str())
            .map(str::to_owned);
        let paths = opts
            .get("paths")
            .and_then(|p| p.as_object())
            .map(|p| {
                p.iter()
                    .map(|(k, v)| {
                        let targets = v
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (k.clone(), targets)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Self {
            dir: dir.to_owned(),
            base_url,
            paths,
        })
    }
}

/// Contexto do projeto para resolver imports.
pub struct Resolver<'a> {
    files: &'a HashSet<String>,
    tsconfigs: Vec<TsConfig>,
    /// Nome do crate (com `_`) → pasta do Cargo.toml.
    crates: HashMap<String, String>,
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

fn join(dir: &str, rel: &str) -> Option<String> {
    crate::freshness::doc_refs::join_rel(dir, rel)
}

fn is_within(path: &str, dir: &str) -> bool {
    dir.is_empty() || path == dir || path.starts_with(&format!("{dir}/"))
}

const TS_EXTENSIONS: &[&str] = &[
    ".ts", ".tsx", ".d.ts", ".js", ".jsx", ".mjs", ".cjs", ".json",
];

impl<'a> Resolver<'a> {
    pub fn new(
        files: &'a HashSet<String>,
        tsconfigs: Vec<TsConfig>,
        crates: HashMap<String, String>,
    ) -> Self {
        let mut tsconfigs = tsconfigs;
        // O mais profundo primeiro: o tsconfig mais próximo vence.
        tsconfigs.sort_by_key(|c| std::cmp::Reverse(c.dir.len()));
        Self {
            files,
            tsconfigs,
            crates,
        }
    }

    fn exists(&self, p: &str) -> bool {
        self.files.contains(p)
    }

    pub fn resolve(&self, from: &str, language: &str, import: &Import) -> Target {
        match language {
            "typescript" | "tsx" | "javascript" | "jsx" => self.resolve_ts(from, &import.specifier),
            "python" => self.resolve_python(from, &import.specifier),
            "rust" => self.resolve_rust(from, import),
            _ => Target::Unresolved,
        }
    }

    // ---- TS/JS -------------------------------------------------------------------------

    fn ts_candidates(&self, base: &str) -> Option<String> {
        if self.exists(base) {
            return Some(base.to_owned());
        }
        // ESM com TypeScript: `./x.js` aponta para `./x.ts`.
        for (js, ts) in [
            (".js", ".ts"),
            (".js", ".tsx"),
            (".jsx", ".tsx"),
            (".mjs", ".mts"),
        ] {
            if let Some(stem) = base.strip_suffix(js) {
                let p = format!("{stem}{ts}");
                if self.exists(&p) {
                    return Some(p);
                }
            }
        }
        for ext in TS_EXTENSIONS {
            let p = format!("{base}{ext}");
            if self.exists(&p) {
                return Some(p);
            }
        }
        for ext in TS_EXTENSIONS {
            let p = format!("{base}/index{ext}");
            if self.exists(&p) {
                return Some(p);
            }
        }
        None
    }

    fn resolve_ts(&self, from: &str, spec: &str) -> Target {
        if spec.starts_with("./") || spec.starts_with("../") || spec == "." || spec == ".." {
            return join(dir_of(from), spec)
                .and_then(|base| self.ts_candidates(&base))
                .map_or(Target::Unresolved, Target::File);
        }
        for cfg in self.tsconfigs.iter().filter(|c| is_within(from, &c.dir)) {
            let base_dir = match &cfg.base_url {
                Some(b) => join(&cfg.dir, b).unwrap_or_else(|| cfg.dir.clone()),
                None => cfg.dir.clone(),
            };
            for (pattern, targets) in &cfg.paths {
                let rest = match pattern.strip_suffix('*') {
                    Some(prefix) => spec.strip_prefix(prefix),
                    None => (spec == pattern).then_some(""),
                };
                let Some(rest) = rest else { continue };
                for t in targets {
                    let target = t.replace('*', rest);
                    if let Some(found) =
                        join(&base_dir, &target).and_then(|b| self.ts_candidates(&b))
                    {
                        return Target::File(found);
                    }
                }
            }
            if cfg.base_url.is_some()
                && let Some(found) = join(&base_dir, spec).and_then(|b| self.ts_candidates(&b))
            {
                return Target::File(found);
            }
        }
        if spec.starts_with('/') {
            return Target::Unresolved;
        }
        // Pacote: `@escopo/nome` ou `nome` (subcaminhos ignorados); `node:fs` fica inteiro.
        let package = if spec.starts_with('@') {
            spec.splitn(3, '/').take(2).collect::<Vec<_>>().join("/")
        } else {
            spec.split('/').next().unwrap_or(spec).to_owned()
        };
        Target::External(package)
    }

    // ---- Python ------------------------------------------------------------------------

    fn py_module(&self, base: &str) -> Option<String> {
        [
            format!("{base}.py"),
            format!("{base}/__init__.py"),
            format!("{base}.pyi"),
        ]
        .into_iter()
        .find(|p| self.exists(p))
    }

    fn resolve_python(&self, from: &str, spec: &str) -> Target {
        let dots = spec.chars().take_while(|c| *c == '.').count();
        let rest = &spec[dots..];
        let rel: String = rest.replace('.', "/");
        if dots > 0 {
            let mut dir = dir_of(from).to_owned();
            for _ in 1..dots {
                dir = dir_of(&dir).to_owned();
            }
            let base = if rel.is_empty() {
                dir.clone()
            } else if dir.is_empty() {
                rel.clone()
            } else {
                format!("{dir}/{rel}")
            };
            if rel.is_empty() {
                let init = if dir.is_empty() {
                    "__init__.py".to_owned()
                } else {
                    format!("{dir}/__init__.py")
                };
                return if self.exists(&init) {
                    Target::File(init)
                } else {
                    Target::Unresolved
                };
            }
            return self
                .py_module(&base)
                .map_or(Target::Unresolved, Target::File);
        }
        if rel.is_empty() {
            return Target::Unresolved;
        }
        if let Some(found) = self.py_module(&rel) {
            return Target::File(found);
        }
        // Raízes de código comuns (`src/`) e sufixo único (pacote dentro de uma subpasta).
        if let Some(found) = self.py_module(&format!("src/{rel}")) {
            return Target::File(found);
        }
        let suffixes = [format!("/{rel}.py"), format!("/{rel}/__init__.py")];
        let matches: Vec<&String> = self
            .files
            .iter()
            .filter(|f| suffixes.iter().any(|s| f.ends_with(s.as_str())))
            .collect();
        match matches.as_slice() {
            [one] => Target::File((*one).clone()),
            _ => Target::External(rest.split('.').next().unwrap_or(rest).to_owned()),
        }
    }

    // ---- Rust --------------------------------------------------------------------------

    /// Pasta-raiz do crate (a que contém lib.rs ou main.rs) acima de `from`.
    fn crate_root(&self, from: &str) -> Option<String> {
        let mut dir = dir_of(from).to_owned();
        loop {
            for f in ["lib.rs", "main.rs"] {
                let p = if dir.is_empty() {
                    f.to_owned()
                } else {
                    format!("{dir}/{f}")
                };
                if self.exists(&p) {
                    return Some(dir);
                }
            }
            // Testes de integração (`tests/x.rs`) e exemplos usam o `src/` do pacote vizinho.
            let sibling = if dir.is_empty() {
                "src".to_owned()
            } else {
                format!("{dir}/src")
            };
            if self.exists(&format!("{sibling}/lib.rs"))
                || self.exists(&format!("{sibling}/main.rs"))
            {
                return Some(sibling);
            }
            if dir.is_empty() {
                return None;
            }
            dir = dir_of(&dir).to_owned();
        }
    }

    /// Caminho de módulo de um arquivo relativo à raiz do crate: `src/a/b.rs` → `[a, b]`.
    fn module_path(root: &str, file: &str) -> Vec<String> {
        let rel = if root.is_empty() {
            file
        } else {
            file.strip_prefix(&format!("{root}/")).unwrap_or(file)
        };
        let mut parts: Vec<String> = rel.split('/').map(str::to_owned).collect();
        if let Some(last) = parts.pop() {
            let stem = last.trim_end_matches(".rs");
            if !matches!(stem, "mod" | "lib" | "main") {
                parts.push(stem.to_owned());
            }
        }
        parts
    }

    fn module_file(&self, root: &str, segments: &[String]) -> Option<String> {
        let prefix = if root.is_empty() {
            String::new()
        } else {
            format!("{root}/")
        };
        if segments.is_empty() {
            return ["lib.rs", "main.rs"]
                .iter()
                .map(|f| format!("{prefix}{f}"))
                .find(|p| self.exists(p));
        }
        let base = format!("{prefix}{}", segments.join("/"));
        [format!("{base}.rs"), format!("{base}/mod.rs")]
            .into_iter()
            .find(|p| self.exists(p))
    }

    fn resolve_rust(&self, from: &str, import: &Import) -> Target {
        let Some(root) = self.crate_root(from) else {
            // Sem raiz de crate conhecida: ainda dá para classificar dependências externas.
            let first = import.specifier.split("::").next().unwrap_or("");
            return match first {
                "crate" | "self" | "super" | "" => Target::Unresolved,
                _ if import.kind == ImportKind::Module => Target::Unresolved,
                "std" | "core" | "alloc" => Target::External("std".into()),
                name => Target::External(name.to_owned()),
            };
        };
        let here = Self::module_path(&root, from);
        if import.kind == ImportKind::Module {
            // `mod x;` declarado em `from`: filho do módulo atual.
            let mut segs = here.clone();
            segs.push(import.specifier.clone());
            return self
                .module_file(&root, &segs)
                .map_or(Target::Unresolved, Target::File);
        }
        let mut segs: Vec<String> = import.specifier.split("::").map(str::to_owned).collect();
        let first = segs.first().cloned().unwrap_or_default();
        let mut base: Vec<String> = match first.as_str() {
            "crate" => {
                segs.remove(0);
                Vec::new()
            }
            "self" => {
                segs.remove(0);
                here.clone()
            }
            "super" => {
                let mut b = here.clone();
                while segs.first().is_some_and(|s| s == "super") {
                    segs.remove(0);
                    b.pop();
                }
                b
            }
            name if self.crates.contains_key(name) => {
                segs.remove(0);
                let dir = &self.crates[name];
                let src = if dir.is_empty() {
                    "src".to_owned()
                } else {
                    format!("{dir}/src")
                };
                return self.longest(&src, Vec::new(), &segs);
            }
            "std" | "core" | "alloc" => return Target::External("std".into()),
            // Caminho relativo ao módulo atual (`use config::Config` em lib.rs, com `mod config;`).
            name if self
                .module_file(&root, &[here.clone(), vec![name.to_owned()]].concat())
                .is_some() =>
            {
                here.clone()
            }
            name => return Target::External(name.to_owned()),
        };
        self.longest(&root, std::mem::take(&mut base), &segs)
    }

    /// Maior prefixo de `base + rest` que corresponde a um arquivo de módulo.
    fn longest(&self, root: &str, base: Vec<String>, rest: &[String]) -> Target {
        for k in (0..=rest.len()).rev() {
            let mut segs = base.clone();
            segs.extend(rest[..k].iter().cloned());
            if let Some(f) = self.module_file(root, &segs) {
                return Target::File(f);
            }
        }
        Target::Unresolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn imp(spec: &str) -> Import {
        Import {
            specifier: spec.into(),
            line: 1,
            kind: ImportKind::Import,
        }
    }

    #[test]
    fn typescript_relative_index_alias_and_packages() {
        let f = files(&[
            "src/auth/session.ts",
            "src/utils/index.ts",
            "src/web/App.tsx",
            "src/lib/esm.ts",
        ]);
        let cfg = TsConfig::parse(
            "",
            r#"{ // comentário
              "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["./src/*"], }, }
            }"#,
        )
        .unwrap();
        let r = Resolver::new(&f, vec![cfg], HashMap::new());
        let t = |from: &str, spec: &str| r.resolve(from, "typescript", &imp(spec));
        assert_eq!(
            t("src/web/App.tsx", "../auth/session"),
            Target::File("src/auth/session.ts".into())
        );
        assert_eq!(
            t("src/web/App.tsx", "../utils"),
            Target::File("src/utils/index.ts".into())
        );
        assert_eq!(
            t("src/web/App.tsx", "@/auth/session"),
            Target::File("src/auth/session.ts".into())
        );
        assert_eq!(
            t("src/web/App.tsx", "../lib/esm.js"),
            Target::File("src/lib/esm.ts".into())
        );
        assert_eq!(
            t("src/web/App.tsx", "react-dom/client"),
            Target::External("react-dom".into())
        );
        assert_eq!(
            t("src/web/App.tsx", "@tanstack/react-query"),
            Target::External("@tanstack/react-query".into())
        );
        assert_eq!(t("src/web/App.tsx", "./missing"), Target::Unresolved);
        // baseUrl ".": "src/auth/session" também resolve.
        assert_eq!(
            t("src/web/App.tsx", "src/auth/session"),
            Target::File("src/auth/session.ts".into())
        );
    }

    #[test]
    fn python_relative_absolute_and_external() {
        let f = files(&[
            "app/__init__.py",
            "app/payments/__init__.py",
            "app/payments/gateway.py",
            "app/payments/service.py",
            "tests/test_payments.py",
        ]);
        let r = Resolver::new(&f, Vec::new(), HashMap::new());
        let t = |from: &str, spec: &str| r.resolve(from, "python", &imp(spec));
        assert_eq!(
            t("app/payments/service.py", ".gateway"),
            Target::File("app/payments/gateway.py".into())
        );
        assert_eq!(
            t("app/payments/service.py", ".."),
            Target::File("app/__init__.py".into())
        );
        assert_eq!(
            t("tests/test_payments.py", "app.payments.service"),
            Target::File("app/payments/service.py".into())
        );
        assert_eq!(
            t("tests/test_payments.py", "app.payments"),
            Target::File("app/payments/__init__.py".into())
        );
        assert_eq!(
            t("tests/test_payments.py", "requests.adapters"),
            Target::External("requests".into())
        );
        assert_eq!(t("app/payments/service.py", ".nope"), Target::Unresolved);
    }

    #[test]
    fn rust_modules_crate_paths_and_externals() {
        let f = files(&[
            "src/lib.rs",
            "src/parser.rs",
            "src/parser/token.rs",
            "src/config/mod.rs",
            "tests/parser.rs",
        ]);
        let mut crates = HashMap::new();
        crates.insert("rust_lib".to_owned(), String::new());
        let r = Resolver::new(&f, Vec::new(), crates);
        let module = |from: &str, name: &str| {
            r.resolve(
                from,
                "rust",
                &Import {
                    specifier: name.into(),
                    line: 1,
                    kind: ImportKind::Module,
                },
            )
        };
        let t = |from: &str, spec: &str| r.resolve(from, "rust", &imp(spec));
        assert_eq!(
            module("src/lib.rs", "parser"),
            Target::File("src/parser.rs".into())
        );
        assert_eq!(
            module("src/lib.rs", "config"),
            Target::File("src/config/mod.rs".into())
        );
        assert_eq!(
            module("src/parser.rs", "token"),
            Target::File("src/parser/token.rs".into())
        );
        assert_eq!(
            t("src/config/mod.rs", "crate::parser::Token"),
            Target::File("src/parser.rs".into())
        );
        assert_eq!(
            t("src/parser/token.rs", "super::Parser"),
            Target::File("src/parser.rs".into())
        );
        assert_eq!(
            t("src/parser.rs", "self::token::Kind"),
            Target::File("src/parser/token.rs".into())
        );
        assert_eq!(
            t("src/parser.rs", "crate::Error"),
            Target::File("src/lib.rs".into())
        );
        assert_eq!(
            t("tests/parser.rs", "rust_lib::parser::parse"),
            Target::File("src/parser.rs".into())
        );
        assert_eq!(t("src/lib.rs", "std::fmt"), Target::External("std".into()));
        assert_eq!(
            t("src/lib.rs", "serde::Serialize"),
            Target::External("serde".into())
        );
    }
}
