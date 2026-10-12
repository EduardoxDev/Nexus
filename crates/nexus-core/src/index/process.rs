//! Processamento de um arquivo, sem banco: classificação, exclusões, leitura estável, hash e chunks.

use std::path::Path;

use crate::analysis::{self, FileAnalysis};
use crate::chunk::{self, Chunk};
use crate::classify::{self, FileKind};
use crate::scan::{FileEntry, mtime_ns};
use crate::sensitive;

/// Bytes inspecionados para detectar conteúdo binário.
const BINARY_SNIFF: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipStatus {
    Sensitive,
    Binary,
    Large,
    Encoding,
}

impl SkipStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sensitive => "skipped_sensitive",
            Self::Binary => "skipped_binary",
            Self::Large => "skipped_large",
            Self::Encoding => "skipped_encoding",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub language: Option<&'static str>,
    pub kind: FileKind,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    Indexed {
        hash: String,
        line_count: i64,
        chunks: Vec<Chunk>,
        bytes_read: u64,
        /// Símbolos e imports, para linguagens com analisador.
        analysis: Option<FileAnalysis>,
    },
    Skipped {
        status: SkipStatus,
        reason: String,
        bytes_read: u64,
    },
    Failed {
        code: &'static str,
        message: String,
    },
    /// O arquivo mudou enquanto era lido; deve ser tentado de novo.
    Unstable,
}

#[derive(Debug, Clone)]
pub struct Processed {
    pub entry: FileEntry,
    pub meta: Meta,
    pub outcome: Outcome,
}

pub fn process(root: &Path, entry: &FileEntry, max_file_size: u64) -> Processed {
    let meta = Meta {
        language: classify::language(&entry.rel_path),
        kind: classify::kind(&entry.rel_path),
    };
    let outcome = outcome(root, entry, max_file_size, meta.language);
    Processed {
        entry: entry.clone(),
        meta,
        outcome,
    }
}

fn outcome(root: &Path, entry: &FileEntry, max_file_size: u64, language: Option<&str>) -> Outcome {
    if let Some(m) = sensitive::check_name(&entry.rel_path) {
        return Outcome::Skipped {
            status: SkipStatus::Sensitive,
            reason: m.reason,
            bytes_read: 0,
        };
    }
    if classify::is_binary_extension(&entry.rel_path) {
        return Outcome::Skipped {
            status: SkipStatus::Binary,
            reason: "extensão de arquivo binário".into(),
            bytes_read: 0,
        };
    }
    if entry.size > max_file_size {
        return Outcome::Skipped {
            status: SkipStatus::Large,
            reason: format!(
                "maior que o limite de {} KiB ({} KiB)",
                max_file_size / 1024,
                entry.size.div_ceil(1024)
            ),
            bytes_read: 0,
        };
    }

    let path = match crate::scan::check_project_path(root, &entry.rel_path) {
        Ok(crate::scan::ProjectPath::File(p)) => p,
        // Sumiu entre a varredura e a leitura: tenta de novo na próxima passada.
        Ok(crate::scan::ProjectPath::Missing) => return Outcome::Unstable,
        Ok(crate::scan::ProjectPath::Outside) => {
            return Outcome::Failed {
                code: "link_outside",
                message: "virou link para fora do projeto durante a indexação; não foi lido".into(),
            };
        }
        Err(e) => {
            return Outcome::Failed {
                code: "invalid_path",
                message: e.to_string(),
            };
        }
    };
    let bytes = match read_stable(&path, entry) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Outcome::Unstable,
        Err(e) => {
            return Outcome::Failed {
                code: "io",
                message: e.to_string(),
            };
        }
    };
    let bytes_read = bytes.len() as u64;

    if bytes[..bytes.len().min(BINARY_SNIFF)].contains(&0) {
        return Outcome::Skipped {
            status: SkipStatus::Binary,
            reason: "conteúdo binário (byte nulo)".into(),
            bytes_read,
        };
    }
    let text = match decode(&bytes) {
        Some(text) => text,
        None => {
            return Outcome::Skipped {
                status: SkipStatus::Encoding,
                reason: "não é texto UTF-8 válido".into(),
                bytes_read,
            };
        }
    };
    if let Some(m) = sensitive::check_content(&text) {
        return Outcome::Skipped {
            status: SkipStatus::Sensitive,
            reason: m.reason,
            bytes_read,
        };
    }
    Outcome::Indexed {
        hash: blake3::hash(&bytes).to_hex().to_string(),
        line_count: chunk::line_count(&text),
        chunks: chunk::chunk_lines(&text),
        bytes_read,
        analysis: analysis::analyze(language, &text),
    }
}

