//! Formatação da saída da CLI: humana (pt-BR) ou JSON estável.

use std::fmt::Write as _;
use std::io::Write as _;

use nexus_core::Error;
use nexus_core::diagnostics::status_label;
use nexus_core::time_util::format_utc;
use serde::Serialize;

use crate::{DiagnosticsOutput, InitReport};

pub fn print<T: Serialize>(value: &T, json: bool, human: fn(&T) -> String) -> Result<(), Error> {
    let text = if json {
        let mut s = serde_json::to_string_pretty(value)
            .map_err(|e| Error::Internal(format!("serializar saída: {e}")))?;
        s.push('\n');
        s
    } else {
        human(value)
    };
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|e| Error::io("escrever na saída padrão", e))
}

pub fn print_error(err: &Error, json: bool) {
    let mut stderr = std::io::stderr().lock();
    let text = if json {
        serde_json::to_string(&serde_json::json!({ "error": err.to_info() }))
            .unwrap_or_else(|_| format!("{{\"error\":{{\"message\":\"{err}\"}}}}"))
    } else {
        match err.op_id() {
            Some(op) => format!("erro: {}\n(operação {op})", err.user_message()),
            None => format!("erro: {}", err.user_message()),
        }
    };
    let _ = writeln!(stderr, "{text}");
}

pub fn human_size(bytes: i64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
    }
}

pub fn human_init(r: &InitReport) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "NEXUS pronto.");
    let _ = writeln!(s, "  Dados:          {}", r.data_dir);
    let _ = writeln!(s, "  Banco:          {}", r.database);
    let _ = writeln!(s, "  Schema:         v{}", r.schema_version);
    if r.applied_migrations.is_empty() {
        let _ = writeln!(s, "  Migrations:     nenhuma pendente");
    } else {
        let list: Vec<String> = r.applied_migrations.iter().map(i64::to_string).collect();
        let _ = writeln!(s, "  Migrations:     aplicadas {}", list.join(", "));
    }
    if let Some(b) = &r.backup {
        let _ = writeln!(s, "  Backup prévio:  {b}");
    }
    s
}

pub fn human_diagnostics(r: &DiagnosticsOutput) -> String {
    let d = &r.diagnostics;
    let mut s = String::new();
    let _ = writeln!(s, "NEXUS {} ({})", r.app.app_version, r.app.build_profile);
    let _ = writeln!(s, "  Sistema:        {} {}", r.app.os, r.app.arch);
    let _ = writeln!(
        s,
        "  SQLite:         {} (FTS5 {})",
        d.sqlite_version,
        if d.fts5_available {
            "disponível"
        } else {
            "indisponível"
        }
    );
    let _ = writeln!(s, "  Dados:          {}", d.data_dir);
    let _ = writeln!(
        s,
        "  Banco:          {} ({}; WAL {})",
        d.database.path,
        human_size(d.database.size_bytes),
        human_size(d.wal.size_bytes)
    );
    let _ = writeln!(
        s,
        "  Schema:         v{} (mais recente conhecido: v{})",
        d.schema_version, d.latest_schema_version
    );
    let _ = writeln!(
        s,
        "  Logs:           {} ({} arquivo(s), {})",
        d.logs_dir,
        d.log_files,
        human_size(d.logs_size_bytes)
    );
    let _ = writeln!(
        s,
        "  Backups:        {} em {}",
        d.backups.len(),
        d.backups_dir
    );
    let _ = writeln!(s, "  Operações:      {} registradas", d.operations_total);

    let _ = writeln!(s, "\nMigrations aplicadas:");
    for m in &d.migrations {
        let _ = writeln!(
            s,
            "  v{:<3} {:<20} {}  {} ms  {}",
            m.version,
            m.name,
            format_utc(m.applied_at),
            m.duration_ms,
            &m.checksum[..12.min(m.checksum.len())]
        );
    }

    match (&r.integrity, &d.last_integrity_check) {
        (Some(check), _) => {
            let _ = writeln!(
                s,
                "\nIntegridade:     {} ({} ms, operação {})",
                if check.ok {
                    "ok"
                } else {
                    "PROBLEMAS ENCONTRADOS"
                },
                check.duration_ms,
                check.op_id
            );
            if !check.ok {
                for m in &check.messages {
                    let _ = writeln!(s, "  - {m}");
                }
            }
        }
        (None, Some(last)) => {
            let _ = writeln!(
                s,
                "\nÚltima verificação de integridade: {} ({})",
                format_utc(last.started_at),
                status_label(last.status)
            );
        }
        (None, None) => {
            let _ = writeln!(
                s,
                "\nIntegridade:     nunca verificada (use `nexus diagnostics --check`)"
            );
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::human_size;

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2,0 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5,0 MiB");
    }
}
