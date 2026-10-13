//! Precisão dos analisadores e do grafo contra respostas conferidas à mão
//! (`tests/evaluation/analysis.json`), por linguagem.
//!
//! ```text
//! cargo test -p nexus-core --test analysis -- --nocapture
//! ```
//!
//! Também cobre o comportamento incremental: arestas atualizadas quando arquivos aparecem,
//! somem ou são renomeados, e conteúdo idêntico compartilhando a análise.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::{Core, DataPaths};
use serde::Deserialize;

#[derive(Deserialize)]
struct Truth {
    fixtures: BTreeMap<String, BTreeMap<String, FileTruth>>,
}

#[derive(Deserialize)]
struct FileTruth {
    symbols: Vec<serde_json::Value>,
    imports: Vec<(i64, String, String)>,
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}

fn reindex(core: &Core, pid: i64) {
    core.index_project(
        pid,
        &IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli),
        &JobControl::new(),
        &NoopObserver,
    )
    .unwrap();
}

fn open(fixture: &str) -> (tempfile::TempDir, PathBuf, Core, i64) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join(fixture);
    copy_dir(&repo().join("tests/fixtures").join(fixture), &root);
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(&root).unwrap().id;
    reindex(&core, pid);
    (tmp, root, core, pid)
}

#[derive(Default)]
struct Score {
    tp: usize,
    fp: usize,
    fn_: usize,
}

impl Score {
    fn add(
        &mut self,
        expected: &BTreeSet<String>,
        got: &BTreeSet<String>,
        errors: &mut Vec<String>,
        ctx: &str,
    ) {
        for g in got {
            if expected.contains(g) {
                self.tp += 1;
            } else {
                self.fp += 1;
                errors.push(format!("{ctx}: a mais {g}"));
            }
        }
        for e in expected.difference(got) {
            self.fn_ += 1;
            errors.push(format!("{ctx}: faltou {e}"));
        }
    }
    fn precision(&self) -> f64 {
        self.tp as f64 / (self.tp + self.fp).max(1) as f64
    }
    fn recall(&self) -> f64 {
        self.tp as f64 / (self.tp + self.fn_).max(1) as f64
    }
}

fn family(language: &str) -> &'static str {
    match language {
        "typescript" | "tsx" | "javascript" | "jsx" => "typescript/javascript",
        "python" => "python",
        "rust" => "rust",
        "go" => "go",
        "java" => "java",
        "csharp" => "c#",
        _ => "outra",
    }
}

