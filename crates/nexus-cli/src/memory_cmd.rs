//! `nexus memory` e `nexus freshness`. A CLI age como o usuário (pode confirmar e revisar);
//! toda a lógica e as regras de estado ficam no core.

use std::fmt::Write as _;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use nexus_core::freshness::{
    FreshnessIssue, FreshnessSummary, IssueKind, IssueQuery, IssueStatus, ScanReport, Severity,
};
use nexus_core::memory::adr::ImportReport;
use nexus_core::memory::{
    Actor, Memory, MemoryDetail, MemoryKind, MemoryQuery, MemoryRefInput, MemoryStatus,
    MemoryUpdate, NewMemory, RefState, Transition,
};
use nexus_core::time_util::format_utc;
use nexus_core::{Core, Error};
use serde::Serialize;

use crate::context_cmd::resolve_project;
use crate::output;

const ACTOR: Actor = Actor::Cli;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KindArg {
    Decision,
    Convention,
    Requirement,
    Constraint,
    Bug,
    Solution,
    Dependency,
    Task,
    Lesson,
}

impl From<KindArg> for MemoryKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Decision => Self::Decision,
            KindArg::Convention => Self::Convention,
            KindArg::Requirement => Self::Requirement,
            KindArg::Constraint => Self::Constraint,
            KindArg::Bug => Self::Bug,
            KindArg::Solution => Self::Solution,
            KindArg::Dependency => Self::Dependency,
            KindArg::Task => Self::Task,
            KindArg::Lesson => Self::Lesson,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StatusArg {
    Proposed,
    Confirmed,
    Archived,
    Superseded,
}

impl From<StatusArg> for MemoryStatus {
    fn from(s: StatusArg) -> Self {
        match s {
            StatusArg::Proposed => Self::Proposed,
            StatusArg::Confirmed => Self::Confirmed,
            StatusArg::Archived => Self::Archived,
            StatusArg::Superseded => Self::Superseded,
        }
    }
}

#[derive(Debug, Args)]
pub struct MemoryArgs {
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short, global = true)]
    project: Option<String>,
    #[command(subcommand)]
    action: Option<MemoryAction>,
}

