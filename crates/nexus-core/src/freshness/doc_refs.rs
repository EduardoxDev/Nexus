//! Extração de referências citadas em documentos Markdown: caminhos (links relativos e código
//! inline com cara de caminho) e identificadores de código (código inline com cara de símbolo).
//!
//! Heurística conservadora para evitar falsos positivos: blocos de código cercados são ignorados
//! (costumam ser exemplos), URLs, âncoras, globs, variáveis e placeholders também.

use std::sync::LazyLock;

use regex::Regex;

/// Como a referência apareceu no documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefSyntax {
    /// `[texto](caminho)` — o autor quis apontar para um arquivo.
    Link,
    /// `` `caminho` `` — pode ser exemplo ou nome genérico.
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRef {
    /// Caminho como escrito (sem âncora/consulta).
    pub raw: String,
    /// Normalizado relativo à raiz do projeto, se resolvível.
    pub resolved: Option<String>,
    /// Alternativa relativa à pasta do documento (código inline pode ser de qualquer um dos dois).
    pub alternative: Option<String>,
    pub syntax: RefSyntax,
    pub line: i64,
    /// Só o nome do arquivo, sem pasta (`package.json`).
    pub bare_name: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    pub name: String,
    pub line: i64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocRefs {
    pub paths: Vec<PathRef>,
    pub symbols: Vec<SymbolRef>,
}

static LINK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"!?\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+[^)]*)?\)").ok());
static CODE_SPAN: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"`([^`\n]+)`").ok());
static IDENT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)(?:\(\))?$").ok());

/// Extensões que fazem um texto inline parecer nome de arquivo.
const FILE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "cs", "rb", "php",
    "swift", "c", "h", "cpp", "hpp", "md", "mdx", "json", "toml", "yaml", "yml", "sql", "sh",
    "ps1", "css", "scss", "html", "vue", "svelte", "lock", "txt", "xml", "gradle", "proto",
];

/// Primeiras pastas que são saída de build ou dependências: citá-las não indica erro.
const GENERATED_ROOTS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    ".git",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
];

/// Palavras que parecem identificadores mas são termos gerais em documentação técnica.
const COMMON_WORDS: &[&str] = &[
    "true",
    "false",
    "null",
    "none",
    "undefined",
    "self",
    "this",
    "async",
    "await",
    "string",
    "number",
    "boolean",
    "object",
    "array",
    "todo",
    "fixme",
    "readme",
    "json",
    "yaml",
    "http",
    "https",
    "localhost",
    "stdin",
    "stdout",
    "stderr",
    "utf8",
    "utf_8",
];

fn has_extension(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && FILE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
    })
}

fn is_external(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower.starts_with('#')
        || lower.starts_with("//")
        || lower.contains("://")
        || ["mailto:", "tel:", "data:", "javascript:"]
            .iter()
            .any(|p| lower.starts_with(p))
}