#[test]
fn analyzers_match_hand_checked_answers() {
    let truth: Truth = serde_json::from_str(
        &std::fs::read_to_string(repo().join("tests/evaluation/analysis.json")).unwrap(),
    )
    .unwrap();
    let mut symbols: BTreeMap<&str, Score> = BTreeMap::new();
    let mut imports: BTreeMap<&str, Score> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut files = 0;

    for (fixture, expected_files) in &truth.fixtures {
        let (_tmp, _root, core, pid) = open(fixture);
        let overview = core.analysis_overview(pid).unwrap();
        assert!(
            overview.graph.is_some(),
            "{fixture}: grafo construído na indexação"
        );
        // Todo arquivo analisável da fixture entra na conta (os ausentes do gabarito esperam zero).
        let mut analyzable: Vec<(String, String)> = Vec::new();
        let _paths: Vec<String> = {
            let db = core.db();
            db.read(|c| {
                let mut stmt = c
                    .prepare(
                        "SELECT rel_path, language FROM files WHERE project_id = ?1 AND status = 'indexed'
                         AND language IN ('typescript','tsx','javascript','jsx','python','rust','go','java','csharp') ORDER BY rel_path",
                    )
                    .unwrap();
                let rows = stmt
                    .query_map([pid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                    .unwrap();
                for row in rows {
                    analyzable.push(row.unwrap());
                }
                Ok(analyzable.iter().map(|(p, _)| p.clone()).collect())
            })
            .unwrap()
        };
        for (path, language) in &analyzable {
            files += 1;
            let fam = family(language);
            let view = core.file_analysis(pid, path).unwrap();
            let empty = FileTruth {
                symbols: Vec::new(),
                imports: Vec::new(),
            };
            let exp = expected_files.get(path).unwrap_or(&empty);
            let exp_symbols: BTreeSet<String> = exp
                .symbols
                .iter()
                .map(|v| {
                    let a = v.as_array().unwrap();
                    let container = a.get(3).and_then(|c| c.as_str()).unwrap_or("");
                    format!(
                        "{} {} L{} {}",
                        a[0].as_str().unwrap(),
                        a[1].as_str().unwrap(),
                        a[2].as_i64().unwrap(),
                        container
                    )
                })
                .collect();
            let got_symbols: BTreeSet<String> = view
                .symbols
                .iter()
                .map(|s| {
                    format!(
                        "{} {} L{} {}",
                        s.name,
                        s.kind.as_str(),
                        s.line,
                        s.container.as_deref().unwrap_or("")
                    )
                })
                .collect();
            symbols.entry(fam).or_default().add(
                &exp_symbols,
                &got_symbols,
                &mut errors,
                &format!("{fixture}/{path} símbolo"),
            );
            let exp_imports: BTreeSet<String> = exp
                .imports
                .iter()
                .map(|(l, s, t)| format!("L{l} {s} -> {t}"))
                .collect();
            let got_imports: BTreeSet<String> = view
                .imports
                .iter()
                .map(|e| {
                    let target = match (&e.to_path, &e.external) {
                        (Some(p), _) => p.clone(),
                        (None, Some(x)) => format!("ext:{x}"),
                        (None, None) => "?".to_owned(),
                    };
                    format!("L{} {} -> {target}", e.line, e.specifier)
                })
                .collect();
            imports.entry(fam).or_default().add(
                &exp_imports,
                &got_imports,
                &mut errors,
                &format!("{fixture}/{path} import"),
            );
        }
    }

    eprintln!("\n### Analisadores ({files} arquivos)\n");
    eprintln!(
        "| Linguagem | Símbolos: precisão | Símbolos: revocação | Imports resolvidos: precisão | Imports resolvidos: revocação |"
    );
    eprintln!("|---|---|---|---|---|");
    for fam in symbols.keys() {
        let s = &symbols[fam];
        let i = &imports[fam];
        eprintln!(
            "| {fam} | {:.3} ({}/{}) | {:.3} | {:.3} ({}/{}) | {:.3} |",
            s.precision(),
            s.tp,
            s.tp + s.fp,
            s.recall(),
            i.precision(),
            i.tp,
            i.tp + i.fp,
            i.recall()
        );
    }
    for e in &errors {
        eprintln!("- {e}");
    }
    // Guardas de regressão (definidas depois da primeira medição; ver docs/evaluation.md).
    for (fam, s) in &symbols {
        assert!(
            s.precision() >= 0.95 && s.recall() >= 0.95,
            "símbolos {fam}: {errors:#?}"
        );
    }
    for (fam, i) in &imports {
        assert!(
            i.precision() >= 0.95 && i.recall() >= 0.95,
            "imports {fam}: {errors:#?}"
        );
    }
}

fn edges_from(core: &Core, pid: i64, path: &str) -> Vec<(String, Option<String>)> {
    core.file_analysis(pid, path)
        .unwrap()
        .imports
        .into_iter()
        .map(|e| (e.specifier, e.to_path))
        .collect()
}

#[test]
fn graph_follows_added_removed_and_renamed_files() {
    let (_tmp, root, core, pid) = open("saas-app");
    // Import para um arquivo que ainda não existe: não resolvido.
    std::fs::write(
        root.join("src/checkout/gift.ts"),
        "import { cartSubtotal } from \"./cart\";\nimport { wrap } from \"./wrapping\";\nexport const gift = () => wrap(cartSubtotal([]));\n",
    )
    .unwrap();
    reindex(&core, pid);
    assert_eq!(
        edges_from(&core, pid, "src/checkout/gift.ts"),
        [
            ("./cart".to_owned(), Some("src/checkout/cart.ts".to_owned())),
            ("./wrapping".to_owned(), None)
        ]
    );
    // O arquivo aparece: a aresta passa a resolver sem que gift.ts mude.
    std::fs::write(
        root.join("src/checkout/wrapping.ts"),
        "export function wrap(x: number) { return x; }\n",
    )
    .unwrap();
    reindex(&core, pid);
    assert_eq!(
        edges_from(&core, pid, "src/checkout/gift.ts")[1]
            .1
            .as_deref(),
        Some("src/checkout/wrapping.ts")
    );
    let wrapping = core.file_analysis(pid, "src/checkout/wrapping.ts").unwrap();
    assert_eq!(wrapping.imported_by.len(), 1);
    assert_eq!(wrapping.symbols[0].name, "wrap");

    // Renomeação: a aresta antiga some (fica não resolvida) até o import ser corrigido.
    std::fs::rename(
        root.join("src/checkout/cart.ts"),
        root.join("src/checkout/basket.ts"),
    )
    .unwrap();
    reindex(&core, pid);
    assert_eq!(edges_from(&core, pid, "src/checkout/gift.ts")[0].1, None);
    let basket = core.file_analysis(pid, "src/checkout/basket.ts").unwrap();
    assert!(
        basket.symbols.iter().any(|s| s.name == "cartSubtotal"),
        "conteúdo reaproveitado mantém a análise"
    );
    assert!(basket.imported_by.is_empty());

    // Símbolos por nome.
    let hits = core.find_symbols(pid, "cart_subtotal", 10).unwrap();
    assert_eq!(hits[0].rel_path, "src/checkout/basket.ts");
    let overview = core.analysis_overview(pid).unwrap();
    let ts = overview
        .languages
        .iter()
        .find(|l| l.language == "typescript")
        .unwrap();
    assert!(ts.files > 20 && ts.symbols > 50 && ts.resolved > 40);
    assert!(
        overview
            .external_packages
            .iter()
            .any(|p| p.name == "vitest")
    );
    assert!(
        overview
            .most_imported
            .iter()
            .any(|p| p.name == "src/utils/money.ts")
    );
}

/// Relatório num repositório real: resolução de imports internos, revocação contra uma busca
/// independente (regex simples de declarações exportadas/públicas de nível superior) e uma
/// amostra determinística de símbolos para conferência manual.
/// `NEXUS_ANALYSIS_REPO=<pasta> cargo test -p nexus-core --test analysis -- --ignored --nocapture`
#[test]
#[ignore = "manual: requer NEXUS_ANALYSIS_REPO"]
fn report_for_real_repository() {
    let Some(repo_dir) = std::env::var_os("NEXUS_ANALYSIS_REPO") else {
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let root = PathBuf::from(&repo_dir);
    let pid = core.add_project(&root).unwrap().id;
    reindex(&core, pid);
    let overview = core.analysis_overview(pid).unwrap();
    println!("{:#?}", overview.graph);
    for l in &overview.languages {
        println!("{l:?}");
    }
    let files: Vec<(String, String)> = core
        .db()
        .read(|c| {
            let mut stmt = c
                .prepare(
                    "SELECT rel_path, language FROM files WHERE project_id = ?1 AND status = 'indexed'
                     AND language IN ('typescript','tsx','rust','python') ORDER BY rel_path",
                )
                .unwrap();
            let rows = stmt
                .query_map([pid], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap();
            Ok(rows.map(|r| r.unwrap()).collect())
        })
        .unwrap();
    let independent = regex::Regex::new(
        r"^(?:export\s+(?:default\s+)?(?:async\s+)?(?:function\*?|class|interface|type|enum|const)|pub\s+(?:async\s+|const\s+|unsafe\s+)*(?:fn|struct|enum|trait|type|const|static|mod))\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
    let (mut expected, mut found) = (0, 0);
    let mut missed = Vec::new();
    let mut unresolved = Vec::new();
    let mut sample = Vec::new();
    for (i, (path, _)) in files.iter().enumerate() {
        let text = std::fs::read_to_string(root.join(path)).unwrap_or_default();
        let view = core.file_analysis(pid, path).unwrap();
        let style = if path.ends_with(".rs") {
            nexus_core::analysis::lexer::RUST
        } else if path.ends_with(".py") {
            nexus_core::analysis::lexer::PYTHON
        } else {
            nexus_core::analysis::lexer::C_LIKE
        };
        let masked = nexus_core::analysis::lexer::mask(&text, style).bare;
        for (n, line) in text.lines().enumerate() {
            // `pub mod x;` é import de arquivo (não declaração); linhas dentro de strings
            // (fontes de teste embutidas) também não contam.
            let masked_line = masked.lines().nth(n).unwrap_or("");
            if line.trim_end().ends_with(';') && line.contains(" mod ") {
                continue;
            }
            // A palavra-chave precisa estar fora de strings: casa no texto mascarado.
            if let Some(c) = independent.captures(masked_line) {
                expected += 1;
                let name = &c[1];
                if view
                    .symbols
                    .iter()
                    .any(|s| s.name == name && s.line == n as i64 + 1)
                {
                    found += 1;
                } else {
                    missed.push(format!("{path}:{} {}", n + 1, line.trim()));
                }
            }
        }
        for e in &view.imports {
            let internal = e.specifier.starts_with('.')
                || e.specifier.starts_with("@/")
                || e.specifier.starts_with("crate::")
                || e.specifier.starts_with("super::")
                || e.specifier.starts_with("self::")
                || e.kind == "module";
            if internal && e.to_path.is_none() {
                unresolved.push(format!("{path}:{} {}", e.line, e.specifier));
            }
        }
        // Amostra: 1 símbolo a cada 7 arquivos, o do meio.
        if i % 7 == 0 && !view.symbols.is_empty() {
            let s = &view.symbols[view.symbols.len() / 2];
            let src = text.lines().nth((s.line - 1) as usize).unwrap_or("").trim();
            sample.push(format!(
                "{path}:{} [{} {}{}] {src}",
                s.line,
                s.kind.as_str(),
                s.name,
                s.container
                    .as_deref()
                    .map(|c| format!(" em {c}"))
                    .unwrap_or_default()
            ));
        }
    }
    println!("\nRevocação contra regex independente: {found}/{expected}");
    for m in &missed {
        println!("  faltou {m}");
    }
    let all_unresolved: Vec<(String, i64, String)> = core
        .db()
        .read(|c| {
            let mut stmt = c
                .prepare(
                    "SELECT from_path, line, specifier FROM edges WHERE project_id = ?1
                     AND to_path IS NULL AND external IS NULL ORDER BY from_path, line",
                )
                .unwrap();
            let rows = stmt
                .query_map([pid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .unwrap();
            Ok(rows.map(|r| r.unwrap()).collect())
        })
        .unwrap();
    let resolved_sample: Vec<(String, i64, String, String)> = core
        .db()
        .read(|c| {
            let mut stmt = c
                .prepare(
                    "SELECT from_path, line, specifier, to_path FROM edges WHERE project_id = ?1
                     AND to_path IS NOT NULL ORDER BY from_path, line",
                )
                .unwrap();
            let rows = stmt
                .query_map([pid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
                .unwrap();
            Ok(rows.map(|r| r.unwrap()).step_by(45).collect())
        })
        .unwrap();
    println!(
        "\nAmostra de arestas resolvidas ({}):",
        resolved_sample.len()
    );
    for (f, l, sp, to) in &resolved_sample {
        println!("  {f}:{l} {sp} -> {to}");
    }
    println!("\nArestas sem destino (todas): {}", all_unresolved.len());
    for (f, l, sp) in &all_unresolved {
        println!("  {f}:{l} {sp}");
    }
    println!("\nImports internos não resolvidos: {}", unresolved.len());
    for u in &unresolved {
        println!("  {u}");
    }
    println!("\nAmostra para conferência ({}):", sample.len());
    for s in &sample {
        println!("  {s}");
    }
}
