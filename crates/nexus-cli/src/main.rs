//! `nexus` — CLI do NEXUS. Adaptador fino sobre `nexus-core`: valida argumentos, chama o core e
//! formata a saída (humana ou `--json`).

mod context_cmd;
mod output;
mod projects_cmd;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use nexus_core::diagnostics::AppInfo;
use nexus_core::logging::{self, Console};
use nexus_core::{Core, DataPaths, Error, ErrorCode};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "nexus",
    version,
    about = "NEXUS — camada de contexto local para engenharia de software",
    long_about = None
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct GlobalArgs {
    /// Diretório de dados (padrão: NEXUS_DATA_DIR ou o diretório de dados do usuário).
    #[arg(long, global = true, value_name = "PASTA")]
    data_dir: Option<PathBuf>,
    /// Saída em JSON para automações.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Cria ou atualiza o banco local e mostra onde os dados ficam.
    Init,
    /// Mostra o estado do banco, migrations, logs e operações recentes.
    Diagnostics {
        /// Também executa a verificação de integridade do banco.
        #[arg(long)]
        check: bool,
    },
    /// Lista, adiciona ou remove projetos autorizados.
    Projects {
        #[command(subcommand)]
        action: Option<ProjectsAction>,
    },
    /// Indexa um projeto (incremental por padrão).
    Index {
        /// ID, nome ou caminho do projeto.
        project: String,
        /// Relê e confere o hash de todos os arquivos.
        #[arg(long)]
        full: bool,
    },
    /// Estado do índice de um projeto (ou de todos).
    Status {
        /// ID, nome ou caminho do projeto.
        project: Option<String>,
    },
    /// Busca no índice de um projeto (código e documentação), com motivos e trechos verificados.
    Search(context_cmd::SearchArgs),
    /// Compila um Context Pack para uma tarefa e o exporta (Markdown por padrão).
    Context(context_cmd::ContextArgs),
}

#[derive(Debug, Subcommand)]
enum ProjectsAction {
    /// Lista os projetos (padrão).
    List,
    /// Autoriza uma pasta como projeto.
    Add {
        /// Pasta do projeto.
        path: PathBuf,
        /// Indexa logo após adicionar.
        #[arg(long)]
        index: bool,
    },
    /// Remove o projeto do índice. Os arquivos originais não são alterados.
    Remove {
        /// ID, nome ou caminho do projeto.
        project: String,
        /// Confirma a remoção.
        #[arg(long)]
        yes: bool,
    },
}

/// Códigos de saída estáveis (documentados em docs/cli.md).
mod exit {
    pub const OK: u8 = 0;
    pub const FAILURE: u8 = 1;
    pub const INVALID_INPUT: u8 = 2;
    pub const STORAGE: u8 = 3;
    pub const INCOMPATIBLE_SCHEMA: u8 = 4;
}

fn exit_code_for(err: &Error) -> u8 {
    match err.code() {
        ErrorCode::InvalidInput | ErrorCode::NotFound => exit::INVALID_INPUT,
        ErrorCode::Storage | ErrorCode::Io => exit::STORAGE,
        ErrorCode::IncompatibleSchema => exit::INCOMPATIBLE_SCHEMA,
        _ => exit::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.global.json;
    match run(cli) {
        Ok(()) => ExitCode::from(exit::OK),
        Err(err) => {
            output::print_error(&err, json);
            ExitCode::from(exit_code_for(&err))
        }
    }
}

fn resolve_paths(global: &GlobalArgs) -> Result<DataPaths, Error> {
    match &global.data_dir {
        Some(dir) => {
            let dir = if dir.is_absolute() {
                dir.clone()
            } else {
                std::env::current_dir()
                    .map_err(|e| Error::io("ler diretório atual", e))?
                    .join(dir)
            };
            Ok(DataPaths::at(dir))
        }
        None => DataPaths::resolve(),
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let paths = resolve_paths(&cli.global)?;
    paths.ensure()?;
    // Logs vão apenas para arquivo; stdout/stderr ficam reservados para a saída da CLI.
    let _log_guard = logging::init(&paths.logs(), Console::Off).ok();
    let core = Core::open(paths)?;

    match cli.command {
        Command::Init => {
            let report = InitReport::from_core(&core);
            core.record_event(
                nexus_core::ops::kinds::CLI_INIT,
                serde_json::json!({ "schemaVersion": report.schema_version }),
            )?;
            output::print(&report, cli.global.json, output::human_init)
        }
        Command::Diagnostics { check } => {
            let integrity = if check {
                Some(core.integrity_check()?)
            } else {
                None
            };
            let report = DiagnosticsOutput {
                app: AppInfo::base(env!("CARGO_PKG_VERSION")),
                diagnostics: core.diagnostics()?,
                integrity,
            };
            output::print(&report, cli.global.json, output::human_diagnostics)
        }
        Command::Projects { action } => {
            let json = cli.global.json;
            match action.unwrap_or(ProjectsAction::List) {
                ProjectsAction::List => projects_cmd::list(&core, json),
                ProjectsAction::Add { path, index } => projects_cmd::add(&core, &path, index, json),
                ProjectsAction::Remove { project, yes } => {
                    projects_cmd::remove(&core, &project, yes, json)
                }
            }
        }
        Command::Index { project, full } => {
            projects_cmd::index(&core, &project, full, cli.global.json)
        }
        Command::Status { project } => {
            projects_cmd::status(&core, project.as_deref(), cli.global.json)
        }
        Command::Search(args) => context_cmd::search(&core, &args, cli.global.json),
        Command::Context(args) => context_cmd::context(&core, &args, cli.global.json),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReport {
    pub data_dir: String,
    pub database: String,
    pub schema_version: i64,
    pub applied_migrations: Vec<i64>,
    pub backup: Option<String>,
}

impl InitReport {
    fn from_core(core: &Core) -> Self {
        let migration = &core.open_report().migration;
        Self {
            data_dir: core.paths().root().display().to_string(),
            database: core.paths().db().display().to_string(),
            schema_version: migration.to_version,
            applied_migrations: migration.applied.clone(),
            backup: migration.backup.as_ref().map(|p| p.display().to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsOutput {
    pub app: AppInfo,
    pub diagnostics: nexus_core::diagnostics::DiagnosticsReport,
    pub integrity: Option<nexus_core::diagnostics::IntegrityReport>,
}
