//! Comandos de projetos e indexação. Toda a lógica está no core; aqui só há entrada e saída.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use nexus_core::index::{IndexJob, IndexMode, IndexObserver, IndexTrigger, JobControl, JobStatus};
use nexus_core::projects::ProjectSummary;
use nexus_core::time_util::format_utc;
use nexus_core::{Core, Error};
use serde::Serialize;

use crate::output::{self, human_size};

pub fn list(core: &Core, json: bool) -> Result<(), Error> {
    let projects = core.list_projects()?;
    output::print(&projects, json, |list: &Vec<ProjectSummary>| {
        if list.is_empty() {
            return "Nenhum projeto autorizado. Use `nexus projects add <pasta>`.\n".to_owned();
        }
        let mut s = String::new();
        let _ = writeln!(
            s,
            "{:>4}  {:<24} {:>9} {:>9}  {:<24} CAMINHO",
            "ID", "NOME", "INDEXADOS", "IGNORADOS", "ÚLTIMA INDEXAÇÃO"
        );
        for p in list {
            let _ = writeln!(
                s,
                "{:>4}  {:<24} {:>9} {:>9}  {:<24} {}{}",
                p.project.id,
                truncate(&p.project.name, 24),
                p.counts.indexed,
                p.counts.skipped,
                p.project.last_indexed_at.map_or("nunca".into(), format_utc),
                p.project.root_path,
                if p.root_exists {
                    ""
                } else {
                    "  (pasta não encontrada)"
                }
            );
        }
        s
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddOutput {
    project: nexus_core::projects::Project,
    job: Option<IndexJob>,
}

pub fn add(core: &Core, path: &Path, index_now: bool, json: bool) -> Result<(), Error> {
    let project = core.add_project(path)?;
    let job = if index_now {
        Some(run_index(core, project.id, IndexMode::Incremental, json)?)
    } else {
        None
    };
    output::print(&AddOutput { project, job }, json, |o: &AddOutput| {
        let mut s = format!(
            "Projeto \"{}\" adicionado (ID {}).\n  Pasta: {}\n",
            o.project.name, o.project.id, o.project.root_path
        );
        match &o.job {
            Some(job) => s.push_str(&human_job(job)),
            None => {
                let _ = writeln!(s, "  Para indexar: nexus index {}", o.project.id);
            }
        }
        s
    })
}

pub fn remove(core: &Core, reference: &str, yes: bool, json: bool) -> Result<(), Error> {
    let project = core.find_project(reference)?;
    if !yes {
        return Err(Error::InvalidInput(format!(
            "confirme com --yes para remover \"{}\" do índice (os arquivos da pasta não são alterados)",
            project.name
        )));
    }
    let removed = core.remove_project(project.id)?;
    output::print(&removed, json, |p: &nexus_core::projects::Project| {
        format!(
            "Projeto \"{}\" removido do índice. Nenhum arquivo em {} foi alterado.\n",
            p.name, p.root_path
        )
    })
}

pub fn index(core: &Core, reference: &str, full: bool, json: bool) -> Result<(), Error> {
    let project = core.find_project(reference)?;
    let mode = if full {
        IndexMode::Full
    } else {
        IndexMode::Incremental
    };
    let job = run_index(core, project.id, mode, json)?;
    output::print(&job, json, |j: &IndexJob| human_job(j))
}

pub fn status(core: &Core, reference: Option<&str>, json: bool) -> Result<(), Error> {
    let summaries = match reference {
        Some(r) => vec![core.project_summary(core.find_project(r)?.id)?],
        None => core.list_projects()?,
    };
    output::print(&summaries, json, |list: &Vec<ProjectSummary>| {
        let mut s = String::new();
        for p in list {
            let _ = writeln!(s, "{} (ID {})", p.project.name, p.project.id);
            let _ = writeln!(s, "  Pasta:      {}", p.project.root_path);
            let _ = writeln!(
                s,
                "  Arquivos:   {} indexados ({}, {} linhas), {} ignorados, {} com erro",
                p.counts.indexed,
                human_size(p.counts.indexed_bytes),
                p.counts.indexed_lines,
                p.counts.skipped,
                p.counts.failed
            );
            let _ = writeln!(s, "  Geração:    {}", p.project.index_generation);
            if let Some(commit) = &p.project.head_commit {
                let _ = writeln!(
                    s,
                    "  Git:        {} {}",
                    p.project.head_ref.as_deref().unwrap_or("(HEAD destacado)"),
                    &commit[..12.min(commit.len())]
                );
            }
            match (&p.active_job, &p.last_job) {
                (Some(job), _) => {
                    let _ = writeln!(
                        s,
                        "  Em andamento: {}/{} processados",
                        job.processed, job.to_process
                    );
                }
                (None, Some(job)) => {
                    let _ = writeln!(
                        s,
                        "  Último job: {} em {}",
                        status_label(job.status),
                        format_utc(job.started_at)
                    );
                }
                (None, None) => {
                    let _ = writeln!(s, "  Ainda não indexado.");
                }
            }
        }
        if list.is_empty() {
            s.push_str("Nenhum projeto autorizado.\n");
        }
        s
    })
}

/// Concordância com "indexação" (feminino).
fn job_status_label(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Running => "em execução",
        JobStatus::Paused => "pausada",
        JobStatus::Succeeded => "concluída",
        JobStatus::Failed => "falhou",
        JobStatus::Cancelled => "cancelada",
        JobStatus::Interrupted => "interrompida",
    }
}

fn phase_label(phase: nexus_core::index::JobPhase) -> &'static str {
    use nexus_core::index::JobPhase;
    match phase {
        JobPhase::Scanning => "varredura",
        JobPhase::Indexing => "indexação",
        JobPhase::Finalizing => "finalização",
        JobPhase::Done => "concluído",
    }
}

fn status_label(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Running => "em execução",
        JobStatus::Paused => "pausado",
        JobStatus::Succeeded => "concluído",
        JobStatus::Failed => "falhou",
        JobStatus::Cancelled => "cancelado",
        JobStatus::Interrupted => "interrompido",
    }
}