fn has_placeholder(target: &str) -> bool {
    target.chars().any(|c| "*?{}<>$%~|\"'`[]()…".contains(c)) || target.contains("...")
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(v) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Junta `base` (pasta, relativa à raiz, com `/`) e `rel`, resolvendo `.` e `..`.
/// `None` se sair da raiz.
pub fn join_rel(base: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = if rel.starts_with('/') {
        Vec::new()
    } else {
        base.split('/').filter(|s| !s.is_empty()).collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn is_generated(path: &str) -> bool {
    path.split('/')
        .next()
        .is_some_and(|first| GENERATED_ROOTS.contains(&first))
}

fn looks_like_path(text: &str) -> bool {
    if text.contains(' ') || text.contains('\\') && text.contains(':') {
        return false;
    }
    let t = text.trim_start_matches("./");
    if t.contains('/') {
        let last = t.trim_end_matches('/').rsplit('/').next().unwrap_or(t);
        // Pasta com dois ou mais níveis (`src/auth/`) ou arquivo com extensão conhecida
        // (`src/auth/session.ts`). Pastas de um nível (`logs/`, `backups/`) costumam ser pastas
        // de execução, fora do repositório.
        let dir = t.trim_end_matches('/');
        return (t.ends_with('/') && dir.contains('/')) || has_extension(last);
    }
    has_extension(t)
}

fn looks_like_symbol(text: &str) -> Option<String> {
    let caps = IDENT.as_ref()?.captures(text)?;
    let name = caps.get(1)?.as_str();
    let called = text.ends_with("()");
    if name.chars().count() < 4 || COMMON_WORDS.contains(&name.to_ascii_lowercase().as_str()) {
        return None;
    }
    let has_upper_inner = name
        .char_indices()
        .any(|(i, c)| i > 0 && c.is_ascii_uppercase())
        && name.chars().any(|c| c.is_ascii_lowercase());
    let snake = name.trim_matches('_').contains('_') && name.chars().any(char::is_alphabetic);
    // Só nomes compostos (`createSession`, `MAX_RETRIES`, `verify_hash`) ou chamadas (`run()`):
    // palavras simples em código inline costumam ser termos, comandos ou valores.
    (has_upper_inner || snake || called).then(|| name.to_owned())
}

/// Extrai referências de um documento em `doc_path` (relativo à raiz).
pub fn extract(text: &str, doc_path: &str) -> DocRefs {
    let doc_dir = doc_path.rsplit_once('/').map_or("", |(d, _)| d);
    let mut out = DocRefs::default();
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        let line_no = i as i64 + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        // Blocos indentados e tabelas de comandos também costumam ser exemplos; só cercas
        // são ignoradas — o resto é tratado como texto.
        if in_fence {
            continue;
        }
        if let Some(re) = LINK.as_ref() {
            for caps in re.captures_iter(line) {
                let Some(target) = caps.get(1).map(|m| m.as_str()) else {
                    continue;
                };
                if is_external(target) {
                    continue;
                }
                let clean = target.split(['#', '?']).next().unwrap_or("").trim();
                let clean = percent_decode(clean);
                if clean.is_empty() || has_placeholder(&clean) || clean.contains(':') {
                    continue;
                }
                let resolved = join_rel(doc_dir, &clean);
                if resolved.as_deref().is_some_and(is_generated) {
                    continue;
                }
                out.paths.push(PathRef {
                    raw: clean.clone(),
                    resolved,
                    alternative: None,
                    syntax: RefSyntax::Link,
                    line: line_no,
                    bare_name: false,
                });
            }
        }
        if let Some(re) = CODE_SPAN.as_ref() {
            for caps in re.captures_iter(line) {
                let Some(span) = caps.get(1).map(|m| m.as_str().trim()) else {
                    continue;
                };
                if span.is_empty() {
                    continue;
                }
                if looks_like_path(span) {
                    if is_external(span) || has_placeholder(span) || span.contains(':') {
                        continue;
                    }
                    let t = span.trim_start_matches("./");
                    let bare = !t.trim_end_matches('/').contains('/');
                    let from_root = join_rel("", t);
                    if from_root.as_deref().is_some_and(is_generated) {
                        continue;
                    }
                    let from_doc = join_rel(doc_dir, t).filter(|p| Some(p) != from_root.as_ref());
                    out.paths.push(PathRef {
                        raw: span.to_owned(),
                        resolved: from_root,
                        alternative: from_doc,
                        syntax: RefSyntax::Code,
                        line: line_no,
                        bare_name: bare,
                    });
                } else if let Some(name) = looks_like_symbol(span) {
                    out.symbols.push(SymbolRef {
                        name,
                        line: line_no,
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(text: &str) -> Vec<(String, Option<String>, RefSyntax)> {
        extract(text, "docs/guide.md")
            .paths
            .into_iter()
            .map(|p| (p.raw, p.resolved, p.syntax))
            .collect()
    }

    #[test]
    fn links_resolve_relative_to_the_document() {
        let p = paths("Veja [ADR](decisions/0001.md#contexto) e [raiz](../README.md).");
        assert_eq!(
            p,
            vec![
                (
                    "decisions/0001.md".into(),
                    Some("docs/decisions/0001.md".into()),
                    RefSyntax::Link
                ),
                (
                    "../README.md".into(),
                    Some("README.md".into()),
                    RefSyntax::Link
                ),
            ]
        );
    }

    #[test]
    fn ignores_external_and_placeholder_targets() {
        let text = "[a](https://x.com/a.md) [b](#topo) [c](mailto:x@y) [d](src/*.ts) [e](<id>.md) [f](C:/x.md)";
        assert!(paths(text).is_empty());
    }

    #[test]
    fn code_spans_that_look_like_paths() {
        let r = extract(
            "O código fica em `src/auth/session.ts` e `src/auth/`; config em `package.json`. Rode `npm test`.",
            "docs/a.md",
        );
        let raw: Vec<&str> = r.paths.iter().map(|p| p.raw.as_str()).collect();
        assert_eq!(raw, ["src/auth/session.ts", "src/auth/", "package.json"]);
        assert!(r.paths[2].bare_name);
        assert_eq!(
            r.paths[0].alternative.as_deref(),
            Some("docs/src/auth/session.ts")
        );
    }

    #[test]
    fn fenced_blocks_and_generated_dirs_are_skipped() {
        let text = "```\n`src/exemplo.ts`\n[x](nao/existe.md)\n```\n`target/release/nexus.exe` `node_modules/x/index.js` `logs/` `/`";
        let r = extract(text, "README.md");
        assert!(r.paths.is_empty(), "{:?}", r.paths);
    }

    #[test]
    fn symbols_are_compound_identifiers_only() {
        let r = extract(
            "Chame `createSession` ou `verify_hash()`; constante `MAX_RETRIES`; `run()`; não: `pnpm`, `true`, `Error`, `a.b`.",
            "docs/a.md",
        );
        let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["createSession", "verify_hash", "MAX_RETRIES"]);
    }

    #[test]
    fn join_rel_stays_inside_root() {
        assert_eq!(join_rel("docs", "../a.md").as_deref(), Some("a.md"));
        assert_eq!(join_rel("docs", "../../a.md"), None);
        assert_eq!(join_rel("docs/x", "/src/a.ts").as_deref(), Some("src/a.ts"));
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(percent_decode("meu%20arquivo.md"), "meu arquivo.md");
        assert_eq!(percent_decode("100%"), "100%");
    }
}