/// Lê o arquivo e confirma que tamanho e data não mudaram durante a leitura.
/// `Ok(None)` indica instabilidade (arquivo sendo escrito).
pub fn read_stable(path: &Path, expected: &FileEntry) -> std::io::Result<Option<Vec<u8>>> {
    let bytes = std::fs::read(path)?;
    let after = std::fs::metadata(path)?;
    let stable = after.len() == expected.size
        && mtime_ns(&after) == expected.mtime_ns
        && bytes.len() as u64 == expected.size;
    Ok(stable.then_some(bytes))
}

/// UTF-8 (com ou sem BOM). Textos com poucas sequências inválidas são aceitos com substituição;
/// acima de 1% de substituições o arquivo é considerado de outra codificação.
fn decode(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(_) => {
            let lossy = String::from_utf8_lossy(bytes);
            let replaced = lossy.chars().filter(|&c| c == '\u{FFFD}').count();
            (replaced * 100 <= lossy.chars().count().max(1)).then(|| lossy.into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_for(path: &Path, rel: &str) -> FileEntry {
        let meta = std::fs::metadata(path).unwrap();
        FileEntry {
            rel_path: rel.to_owned(),
            size: meta.len(),
            mtime_ns: mtime_ns(&meta),
        }
    }

    #[test]
    fn detects_change_during_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "um").unwrap();
        let mut entry = entry_for(&path, "a.txt");
        assert!(read_stable(&path, &entry).unwrap().is_some());
        // Simula o arquivo mudando entre a varredura e a leitura.
        entry.mtime_ns -= 1_000;
        assert!(read_stable(&path, &entry).unwrap().is_none());
        entry = entry_for(&path, "a.txt");
        std::fs::write(&path, "conteúdo maior agora").unwrap();
        assert!(read_stable(&path, &entry).unwrap().is_none());
    }

    #[test]
    fn decodes_utf8_with_bom_and_rejects_latin1() {
        assert_eq!(decode(b"\xEF\xBB\xBFol\xC3\xA1").as_deref(), Some("olá"));
        let latin1: Vec<u8> = "ação é pública"
            .bytes()
            .map(|b| if b > 127 { 0xE9 } else { b })
            .collect();
        assert!(decode(&latin1).is_none());
    }

    #[test]
    fn outcomes_for_common_cases() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("blob.txt"), b"abc\0def").unwrap();
        std::fs::write(root.join("big.txt"), "x".repeat(2048)).unwrap();

        let p = process(root, &entry_for(&root.join("main.rs"), "main.rs"), 1024);
        assert!(matches!(p.outcome, Outcome::Indexed { line_count: 1, .. }));
        assert_eq!(p.meta.language, Some("rust"));

        let p = process(root, &entry_for(&root.join("blob.txt"), "blob.txt"), 1024);
        assert!(matches!(
            p.outcome,
            Outcome::Skipped {
                status: SkipStatus::Binary,
                ..
            }
        ));

        let p = process(root, &entry_for(&root.join("big.txt"), "big.txt"), 1024);
        assert!(matches!(
            p.outcome,
            Outcome::Skipped {
                status: SkipStatus::Large,
                bytes_read: 0,
                ..
            }
        ));
    }
}