/// Progresso no stderr (somente saída humana), no máximo 2 vezes por segundo.
struct StderrProgress {
    last: Mutex<Option<Instant>>,
}

impl IndexObserver for StderrProgress {
    fn on_update(&self, job: &IndexJob) {
        let Ok(mut last) = self.last.lock() else {
            return;
        };
        if last.is_some_and(|t| t.elapsed() < Duration::from_millis(500)) && job.is_active() {
            return;
        }
        *last = Some(Instant::now());
        let mut err = std::io::stderr().lock();
        let _ = write!(
            err,
            "\r  {}: {}/{} processados, {} inalterados   ",
            phase_label(job.phase),
            job.processed,
            job.to_process,
            job.unchanged
        );
        if !job.is_active() {
            let _ = writeln!(err);
        }
    }
}

fn run_index(core: &Core, project_id: i64, mode: IndexMode, json: bool) -> Result<IndexJob, Error> {
    let opts = core.index_options(mode, IndexTrigger::Cli)?;
    let control = JobControl::new();
    if json {
        core.index_project(
            project_id,
            &opts,
            &control,
            &nexus_core::index::NoopObserver,
        )
    } else {
        let progress = StderrProgress {
            last: Mutex::new(None),
        };
        core.index_project(project_id, &opts, &control, &progress)
    }
}

fn human_job(job: &IndexJob) -> String {
    let mut s = String::new();
    let duration = job.finished_at.map(|f| f - job.started_at).unwrap_or(0);
    let _ = writeln!(
        s,
        "Indexação {} em {:.1} s (geração {}).",
        job_status_label(job.status),
        duration as f64 / 1000.0,
        job.generation.map_or("—".into(), |g| g.to_string())
    );
    let _ = writeln!(
        s,
        "  Encontrados {} · processados {} · inalterados {} · ignorados {} · removidos {} · renomeados {} · erros {}",
        job.discovered,
        job.processed,
        job.unchanged,
        job.skipped,
        job.removed,
        job.renamed,
        job.failed
    );
    let _ = writeln!(
        s,
        "  Conteúdos novos {} · reaproveitados {} · lidos {}",
        job.new_contents,
        job.reused_contents,
        human_size(job.bytes_read)
    );
    if let Some(message) = &job.error_message {
        let _ = writeln!(s, "  {message}");
    }
    s
}
