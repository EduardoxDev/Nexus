//! Registro de projetos autorizados e visão geral do índice. Remover um projeto apaga apenas os
//! dados do índice — nunca os arquivos originais.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::classify::FileKind;
use crate::error::{Error, Result};
use crate::index::job::{self, IndexJob};
use crate::time_util::now_ms;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub created_at: i64,
    pub last_opened_at: i64,
    pub last_indexed_at: Option<i64>,
    pub index_generation: i64,
    pub head_ref: Option<String>,
    pub head_commit: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileCounts {
    pub total: i64,
    pub indexed: i64,
    pub skipped: i64,
    pub failed: i64,
    /// Bytes dos arquivos indexados.
    pub indexed_bytes: i64,
    pub indexed_lines: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project: Project,
    /// A pasta ainda existe no disco?
    pub root_exists: bool,
    pub counts: FileCounts,
    pub active_job: Option<IndexJob>,
    pub last_job: Option<IndexJob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStat {
    pub language: String,
    pub files: i64,
    pub bytes: i64,
    pub lines: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KindStat {
    pub kind: FileKind,
    pub files: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StatusStat {
    pub status: String,
    pub files: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DirStat {
    /// Diretório de primeiro nível (`""` = arquivos na raiz).
    pub name: String,
    pub files: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub rel_path: String,
    pub status: String,
    pub reason: Option<String>,
    pub size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectOverview {
    pub summary: ProjectSummary,
    pub languages: Vec<LanguageStat>,
    pub kinds: Vec<KindStat>,
    pub statuses: Vec<StatusStat>,
    pub top_directories: Vec<DirStat>,
    /// Manifestos e lockfiles encontrados (caminhos relativos).
    pub manifests: Vec<String>,
    pub ci_files: Vec<String>,
    /// Arquivos mais alterados observados pelo NEXUS desde que o projeto foi adicionado.
    pub most_changed: Vec<ChangedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub rel_path: String,
    pub change_count: i64,
}

/// Chave de comparação de caminhos (Windows não diferencia maiúsculas).
fn path_key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.trim_end_matches('/').to_owned();
    if cfg!(windows) { s.to_lowercase() } else { s }
}

fn is_within(child: &Path, parent: &Path) -> bool {
    let (c, p) = (path_key(child), path_key(parent));
    c == p || c.starts_with(&format!("{p}/"))
}

/// Caminho real (sem `..`, links ou nomes curtos 8.3 do Windows) para comparar com `path`, que já
/// chega canônico. Se a pasta não existe, compara o texto como veio.
fn real(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Pastas do sistema no Linux/macOS. No Windows, vêm das variáveis `SystemRoot`, `ProgramFiles`...
/// `/home`, `/Users`, `/tmp` e `/var` inteira ficam de fora: projetos e a pasta temporária do
/// macOS (`/private/var/folders`) vivem dentro delas.
const UNIX_SYSTEM_DIRS: &[&str] = &[
    "/usr",
    "/etc",
    "/bin",
    "/sbin",
    "/lib",
    "/var/lib",
    "/var/log",
    "/opt",
    "/proc",
    "/sys",
    "/dev",
    "/boot",
    "/System",
    "/Library",
    "/Applications",
    "/private/etc",
    "/private/var/db",
];

/// Pastas que nunca podem ser autorizadas como projeto inteiro.
fn forbidden_reason(path: &Path, data_dir: &Path) -> Option<String> {
    if path.parent().is_none() {
        return Some("a raiz de um disco não pode ser um projeto".into());
    }
    for dir in UNIX_SYSTEM_DIRS.iter().filter(|_| cfg!(unix)) {
        let dir = real(Path::new(dir));
        // A pasta e qualquer coisa dentro dela; as acima (ex.: a raiz) já foram tratadas.
        if is_within(path, &dir) {
            return Some(format!(
                "pastas do sistema não podem ser indexadas ({})",
                dir.display()
            ));
        }
    }
    if let Some(home) = dirs::home_dir().and_then(|h| dunce::canonicalize(h).ok())
        && is_within(&home, path)
    {
        return Some("a pasta pessoal (ou uma pasta acima dela) não pode ser um projeto inteiro; escolha a pasta do repositório".into());
    }
    for var in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ] {
        if let Some(dir) = std::env::var_os(var).map(|d| real(Path::new(&d)))
            && (is_within(path, &dir) || is_within(&dir, path))
        {
            return Some(format!(
                "pastas do sistema não podem ser indexadas ({})",
                dir.display()
            ));
        }
    }
    let data_dir = real(data_dir);
    if is_within(path, &data_dir) || is_within(&data_dir, path) {
        return Some("a pasta de dados do NEXUS não pode ser indexada".into());
    }
    None
}

const PROJECT_COLUMNS: &str = "id, name, root_path, created_at, last_opened_at, last_indexed_at,
    index_generation, head_ref, head_commit";

fn row_to_project(r: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: r.get(0)?,
        name: r.get(1)?,
        root_path: r.get(2)?,
        created_at: r.get(3)?,
        last_opened_at: r.get(4)?,
        last_indexed_at: r.get(5)?,
        index_generation: r.get(6)?,
        head_ref: r.get(7)?,
        head_commit: r.get(8)?,
    })
}

/// Valida e registra uma pasta. O caminho é canonicalizado; links são resolvidos aqui, uma única vez.
pub fn add(conn: &Connection, path: &Path, data_dir: &Path) -> Result<Project> {
    if path.as_os_str().is_empty() {
        return Err(Error::InvalidInput("informe a pasta do projeto".into()));
    }
    let canonical = dunce::canonicalize(path).map_err(|e| {
        Error::InvalidInput(format!(
            "a pasta {} não pode ser aberta: {e}",
            path.display()
        ))
    })?;
    if !canonical.is_dir() {
        return Err(Error::InvalidInput(format!(
            "{} não é uma pasta",
            canonical.display()
        )));
    }
    if let Some(reason) = forbidden_reason(&canonical, data_dir) {
        return Err(Error::InvalidInput(reason));
    }
    for existing in list(conn)? {
        let other = Path::new(&existing.root_path);
        if path_key(other) == path_key(&canonical) {
            return Err(Error::Conflict(format!(
                "esta pasta já é o projeto \"{}\"",
                existing.name
            )));
        }
        if is_within(&canonical, other) || is_within(other, &canonical) {
            return Err(Error::Conflict(format!(
                "a pasta se sobrepõe ao projeto \"{}\" ({})",
                existing.name, existing.root_path
            )));
        }
    }
    let name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| canonical.display().to_string());
    let now = now_ms();
    conn.execute(
        "INSERT INTO projects (name, root_path, created_at, last_opened_at) VALUES (?1, ?2, ?3, ?3)",
        params![name, canonical.to_string_lossy(), now],
    )?;
    get(conn, conn.last_insert_rowid())?
        .ok_or_else(|| Error::Internal("projeto recém-criado não encontrado".into()))
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Project>> {
    Ok(conn
        .query_row(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?1"),
            [id],
            row_to_project,
        )
        .optional()?)
}

pub fn require(conn: &Connection, id: i64) -> Result<Project> {
    get(conn, id)?.ok_or_else(|| Error::NotFound(format!("projeto {id}")))
}

/// Projetos ordenados pelo uso mais recente.
pub fn list(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PROJECT_COLUMNS} FROM projects ORDER BY last_opened_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], row_to_project)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Localiza um projeto por ID, nome ou caminho (usado pela CLI).
pub fn find(conn: &Connection, reference: &str) -> Result<Project> {
    if let Ok(id) = reference.parse::<i64>()
        && let Some(p) = get(conn, id)?
    {
        return Ok(p);
    }
    let all = list(conn)?;
    let by_name: Vec<&Project> = all
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case(reference))
        .collect();
    if by_name.len() == 1 {
        return Ok(by_name[0].clone());
    }
    if by_name.len() > 1 {
        return Err(Error::InvalidInput(format!(
            "há mais de um projeto chamado \"{reference}\"; use o ID"
        )));
    }
    if let Ok(canonical) = dunce::canonicalize(reference)
        && let Some(p) = all
            .iter()
            .find(|p| path_key(Path::new(&p.root_path)) == path_key(&canonical))
    {
        return Ok(p.clone());
    }
    Err(Error::NotFound(format!("projeto \"{reference}\"")))
}

