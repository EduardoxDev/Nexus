//! Evidência verificável: trechos são lidos do disco no momento do uso e comparados com o hash
//! indexado (ADR-0003). Só arquivos com status `indexed` podem ser lidos — nunca sensíveis.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::scan::resolve_rel;

/// Estado de uma fonte em relação ao índice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// O arquivo no disco é idêntico ao indexado.
    Fresh,
    /// O arquivo mudou desde a indexação (linhas podem não corresponder).
    Modified,
    /// O arquivo não existe mais.
    Missing,
}

/// Arquivo indexado, lido e verificado.
pub struct VerifiedFile {
    pub freshness: Freshness,
    /// Linhas atuais do arquivo (vazio se ausente).
    pub lines: Vec<String>,
}

/// Registro mínimo de um arquivo indexado.
#[derive(Debug, Clone)]
pub struct IndexedFile {
    pub rel_path: String,
    pub content_hash: String,
    pub language: Option<String>,
    pub kind: String,
    pub indexed_at: i64,
    pub line_count: i64,
}

/// Busca um arquivo **indexado** do projeto. Arquivos ignorados (sensíveis, binários…) não
/// são devolvidos — a leitura de conteúdo só é permitida para o que já está no índice.
pub fn indexed_file(conn: &Connection, project_id: i64, rel_path: &str) -> Result<IndexedFile> {
    conn.query_row(
        "SELECT rel_path, content_hash, language, kind, indexed_at, COALESCE(line_count, 0)
         FROM files WHERE project_id = ?1 AND rel_path = ?2 AND status = 'indexed'",
        params![project_id, rel_path],
        |r| {
            Ok(IndexedFile {
                rel_path: r.get(0)?,
                content_hash: r.get(1)?,
                language: r.get(2)?,
                kind: r.get(3)?,
                indexed_at: r.get(4)?,
                line_count: r.get(5)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| Error::NotFound(format!("arquivo indexado \"{rel_path}\"")))
}

/// Lê o arquivo e compara com o hash indexado.
pub fn verify(root: &Path, rel_path: &str, content_hash: &str) -> Result<VerifiedFile> {
    let path = resolve_rel(root, rel_path)?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(VerifiedFile {
                freshness: Freshness::Missing,
                lines: Vec::new(),
            });
        }
        Err(e) => return Err(Error::io(format!("ler {rel_path}"), e)),
    };
    let freshness = if blake3::hash(&bytes).to_hex().as_str() == content_hash {
        Freshness::Fresh
    } else {
        Freshness::Modified
    };
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let text = String::from_utf8_lossy(bytes);
    Ok(VerifiedFile {
        freshness,
        lines: text.lines().map(str::to_owned).collect(),
    })
}

/// Linhas `start..=end` (1-based), limitadas ao tamanho atual do arquivo.
pub fn slice(lines: &[String], start: i64, end: i64) -> (i64, i64, String) {
    let total = i64::try_from(lines.len()).unwrap_or(i64::MAX);
    if total == 0 {
        return (start, start.saturating_sub(1), String::new());
    }
    let s = start.clamp(1, total);
    let e = end.clamp(s, total);
    let from = usize::try_from(s - 1).unwrap_or(0);
    let to = usize::try_from(e).unwrap_or(lines.len()).min(lines.len());
    (s, e, lines[from..to].join("\n"))
}

/// Contexto de um arquivo para prévia (MCP e interface).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileContext {
    pub rel_path: String,
    pub language: Option<String>,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub total_lines: i64,
    pub text: String,
    pub freshness: Freshness,
    pub content_hash: String,
    pub indexed_at: i64,
}

pub const MAX_CONTEXT_LINES: i64 = 400;

pub fn file_context(
    conn: &Connection,
    root: &Path,
    project_id: i64,
    rel_path: &str,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<FileContext> {
    let file = indexed_file(conn, project_id, rel_path)?;
    let verified = verify(root, &file.rel_path, &file.content_hash)?;
    let start = start.unwrap_or(1).max(1);
    let end = end.unwrap_or(start + MAX_CONTEXT_LINES - 1);
    if end < start {
        return Err(Error::InvalidInput(
            "a linha final deve ser maior ou igual à inicial".into(),
        ));
    }
    let end = end.min(start + MAX_CONTEXT_LINES - 1);
    let (s, e, text) = slice(&verified.lines, start, end);
    Ok(FileContext {
        rel_path: file.rel_path,
        language: file.language,
        kind: file.kind,
        start_line: s,
        end_line: e,
        total_lines: i64::try_from(verified.lines.len()).unwrap_or(i64::MAX),
        text,
        freshness: verified.freshness,
        content_hash: file.content_hash,
        indexed_at: file.indexed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_clamps_ranges() {
        let lines: Vec<String> = (1..=5).map(|i| format!("l{i}")).collect();
        assert_eq!(slice(&lines, 2, 3), (2, 3, "l2\nl3".to_owned()));
        assert_eq!(slice(&lines, 4, 99), (4, 5, "l4\nl5".to_owned()));
        assert_eq!(slice(&lines, 0, 1), (1, 1, "l1".to_owned()));
        assert_eq!(slice(&[], 3, 9).2, "");
    }

    #[test]
    fn verify_reports_freshness() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "um\ndois\n").unwrap();
        let hash = blake3::hash(b"um\ndois\n").to_hex().to_string();
        let v = verify(dir.path(), "a.txt", &hash).unwrap();
        assert_eq!(v.freshness, Freshness::Fresh);
        assert_eq!(v.lines, ["um", "dois"]);
        std::fs::write(dir.path().join("a.txt"), "mudou\n").unwrap();
        assert_eq!(
            verify(dir.path(), "a.txt", &hash).unwrap().freshness,
            Freshness::Modified
        );
        std::fs::remove_file(dir.path().join("a.txt")).unwrap();
        assert_eq!(
            verify(dir.path(), "a.txt", &hash).unwrap().freshness,
            Freshness::Missing
        );
        assert!(verify(dir.path(), "../fora.txt", &hash).is_err());
    }
}
