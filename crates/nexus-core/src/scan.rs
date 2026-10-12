//! Varredura do diretório autorizado. Respeita `.gitignore`, `.ignore` e `.nexusignore`, aplica
//! exclusões padrão e **nunca** segue links simbólicos ou junctions. Não abre conexão com o banco.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::classify::DEFAULT_EXCLUDED_DIRS;
use crate::error::{Error, Result};

/// Nome do arquivo de exclusões próprio do NEXUS (mesma sintaxe do `.gitignore`).
pub const NEXUS_IGNORE_FILE: &str = ".nexusignore";
pub const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Caminho relativo à raiz, com `/`.
    pub rel_path: String,
    pub size: u64,
    pub mtime_ns: i64,
}

#[derive(Debug, Default)]
pub struct ScanOutcome {
    pub files: Vec<FileEntry>,
    /// Links simbólicos/junctions encontrados e não seguidos.
    pub links_skipped: u64,
    /// Entradas que não puderam ser lidas (caminho relativo, mensagem).
    pub errors: Vec<(String, String)>,
}

pub fn mtime_ns(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
        .unwrap_or(0)
}

/// Converte um caminho absoluto sob `root` em relativo com `/`.
pub fn to_rel(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_owned()),
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Resolve um caminho relativo vindo de fora (ex.: MCP, watcher) garantindo que fique sob `root`.
pub fn resolve_rel(root: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() || rel.contains('\0') {
        return Err(Error::InvalidInput(
            "caminho relativo vazio ou inválido".into(),
        ));
    }
    let mut path = root.to_path_buf();
    for part in rel.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                return Err(Error::InvalidInput(format!(
                    "caminho fora do projeto não é permitido: {rel}"
                )));
            }
            p if p.contains(':') => {
                return Err(Error::InvalidInput(format!("caminho inválido: {rel}")));
            }
            p => path.push(p),
        }
    }
    if Path::new(rel).is_absolute() {
        return Err(Error::InvalidInput(format!(
            "caminho absoluto não é permitido: {rel}"
        )));
    }
    Ok(path)
}

#[cfg(windows)]
fn is_reparse_point(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &std::fs::Metadata) -> bool {
    false
}

/// Resultado de conferir um caminho do projeto antes de lê-lo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectPath {
    /// Arquivo comum dentro do projeto.
    File(PathBuf),
    /// Não existe.
    Missing,
    /// É um link (ou passa por um) ou resolve para fora do projeto: não é lido.
    Outside,
}

/// Confere, **no momento da leitura**, que `rel` ainda é um arquivo comum dentro de `root`.
/// A varredura não segue links, mas um arquivo indexado pode ser trocado depois por um link
/// (ou uma pasta por uma junction) apontando para fora do projeto.
pub fn check_project_path(root: &Path, rel: &str) -> Result<ProjectPath> {
    let path = resolve_rel(root, rel)?;
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ProjectPath::Missing),
        Err(e) => return Err(Error::io(format!("inspecionar {rel}"), e)),
    };
    if meta.file_type().is_symlink() || is_reparse_point(&meta) {
        return Ok(ProjectPath::Outside);
    }
    // Pastas intermediárias trocadas por links: o caminho canônico denuncia.
    let (Ok(real), Ok(real_root)) = (dunce::canonicalize(&path), dunce::canonicalize(root)) else {
        return Ok(ProjectPath::Missing);
    };
    if !real.starts_with(&real_root) {
        return Ok(ProjectPath::Outside);
    }
    Ok(ProjectPath::File(path))
}

fn is_link(entry: &ignore::DirEntry) -> bool {
    if entry.path_is_symlink() {
        return true;
    }
    // Junctions e outros reparse points no Windows.
    std::fs::symlink_metadata(entry.path())
        .is_ok_and(|m| m.file_type().is_symlink() || is_reparse_point(&m))
}

/// Percorre `root` e devolve os arquivos candidatos, ordenados por caminho.
/// `should_stop` é consultado periodicamente para cancelamento.
pub fn scan(root: &Path, should_stop: &dyn Fn() -> bool) -> Result<ScanOutcome> {
    let meta = std::fs::metadata(root)
        .map_err(|e| Error::io(format!("ler a raiz do projeto {}", root.display()), e))?;
    if !meta.is_dir() {
        return Err(Error::InvalidInput(format!(
            "{} não é um diretório",
            root.display()
        )));
    }

    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(false)
        .parents(false)
        .ignore(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .require_git(false)
        .follow_links(false)
        .max_depth(Some(MAX_DEPTH))
        .add_custom_ignore_filename(NEXUS_IGNORE_FILE)
        .filter_entry(|entry| {
            let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
            !(is_dir
                && entry.depth() > 0
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| DEFAULT_EXCLUDED_DIRS.contains(&n)))
        });

    let mut out = ScanOutcome::default();
    for (i, item) in builder.build().enumerate() {
        if i % 256 == 0 && should_stop() {
            return Err(Error::Cancelled);
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(err) => {
                out.errors.push((String::new(), err.to_string()));
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        if is_link(&entry) {
            out.links_skipped += 1;
            continue;
        }
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Some(rel_path) = to_rel(root, entry.path()) else {
            out.errors.push((
                entry.path().display().to_string(),
                "caminho com caracteres não suportados (não UTF-8)".into(),
            ));
            continue;
        };
        match entry.metadata() {
            Ok(meta) => out.files.push(FileEntry {
                rel_path,
                size: meta.len(),
                mtime_ns: mtime_ns(&meta),
            }),
            Err(err) => out.errors.push((rel_path, err.to_string())),
        }
    }
    out.files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rel_rejects_escape() {
        let root = Path::new("C:/proj");
        assert!(resolve_rel(root, "../fora.txt").is_err());
        assert!(resolve_rel(root, "a/../../b").is_err());
        assert!(resolve_rel(root, "C:/Windows/win.ini").is_err());
        assert!(resolve_rel(root, "").is_err());
        assert_eq!(
            resolve_rel(root, "src\\main.rs").unwrap(),
            root.join("src").join("main.rs")
        );
    }

    #[test]
    fn to_rel_uses_forward_slashes() {
        let root = Path::new("/r");
        assert_eq!(
            to_rel(root, &root.join("a").join("b.rs")).as_deref(),
            Some("a/b.rs")
        );
        assert_eq!(to_rel(root, root), None);
    }
}