pub fn touch(conn: &Connection, id: i64) -> Result<()> {
    let n = conn.execute(
        "UPDATE projects SET last_opened_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    if n == 0 {
        return Err(Error::NotFound(format!("projeto {id}")));
    }
    Ok(())
}

/// Remove o projeto do índice (arquivos, conteúdo, FTS, jobs). Os arquivos no disco não são tocados.
pub fn remove(conn: &mut Connection, id: i64) -> Result<Project> {
    let project = require(conn, id)?;
    job::recover_stale(conn, Some(id), now_ms())?;
    if job::active(conn, id)?.is_some() {
        return Err(Error::Conflict(
            "há uma indexação em andamento; cancele-a antes de remover o projeto".into(),
        ));
    }
    let tx = conn.transaction()?;
    // Ordem explícita: arquivos referenciam conteúdos (FK adiada); chunks disparam a limpeza do FTS.
    tx.execute("DELETE FROM files WHERE project_id = ?1", [id])?;
    tx.execute("DELETE FROM contents WHERE project_id = ?1", [id])?;
    tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(project)
}

pub fn counts(conn: &Connection, id: i64) -> Result<FileCounts> {
    Ok(conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(status = 'indexed'), 0),
                COALESCE(SUM(status LIKE 'skipped_%'), 0),
                COALESCE(SUM(status = 'error'), 0),
                COALESCE(SUM(CASE WHEN status = 'indexed' THEN size ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'indexed' THEN line_count ELSE 0 END), 0)
         FROM files WHERE project_id = ?1",
        [id],
        |r| {
            Ok(FileCounts {
                total: r.get(0)?,
                indexed: r.get(1)?,
                skipped: r.get(2)?,
                failed: r.get(3)?,
                indexed_bytes: r.get(4)?,
                indexed_lines: r.get(5)?,
            })
        },
    )?)
}

