//! Logging estruturado local: JSON Lines diário em `logs/` (7 arquivos) e console opcional.
//! Política: nunca registrar conteúdo de arquivos nem valores sensíveis.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, fmt};

use crate::error::{Error, Result};

/// Variável de ambiente com o filtro de log (sintaxe `EnvFilter`), padrão `info`.
pub const LOG_FILTER_ENV: &str = "NEXUS_LOG";
const RETAINED_FILES: usize = 7;

/// Mantém o writer assíncrono vivo; os logs pendentes são descarregados no `drop`.
pub struct LogGuard {
    _guard: WorkerGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Console {
    /// Sem saída no terminal (ex.: servidor MCP, onde stdout é o protocolo).
    Off,
    /// Logs legíveis em stderr.
    Stderr,
}

pub fn init(logs_dir: &Path, console: Console) -> Result<LogGuard> {
    std::fs::create_dir_all(logs_dir)
        .map_err(|e| Error::io(format!("criar {}", logs_dir.display()), e))?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("nexus")
        .filename_suffix("jsonl")
        .max_log_files(RETAINED_FILES)
        .build(logs_dir)
        .map_err(|e| Error::Internal(format!("não foi possível iniciar o log em arquivo: {e}")))?;
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter =
        || EnvFilter::try_from_env(LOG_FILTER_ENV).unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_target(true)
        .with_writer(writer)
        .with_filter(filter());
    let console_layer = (console == Console::Stderr).then(|| {
        fmt::layer()
            .compact()
            .with_writer(std::io::stderr)
            .with_filter(filter())
    });

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .try_init()
        .map_err(|e| Error::Internal(format!("logging já inicializado: {e}")))?;

    install_panic_hook();
    Ok(LogGuard { _guard: guard })
}

/// Registra panics no log antes do comportamento padrão.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        tracing::error!(location, panic = %info, "panic");
        previous(info);
    }));
}