#[derive(Debug, Args)]
pub struct TextArgs {
    /// Texto da memória.
    #[arg(long, conflicts_with = "body_file")]
    body: Option<String>,
    /// Lê o texto de um arquivo (UTF-8).
    #[arg(long, value_name = "ARQUIVO")]
    body_file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum MemoryAction {
    /// Lista memórias (padrão: propostas e confirmadas).
    List {
        /// Filtra por estado (pode repetir).
        #[arg(long = "status", value_enum)]
        statuses: Vec<StatusArg>,
        /// Filtra por tipo (pode repetir).
        #[arg(long = "kind", value_enum)]
        kinds: Vec<KindArg>,
        /// Busca no título e no texto.
        #[arg(long, short)]
        query: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Mostra uma memória, suas referências e o histórico de revisões.
    Show { id: String },
    /// Registra uma memória (proposta, ou confirmada com --confirm).
    Add {
        #[arg(long, value_enum)]
        kind: KindArg,
        #[arg(long)]
        title: String,
        #[command(flatten)]
        text: TextArgs,
        /// Arquivo indexado referenciado: CAMINHO, CAMINHO:LINHA ou CAMINHO:INÍCIO-FIM (pode repetir).
        #[arg(long = "ref", value_name = "CAMINHO[:LINHAS]")]
        refs: Vec<String>,
        /// Confirma já na criação.
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        note: Option<String>,
    },
    /// Edita título, texto, tipo ou referências (cria uma revisão).
    Edit {
        id: String,
        #[arg(long, value_enum)]
        kind: Option<KindArg>,
        #[arg(long)]
        title: Option<String>,
        #[command(flatten)]
        text: TextArgs,
        /// Substitui as referências (pode repetir).
        #[arg(long = "ref", value_name = "CAMINHO[:LINHAS]")]
        refs: Vec<String>,
        /// Remove todas as referências.
        #[arg(long, conflicts_with = "refs")]
        clear_refs: bool,
        /// Revisão esperada (padrão: a atual). Diferente ⇒ conflito (código 5).
        #[arg(long)]
        revision: Option<i64>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Confirma uma proposta.
    Confirm {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Arquiva uma memória.
    Archive {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Volta uma memória arquivada para proposta.
    Restore { id: String },
    /// Registra a versão atual dos arquivos referenciados e corrige caminhos movidos.
    ReviewRefs { id: String },
    /// Importa os ADRs indexados do projeto como propostas.
    ImportAdrs,
}

#[derive(Debug, Args)]
pub struct FreshnessArgs {
    /// ID, nome ou caminho do projeto (opcional se houver só um).
    #[arg(long, short, global = true)]
    project: Option<String>,
    #[command(subcommand)]
    action: Option<FreshnessAction>,
}

#[derive(Debug, Subcommand)]
enum FreshnessAction {
    /// Lista alertas abertos (padrão).
    List {
        /// Inclui dispensados e resolvidos.
        #[arg(long)]
        all: bool,
    },
    /// Verifica agora (documentos, memória e arquivos) e lista os alertas.
    Scan,
    /// Dispensa um alerta (continua dispensado enquanto o problema persistir).
    Dismiss {
        id: i64,
        #[arg(long)]
        note: Option<String>,
    },
    /// Reabre um alerta dispensado.
    Reopen { id: i64 },
}

/// `CAMINHO`, `CAMINHO:12` ou `CAMINHO:10-20`.
fn parse_ref(raw: &str) -> Result<MemoryRefInput, Error> {
    let invalid = || Error::InvalidInput(format!("referência inválida: {raw}"));
    if let Some((path, lines)) = raw.rsplit_once(':')
        && !lines.is_empty()
        && lines.chars().all(|c| c.is_ascii_digit() || c == '-')
    {
        let (a, b) = match lines.split_once('-') {
            Some((a, b)) => (a, Some(b)),
            None => (lines, None),
        };
        let start: i64 = a.parse().map_err(|_| invalid())?;
        let end = b
            .map(str::parse::<i64>)
            .transpose()
            .map_err(|_| invalid())?;
        return Ok(MemoryRefInput {
            rel_path: path.to_owned(),
            start_line: Some(start),
            end_line: end.or(Some(start)),
        });
    }
    Ok(MemoryRefInput {
        rel_path: raw.to_owned(),
        start_line: None,
        end_line: None,
    })
}

fn read_text(text: &TextArgs) -> Result<Option<String>, Error> {
    match (&text.body, &text.body_file) {
        (Some(b), _) => Ok(Some(b.clone())),
        (None, Some(f)) => std::fs::read_to_string(f)
            .map(Some)
            .map_err(|e| Error::io(format!("ler {}", f.display()), e)),
        (None, None) => Ok(None),
    }
}

/// Últimos 8 caracteres: a parte aleatória do UUIDv7 (o início é o carimbo de tempo).
fn short(id: &str) -> &str {
    &id[id.len().saturating_sub(8)..]
}

fn human_list(list: &Vec<Memory>) -> String {
    if list.is_empty() {
        return "Nenhuma memória encontrada.\n".to_owned();
    }
    let mut s = String::new();
    let _ = writeln!(s, "{:<8}  {:<11}  {:<11}  TÍTULO", "ID", "ESTADO", "TIPO");
    for m in list {
        let _ = writeln!(
            s,
            "{:<8}  {:<11}  {:<11}  {}{}",
            short(&m.id),
            m.status.label(),
            m.kind.label(),
            m.title,
            if m.origin_path.is_some() {
                "  (importada)"
            } else {
                ""
            }
        );
    }
    s
}

fn ref_state_label(s: RefState) -> &'static str {
    match s {
        RefState::Unchanged => "inalterado",
        RefState::Changed => "MUDOU desde o registro",
        RefState::Missing => "AUSENTE",
        RefState::Moved => "MOVIDO",
        RefState::NotIndexed => "fora do índice",
    }
}

fn human_detail(d: &MemoryDetail) -> String {
    let m = &d.memory;
    let mut s = String::new();
    let _ = writeln!(s, "{} — {}", m.kind.label(), m.title);
    let _ = writeln!(
        s,
        "  ID: {} · {} · revisão {} · atualizada {}",
        m.id,
        m.status.label(),
        m.revision,
        format_utc(m.updated_at)
    );
    if let Some(origin) = &m.origin_path {
        let _ = writeln!(s, "  Importada de {origin}");
    }
    if let Some(t) = &d.supersedes_title {
        let _ = writeln!(s, "  Substitui: {t}");
    }
    if let Some(t) = &d.superseded_by_title {
        let _ = writeln!(s, "  Substituída por: {t}");
    }
    if !m.body.is_empty() {
        let _ = writeln!(s);
        for line in m.body.lines() {
            let _ = writeln!(s, "  {line}");
        }
    }
    if !d.ref_checks.is_empty() {
        let _ = writeln!(s, "\nReferências:");
        for r in &d.ref_checks {
            let lines = match (r.reference.start_line, r.reference.end_line) {
                (Some(a), Some(b)) => format!(":{a}-{b}"),
                _ => String::new(),
            };
            let moved = r
                .moved_to
                .as_ref()
                .map_or(String::new(), |t| format!(" → {t}"));
            let _ = writeln!(
                s,
                "  {}{lines}  [{}{moved}]",
                r.reference.rel_path,
                ref_state_label(r.state)
            );
        }
    }
    let _ = writeln!(s, "\nHistórico:");
    for r in &d.revisions {
        let note = r.note.as_ref().map_or(String::new(), |n| format!(" — {n}"));
        let _ = writeln!(
            s,
            "  rev {:<3} {}  {:?} por {:?} ({}){note}",
            r.revision,
            format_utc(r.changed_at),
            r.change,
            r.actor,
            r.status.label()
        );
    }
    s
}

fn human_memory(m: &Memory) -> String {
    format!(
        "Memória {} ({}, revisão {}): {}\n",
        short(&m.id),
        m.status.label(),
        m.revision,
        m.title
    )
}

fn human_import(r: &ImportReport) -> String {
    let mut s = format!(
        "{} ADR(s) examinado(s): {} nova(s) proposta(s), {} proposta(s) atualizada(s), {} proposta(s) de substituição, {} sem mudança, {} ignorado(s) por estarem substituídos/rejeitados.\n",
        r.scanned, r.created, r.updated, r.superseding, r.unchanged, r.skipped_inactive
    );
    if !r.skipped_stale.is_empty() {
        let _ = writeln!(
            s,
            "Mudaram desde a indexação (atualize o índice): {}",
            r.skipped_stale.join(", ")
        );
    }
    if r.created + r.updated + r.superseding > 0 {
        s.push_str("Revise com `nexus memory list --status proposed` e confirme com `nexus memory confirm <id>`.\n");
    }
    s
}

pub fn memory(core: &Core, args: &MemoryArgs, json: bool) -> Result<(), Error> {
    let default = MemoryAction::List {
        statuses: Vec::new(),
        kinds: Vec::new(),
        query: None,
        limit: 50,
    };
    match args.action.as_ref().unwrap_or(&default) {
        MemoryAction::List {
            statuses,
            kinds,
            query,
            limit,
        } => {
            let project_id = resolve_project(core, args.project.as_deref())?;
            let statuses = if statuses.is_empty() {
                vec![MemoryStatus::Proposed, MemoryStatus::Confirmed]
            } else {
                statuses.iter().map(|s| (*s).into()).collect()
            };
            let list = core.list_memories(&MemoryQuery {
                project_id,
                statuses: Some(statuses),
                kinds: (!kinds.is_empty()).then(|| kinds.iter().map(|k| (*k).into()).collect()),
                text: query.clone(),
                limit: Some(*limit),
            })?;
            output::print(&list, json, human_list)
        }
        MemoryAction::Show { id } => {
            let id = core.resolve_memory_id(id)?;
            output::print(&core.memory(&id)?, json, human_detail)
        }
        MemoryAction::Add {
            kind,
            title,
            text,
            refs,
            confirm,
            note,
        } => {
            let project_id = resolve_project(core, args.project.as_deref())?;
            let m = core.create_memory(
                &NewMemory {
                    project_id,
                    kind: (*kind).into(),
                    title: title.clone(),
                    body: read_text(text)?.unwrap_or_default(),
                    refs: refs
                        .iter()
                        .map(|r| parse_ref(r))
                        .collect::<Result<_, _>>()?,
                    confirm: *confirm,
                    note: note.clone(),
                },
                ACTOR,
            )?;
            output::print(&m, json, human_memory)
        }
        MemoryAction::Edit {
            id,
            kind,
            title,
            text,
            refs,
            clear_refs,
            revision,
            note,
        } => {
            let id = core.resolve_memory_id(id)?;
            let expected = match revision {
                Some(r) => *r,
                None => core.memory(&id)?.memory.revision,
            };
            let refs = if *clear_refs {
                Some(Vec::new())
            } else if refs.is_empty() {
                None
            } else {
                Some(
                    refs.iter()
                        .map(|r| parse_ref(r))
                        .collect::<Result<_, _>>()?,
                )
            };
            let m = core.update_memory(
                &id,
                &MemoryUpdate {
                    expected_revision: expected,
                    kind: kind.map(Into::into),
                    title: title.clone(),
                    body: read_text(text)?,
                    refs,
                    note: note.clone(),
                },
                ACTOR,
            )?;
            output::print(&m, json, human_memory)
        }
        MemoryAction::Confirm { id, note } => {
            transition(core, id, Transition::Confirm, note.as_deref(), json)
        }
        MemoryAction::Archive { id, note } => {
            transition(core, id, Transition::Archive, note.as_deref(), json)
        }
        MemoryAction::Restore { id } => transition(core, id, Transition::Restore, None, json),
        MemoryAction::ReviewRefs { id } => {
            let id = core.resolve_memory_id(id)?;
            let rev = core.memory(&id)?.memory.revision;
            let m = core.review_memory_refs(&id, rev, ACTOR)?;
            output::print(&m, json, human_memory)
        }
        MemoryAction::ImportAdrs => {
            let project_id = resolve_project(core, args.project.as_deref())?;
            output::print(&core.import_adrs(project_id)?, json, human_import)
        }
    }
}

fn transition(
    core: &Core,
    id: &str,
    to: Transition,
    note: Option<&str>,
    json: bool,
) -> Result<(), Error> {
    let id = core.resolve_memory_id(id)?;
    let m = core.transition_memory(&id, to, None, note, ACTOR)?;
    output::print(&m, json, human_memory)
}

// ---- Atualidade --------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FreshnessOutput {
    summary: FreshnessSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    scan: Option<ScanReport>,
    issues: Vec<FreshnessIssue>,
}

fn issue_tag(i: &FreshnessIssue) -> String {
    let mut tags = vec![if i.severity == Severity::Warning {
        "aviso"
    } else {
        "info"
    }];
    if i.contradiction {
        tags.push("contradição");
    }
    match i.status {
        IssueStatus::Dismissed => tags.push("dispensado"),
        IssueStatus::Resolved => tags.push("resolvido"),
        IssueStatus::Open => {}
    }
    tags.join(", ")
}

fn human_freshness(o: &FreshnessOutput) -> String {
    let mut s = String::new();
    let sum = &o.summary;
    match &sum.last_scan {
        Some(scan) => {
            let _ = writeln!(
                s,
                "Última verificação: {} ({} documento(s), {} memória(s), {} ms){}",
                format_utc(scan.scanned_at),
                scan.docs_checked,
                scan.memories_checked,
                scan.duration_ms,
                if sum.scan_outdated {
                    " — o índice mudou depois dela"
                } else {
                    ""
                }
            );
        }
        None => {
            let _ = writeln!(s, "Nenhuma verificação ainda. Use `nexus freshness scan`.");
        }
    }
    let _ = writeln!(
        s,
        "{} aberto(s): {} aviso(s), {} contradição(ões), {} requerem revisão humana · {} dispensado(s)\n",
        sum.open, sum.warnings, sum.contradictions, sum.needs_review, sum.dismissed
    );
    for i in &o.issues {
        let _ = writeln!(s, "#{} [{}] {}", i.id, issue_tag(i), i.title);
        let _ = writeln!(s, "    {}", i.reason);
        for src in &i.sources {
            let line = src.line.map_or(String::new(), |l| format!(":{l}"));
            let _ = writeln!(s, "    fonte: {}{line}", src.rel_path);
        }
        let _ = writeln!(s, "    recomendação: {}", i.recommendation);
        if i.needs_review {
            let _ = writeln!(s, "    requer revisão humana");
        }
        let _ = writeln!(s, "    detectado em {}", format_utc(i.first_detected_at));
        s.push('\n');
    }
    s
}

fn human_issue(i: &FreshnessIssue) -> String {
    format!("Alerta #{} [{}]: {}\n", i.id, issue_tag(i), i.title)
}

pub fn freshness(core: &Core, args: &FreshnessArgs, json: bool) -> Result<(), Error> {
    match args
        .action
        .as_ref()
        .unwrap_or(&FreshnessAction::List { all: false })
    {
        FreshnessAction::List { all } => {
            let project_id = resolve_project(core, args.project.as_deref())?;
            list_issues(core, project_id, *all, None, json)
        }
        FreshnessAction::Scan => {
            let project_id = resolve_project(core, args.project.as_deref())?;
            let report = core.scan_freshness(project_id)?;
            list_issues(core, project_id, false, Some(report), json)
        }
        FreshnessAction::Dismiss { id, note } => {
            let i = core.set_issue_dismissed(*id, true, note.as_deref())?;
            output::print(&i, json, human_issue)
        }
        FreshnessAction::Reopen { id } => {
            let i = core.set_issue_dismissed(*id, false, None)?;
            output::print(&i, json, human_issue)
        }
    }
}

fn list_issues(
    core: &Core,
    project_id: i64,
    all: bool,
    scan: Option<ScanReport>,
    json: bool,
) -> Result<(), Error> {
    let statuses = if all {
        vec![
            IssueStatus::Open,
            IssueStatus::Dismissed,
            IssueStatus::Resolved,
        ]
    } else {
        vec![IssueStatus::Open]
    };
    let issues = core.freshness_issues(&IssueQuery {
        project_id,
        statuses: Some(statuses),
        kinds: None::<Vec<IssueKind>>,
        memory_id: None,
        limit: Some(1000),
    })?;
    let out = FreshnessOutput {
        summary: core.freshness_summary(project_id)?,
        scan,
        issues,
    };
    output::print(&out, json, human_freshness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_references() {
        let r = parse_ref("src/a.ts:10-20").unwrap();
        assert_eq!(
            (r.rel_path.as_str(), r.start_line, r.end_line),
            ("src/a.ts", Some(10), Some(20))
        );
        let r = parse_ref("src/a.ts:7").unwrap();
        assert_eq!((r.start_line, r.end_line), (Some(7), Some(7)));
        let r = parse_ref("src/a.ts").unwrap();
        assert_eq!(r.start_line, None);
        // Sufixo que não é intervalo de linhas: faz parte do caminho.
        assert_eq!(parse_ref("src/a.ts:1-x").unwrap().start_line, None);
        assert!(parse_ref("src/a.ts:-").is_err());
    }
}
