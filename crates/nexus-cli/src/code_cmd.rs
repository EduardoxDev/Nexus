//! `nexus symbol` e `nexus deps`: declarações e dependências vindas da análise por linguagem.

use std::fmt::Write as _;

use clap::Args;
use nexus_core::analysis::query::{AnalysisOverview, FileAnalysisView, SymbolHit};
use nexus_core::{Core, Error};

use crate::context_cmd::resolve_project;
use crate::output;

#[derive(Debug, Args)]
pub struct SymbolArgs {
    /// Nome do símbolo (comparação sem maiúsculas e sem `_`: `validate_session` ≈ `validateSession`).
    name: String,
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short)]
    project: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: u32,
}

#[derive(Debug, Args)]
pub struct DepsArgs {
    /// Arquivo indexado (relativo à raiz do projeto). Sem arquivo: visão geral do grafo.
    path: Option<String>,
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short)]
    project: Option<String>,
}

fn human_symbols(hits: &Vec<SymbolHit>) -> String {
    if hits.is_empty() {
        return "Nenhuma declaração com esse nome nos arquivos analisados (TS/JS, Python, Rust).\n"
            .to_owned();
    }
    let mut s = String::new();
    for h in hits {
        let container = h
            .symbol
            .container
            .as_ref()
            .map_or(String::new(), |c| format!(" em {c}"));
        let _ = writeln!(
            s,
            "{}:{}-{}  {} {}{}{}",
            h.rel_path,
            h.symbol.line,
            h.symbol.end_line,
            h.symbol.kind.label(),
            h.symbol.name,
            container,
            if h.symbol.exported {
                ""
            } else {
                "  (não exportado)"
            }
        );
    }
    s
}

fn human_file(v: &FileAnalysisView) -> String {
    let mut s = String::new();
    if !v.analyzed {
        let _ = writeln!(
            s,
            "{}: linguagem sem analisador ({}); sem símbolos nem imports.",
            v.rel_path,
            v.language.as_deref().unwrap_or("desconhecida")
        );
        return s;
    }
    let _ = writeln!(
        s,
        "{} ({})",
        v.rel_path,
        v.language.as_deref().unwrap_or("")
    );
    let _ = writeln!(s, "\nDeclarações ({}):", v.symbols.len());
    for sym in &v.symbols {
        let _ = writeln!(
            s,
            "  {:>5}  {} {}{}",
            sym.line,
            sym.kind.label(),
            sym.name,
            sym.container
                .as_ref()
                .map_or(String::new(), |c| format!(" em {c}"))
        );
    }
    let _ = writeln!(s, "\nImporta ({}):", v.imports.len());
    for e in &v.imports {
        let target = match (&e.to_path, &e.external) {
            (Some(p), _) => p.clone(),
            (None, Some(x)) => format!("pacote {x}"),
            (None, None) => "não resolvido".to_owned(),
        };
        let _ = writeln!(s, "  {:>5}  {}  →  {target}", e.line, e.specifier);
    }
    let _ = writeln!(s, "\nImportado por ({}):", v.imported_by.len());
    for e in &v.imported_by {
        let _ = writeln!(s, "  {}:{}  ({})", e.from_path, e.line, e.specifier);
    }
    s
}

fn human_overview(o: &AnalysisOverview) -> String {
    let mut s = String::new();
    match &o.graph {
        Some(g) => {
            let _ = writeln!(
                s,
                "Grafo: {} arquivos, {} arestas ({} internas, {} externas, {} não resolvidas), montado em {} ms.",
                g.files, g.edges, g.resolved, g.external, g.unresolved, g.duration_ms
            );
        }
        None => {
            let _ = writeln!(s, "Grafo ainda não montado: indexe o projeto.");
        }
    }
    let _ = writeln!(
        s,
        "\n{:<12} {:>7} {:>9} {:>8} {:>9} {:>9}",
        "LINGUAGEM", "ARQUIVOS", "SÍMBOLOS", "IMPORTS", "INTERNOS", "EXTERNOS"
    );
    for l in &o.languages {
        let _ = writeln!(
            s,
            "{:<12} {:>7} {:>9} {:>8} {:>9} {:>9}",
            l.language, l.files, l.symbols, l.imports, l.resolved, l.external
        );
    }
    if !o.not_analyzed.is_empty() {
        let list: Vec<String> = o
            .not_analyzed
            .iter()
            .map(|c| format!("{} ({})", c.name, c.count))
            .collect();
        let _ = writeln!(s, "\nSem analisador: {}", list.join(", "));
    }
    if !o.most_imported.is_empty() {
        let _ = writeln!(s, "\nMais importados:");
        for c in &o.most_imported {
            let _ = writeln!(s, "  {:>4}  {}", c.count, c.name);
        }
    }
    s
}

pub fn symbol(core: &Core, args: &SymbolArgs, json: bool) -> Result<(), Error> {
    let pid = resolve_project(core, args.project.as_deref())?;
    let hits = core.find_symbols(pid, &args.name, args.limit)?;
    output::print(&hits, json, human_symbols)
}

pub fn deps(core: &Core, args: &DepsArgs, json: bool) -> Result<(), Error> {
    let pid = resolve_project(core, args.project.as_deref())?;
    match &args.path {
        Some(path) => output::print(&core.file_analysis(pid, path)?, json, human_file),
        None => output::print(&core.analysis_overview(pid)?, json, human_overview),
    }
}
