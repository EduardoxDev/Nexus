//! `nexus search` e `nexus context`.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use nexus_core::classify::FileKind;
use nexus_core::context::ContextPackRequest;
use nexus_core::context::export::{ExportFormat, ExportOptions};
use nexus_core::search::evidence::Freshness;
use nexus_core::search::{SearchMode, SearchRequest, SearchResponse};
use nexus_core::{Core, Error};

use crate::output;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KindArg {
    Code,
    Test,
    Doc,
    Config,
    Manifest,
    Ci,
}

impl From<KindArg> for FileKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Code => FileKind::Code,
            KindArg::Test => FileKind::Test,
            KindArg::Doc => FileKind::Doc,
            KindArg::Config => FileKind::Config,
            KindArg::Manifest => FileKind::Manifest,
            KindArg::Ci => FileKind::Ci,
        }
    }
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Consulta (português ou inglês).
    query: String,
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short)]
    project: Option<String>,
    /// Número de resultados (1–50).
    #[arg(long, default_value_t = 10)]
    limit: u32,
    /// Restringe a tipos de arquivo (pode repetir).
    #[arg(long = "kind", value_enum)]
    kinds: Vec<KindArg>,
    /// Restringe a uma pasta (relativa à raiz do projeto).
    #[arg(long)]
    path: Option<String>,
    /// Só bm25 com os termos digitados (referência; sem expansão nem ranking híbrido).
    #[arg(long)]
    plain: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Md,
    Json,
}

#[derive(Debug, Args)]
pub struct ContextArgs {
    /// Descrição da tarefa.
    task: String,
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short)]
    project: Option<String>,
    /// Orçamento estimado de tokens (1000–50000).
    #[arg(long, default_value_t = 8000)]
    budget: u32,
    /// Máximo de arquivos (1–40).
    #[arg(long, default_value_t = 12)]
    max_files: u32,
    /// Formato da exportação.
    #[arg(long, value_enum, default_value_t = FormatArg::Md)]
    format: FormatArg,
    /// Grava em arquivo em vez de imprimir.
    #[arg(long, short)]
    output: Option<PathBuf>,
    /// Caminhos a omitir do pack (pode repetir).
    #[arg(long)]
    exclude: Vec<String>,
    /// Notas a incluir no pack.
    #[arg(long)]
    notes: Option<String>,
}

/// Projeto pedido ou, se omitido, o único projeto registrado.
pub(crate) fn resolve_project(core: &Core, reference: Option<&str>) -> Result<i64, Error> {
    if let Some(r) = reference {
        return Ok(core.find_project(r)?.id);
    }
    let all = core.list_projects()?;
    match all.as_slice() {
        [only] => Ok(only.project.id),
        [] => Err(Error::NotFound(
            "nenhum projeto autorizado; use `nexus projects add <pasta>`".into(),
        )),
        _ => Err(Error::InvalidInput(
            "há mais de um projeto; informe --project".into(),
        )),
    }
}

fn freshness_label(f: Freshness) -> &'static str {
    match f {
        Freshness::Fresh => "",
        Freshness::Modified => "  [modificado desde a indexação]",
        Freshness::Missing => "  [arquivo ausente]",
    }
}

fn human_search(r: &SearchResponse) -> String {
    let mut s = String::new();
    for w in &r.warnings {
        let _ = writeln!(s, "aviso: {w}");
    }
    if r.hits.is_empty() {
        return s;
    }
    for (i, h) in r.hits.iter().enumerate() {
        let _ = writeln!(
            s,
            "{}. {}:{}-{}  (pontuação {:.3}){}",
            i + 1,
            h.rel_path,
            h.start_line,
            h.end_line,
            h.score,
            freshness_label(h.freshness)
        );
        let reasons: Vec<&str> = h
            .reasons
            .iter()
            .filter(|r| r.weight > 0.0)
            .map(|r| r.label.as_str())
            .collect();
        if !reasons.is_empty() {
            let _ = writeln!(s, "   motivos: {}", reasons.join(" · "));
        }
        if !h.also_in.is_empty() {
            let _ = writeln!(s, "   também em: {}", h.also_in.join(", "));
        }
        for (n, line) in (h.start_line..).zip(h.snippet.lines().take(6)) {
            let marker = if h.match_lines.contains(&n) { '>' } else { ' ' };
            let _ = writeln!(s, "   {marker}{n:>5} │ {line}");
        }
        s.push('\n');
    }
    let _ = writeln!(
        s,
        "{} resultado(s) de {} candidato(s) em {} ms (geração do índice {}).",
        r.hits.len(),
        r.candidates,
        r.took_ms,
        r.index_generation
    );
    s
}

pub fn search(core: &Core, args: &SearchArgs, json: bool) -> Result<(), Error> {
    let project_id = resolve_project(core, args.project.as_deref())?;
    let response = core.search(&SearchRequest {
        project_id,
        query: args.query.clone(),
        limit: Some(args.limit),
        kinds: (!args.kinds.is_empty()).then(|| args.kinds.iter().map(|k| (*k).into()).collect()),
        path_prefix: args.path.clone(),
        mode: Some(if args.plain {
            SearchMode::Plain
        } else {
            SearchMode::Hybrid
        }),
    })?;
    output::print(&response, json, human_search)
}

pub fn context(core: &Core, args: &ContextArgs, json: bool) -> Result<(), Error> {
    let project_id = resolve_project(core, args.project.as_deref())?;
    let pack = core.compile_context_pack(&ContextPackRequest {
        project_id,
        task: args.task.clone(),
        token_budget: Some(args.budget),
        max_files: Some(args.max_files),
    })?;
    let format = if json {
        ExportFormat::Json
    } else {
        match args.format {
            FormatArg::Md => ExportFormat::Markdown,
            FormatArg::Json => ExportFormat::Json,
        }
    };
    let options = ExportOptions {
        excluded_paths: args.exclude.clone(),
        notes: args.notes.clone(),
        ..Default::default()
    };
    let rendered = core.export_context_pack(&pack.plan.id, format, &options)?;
    match &args.output {
        Some(path) => {
            std::fs::write(path, &rendered)
                .map_err(|e| Error::io(format!("gravar {}", path.display()), e))?;
            let mut err = std::io::stderr().lock();
            let _ = writeln!(
                err,
                "Context Pack {} gravado em {} ({} fontes, ~{} tokens estimados).",
                pack.plan.id,
                path.display(),
                pack.plan.sources.len(),
                pack.plan.estimated_tokens
            );
            Ok(())
        }
        None => {
            let mut out = std::io::stdout().lock();
            out.write_all(rendered.as_bytes())
                .and_then(|()| out.flush())
                .map_err(|e| Error::io("escrever na saída padrão", e))
        }
    }
}