pub fn summary(conn: &Connection, project: Project) -> Result<ProjectSummary> {
    let counts = counts(conn, project.id)?;
    let active_job = job::active(conn, project.id)?;
    let last_job = job::list(conn, project.id, 1)?.into_iter().next();
    Ok(ProjectSummary {
        root_exists: Path::new(&project.root_path).is_dir(),
        project,
        counts,
        active_job,
        last_job,
    })
}

pub fn summaries(conn: &Connection) -> Result<Vec<ProjectSummary>> {
    list(conn)?.into_iter().map(|p| summary(conn, p)).collect()
}

fn strings(conn: &Connection, sql: &str, id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([id], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn overview(conn: &Connection, id: i64) -> Result<ProjectOverview> {
    let summary = summary(conn, require(conn, id)?)?;

    let mut stmt = conn.prepare(
        "SELECT language, COUNT(*), SUM(size), COALESCE(SUM(line_count), 0) FROM files
         WHERE project_id = ?1 AND status = 'indexed' AND language IS NOT NULL
           AND kind <> 'lockfile'
         GROUP BY language ORDER BY SUM(size) DESC LIMIT 12",
    )?;
    let languages = stmt
        .query_map([id], |r| {
            Ok(LanguageStat {
                language: r.get(0)?,
                files: r.get(1)?,
                bytes: r.get(2)?,
                lines: r.get(3)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT kind, COUNT(*) FROM files WHERE project_id = ?1 AND status = 'indexed'
         GROUP BY kind ORDER BY COUNT(*) DESC",
    )?;
    let kinds = stmt
        .query_map([id], |r| {
            Ok(KindStat {
                kind: FileKind::parse(&r.get::<_, String>(0)?),
                files: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT status, COUNT(*) FROM files WHERE project_id = ?1 GROUP BY status ORDER BY COUNT(*) DESC",
    )?;
    let statuses = stmt
        .query_map([id], |r| {
            Ok(StatusStat {
                status: r.get(0)?,
                files: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT CASE WHEN instr(rel_path, '/') = 0 THEN '' ELSE substr(rel_path, 1, instr(rel_path, '/') - 1) END AS dir,
                COUNT(*)
         FROM files WHERE project_id = ?1 AND status = 'indexed'
         GROUP BY dir ORDER BY COUNT(*) DESC, dir LIMIT 12",
    )?;
    let top_directories = stmt
        .query_map([id], |r| {
            Ok(DirStat {
                name: r.get(0)?,
                files: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    let manifests = strings(
        conn,
        "SELECT rel_path FROM files WHERE project_id = ?1 AND kind IN ('manifest', 'lockfile')
         ORDER BY length(rel_path), rel_path LIMIT 30",
        id,
    )?;
    let ci_files = strings(
        conn,
        "SELECT rel_path FROM files WHERE project_id = ?1 AND kind = 'ci' ORDER BY rel_path LIMIT 30",
        id,
    )?;
    let mut stmt = conn.prepare(
        "SELECT rel_path, change_count FROM files WHERE project_id = ?1 AND change_count > 0
         ORDER BY change_count DESC, rel_path LIMIT 10",
    )?;
    let most_changed = stmt
        .query_map([id], |r| {
            Ok(ChangedFile {
                rel_path: r.get(0)?,
                change_count: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    Ok(ProjectOverview {
        summary,
        languages,
        kinds,
        statuses,
        top_directories,
        manifests,
        ci_files,
        most_changed,
    })
}

/// Arquivos não indexados (sensíveis, binários, grandes, codificação, erro), com o motivo.
pub fn skipped_files(conn: &Connection, id: i64, limit: u32) -> Result<Vec<SkippedFile>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, status, skip_reason, size FROM files
         WHERE project_id = ?1 AND status <> 'indexed'
         ORDER BY CASE status WHEN 'error' THEN 0 WHEN 'skipped_sensitive' THEN 1 ELSE 2 END, rel_path
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![id, limit], |r| {
        Ok(SkippedFile {
            rel_path: r.get(0)?,
            status: r.get(1)?,
            reason: r.get(2)?,
            size: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_is_component_aware() {
        assert!(is_within(Path::new("C:/a/b"), Path::new("C:/a")));
        assert!(is_within(Path::new("C:/a"), Path::new("C:/a/")));
        assert!(!is_within(Path::new("C:/ab"), Path::new("C:/a")));
    }
}
