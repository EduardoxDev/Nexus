//! Memória do projeto: decisões, convenções, requisitos e outros conhecimentos registrados ou
//! confirmados pelo usuário, com referências verificáveis a arquivos e histórico completo.
//!
//! Regras (ADR-0011):
//! - Estados: `proposed` → `confirmed` → `archived` | `superseded`; `archived` → `proposed`.
//! - Só quem revisa (o usuário, pelo app ou pela CLI) confirma, arquiva ou edita uma memória
//!   confirmada. Outras fontes (importação de ADRs, agentes) só criam ou ajustam **propostas**.
//! - Uma mudança numa decisão confirmada vinda de outra fonte vira uma nova proposta que a
//!   substitui (`supersedes`); a confirmada só muda de estado quando o usuário confirma a nova.
//! - Toda mudança grava uma revisão completa (retrato de título, corpo, estado e referências).
//! - Edições usam controle otimista (`expected_revision`): duas janelas não se sobrescrevem.

pub mod adr;
pub mod shared;

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::search::query::{analyze, fts_expression};
use crate::time_util::now_ms;

const MIN_TITLE_CHARS: usize = 3;
const MAX_TITLE_CHARS: usize = 200;
pub const MAX_BODY_CHARS: usize = 20_000;
const MAX_REFS: usize = 20;
const MAX_NOTE_CHARS: usize = 1_000;
const DEFAULT_LIST_LIMIT: u32 = 100;
pub const MAX_LIST_LIMIT: u32 = 500;
const MAX_QUERY_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
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

impl MemoryKind {
    pub const ALL: [Self; 9] = [
        Self::Decision,
        Self::Convention,
        Self::Requirement,
        Self::Constraint,
        Self::Bug,
        Self::Solution,
        Self::Dependency,
        Self::Task,
        Self::Lesson,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Convention => "convention",
            Self::Requirement => "requirement",
            Self::Constraint => "constraint",
            Self::Bug => "bug",
            Self::Solution => "solution",
            Self::Dependency => "dependency",
            Self::Task => "task",
            Self::Lesson => "lesson",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// Rótulo pt-BR (CLI e exportação).
    pub fn label(self) -> &'static str {
        match self {
            Self::Decision => "Decisão",
            Self::Convention => "Convenção",
            Self::Requirement => "Requisito",
            Self::Constraint => "Restrição",
            Self::Bug => "Bug",
            Self::Solution => "Solução",
            Self::Dependency => "Dependência",
            Self::Task => "Tarefa",
            Self::Lesson => "Lição",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    /// Ainda não revisada pelo usuário.
    Proposed,
    /// Revisada e aceita pelo usuário.
    Confirmed,
    /// Descartada ou aposentada pelo usuário.
    Archived,
    /// Substituída por outra memória confirmada.
    Superseded,
}

impl MemoryStatus {
    pub const ALL: [Self; 4] = [
        Self::Proposed,
        Self::Confirmed,
        Self::Archived,
        Self::Superseded,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Confirmed => "confirmed",
            Self::Archived => "archived",
            Self::Superseded => "superseded",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "proposta",
            Self::Confirmed => "confirmada",
            Self::Archived => "arquivada",
            Self::Superseded => "substituída",
        }
    }

    /// Ainda vale (entra em verificações de atualidade)?
    pub fn is_active(self) -> bool {
        matches!(self, Self::Proposed | Self::Confirmed)
    }
}

/// Quem originou a memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOrigin {
    User,
    Cli,
    AdrImport,
    Agent,
    /// Lida de um arquivo do repositório (`.nexus/memory/`, ADR-0014).
    SharedFile,
}

impl MemoryOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Cli => "cli",
            Self::AdrImport => "adr_import",
            Self::Agent => "agent",
            Self::SharedFile => "shared_file",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "cli" => Self::Cli,
            "adr_import" => Self::AdrImport,
            "agent" => Self::Agent,
            "shared_file" => Self::SharedFile,
            _ => Self::User,
        }
    }
}

/// Quem está fazendo a mudança. Só `User` e `Cli` revisam (confirmam, arquivam, editam
/// confirmadas); `AdrImport`, `Agent` e `SharedFile` só propõem; `System` faz ajustes de
/// metadados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    User,
    Cli,
    AdrImport,
    Agent,
    System,
    /// Leitura de `.nexus/memory/` (arquivos vindos do repositório).
    SharedFile,
}

impl Actor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Cli => "cli",
            Self::AdrImport => "adr_import",
            Self::Agent => "agent",
            Self::System => "system",
            Self::SharedFile => "shared_file",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "cli" => Self::Cli,
            "adr_import" => Self::AdrImport,
            "agent" => Self::Agent,
            "system" => Self::System,
            "shared_file" => Self::SharedFile,
            _ => Self::User,
        }
    }

    /// Pode revisar (confirmar, arquivar, editar o que já foi confirmado)?
    pub fn can_review(self) -> bool {
        matches!(self, Self::User | Self::Cli)
    }

    fn origin(self) -> MemoryOrigin {
        match self {
            Self::Cli => MemoryOrigin::Cli,
            Self::AdrImport => MemoryOrigin::AdrImport,
            Self::Agent => MemoryOrigin::Agent,
            Self::SharedFile => MemoryOrigin::SharedFile,
            Self::User | Self::System => MemoryOrigin::User,
        }
    }
}

/// O que mudou numa revisão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RevisionChange {
    Created,
    Edited,
    Confirmed,
    Archived,
    Restored,
    Superseded,
    RefsReviewed,
}

impl RevisionChange {
    fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Edited => "edited",
            Self::Confirmed => "confirmed",
            Self::Archived => "archived",
            Self::Restored => "restored",
            Self::Superseded => "superseded",
            Self::RefsReviewed => "refs_reviewed",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "edited" => Self::Edited,
            "confirmed" => Self::Confirmed,
            "archived" => Self::Archived,
            "restored" => Self::Restored,
            "superseded" => Self::Superseded,
            "refs_reviewed" => Self::RefsReviewed,
            _ => Self::Created,
        }
    }
}

/// Referência verificável a um arquivo indexado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRef {
    pub rel_path: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    /// Hash do arquivo indexado quando a referência foi criada ou revisada.
    pub content_hash: String,
}

/// Referência pedida pelo usuário (o hash é preenchido pelo core a partir do índice).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryRefInput {
    pub rel_path: String,
    #[serde(default)]
    #[ts(optional)]
    pub start_line: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub end_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub project_id: i64,
    pub kind: MemoryKind,
    pub status: MemoryStatus,
    pub title: String,
    pub body: String,
    pub origin: MemoryOrigin,
    /// Documento de onde a memória foi importada.
    pub origin_path: Option<String>,
    pub origin_hash: Option<String>,
    /// Memória que esta proposta pretende substituir.
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub confirmed_at: Option<i64>,
    pub refs: Vec<MemoryRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRevision {
    pub revision: i64,
    pub kind: MemoryKind,
    pub status: MemoryStatus,
    pub title: String,
    pub body: String,
    pub refs: Vec<MemoryRef>,
    pub actor: Actor,
    pub change: RevisionChange,
    pub note: Option<String>,
    pub changed_at: i64,
}

/// Estado atual de uma referência em relação ao índice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RefState {
    /// O arquivo indexado tem o mesmo hash de quando a referência foi registrada.
    Unchanged,
    /// O arquivo mudou desde então: a memória pode estar desatualizada.
    Changed,
    /// O arquivo não está mais no índice (e não existe no disco).
    Missing,
    /// O caminho sumiu, mas o mesmo conteúdo está em outro caminho.
    Moved,
    /// Existe no disco, mas não está indexado (ex.: passou a ser ignorado).
    NotIndexed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RefCheck {
    pub reference: MemoryRef,
    pub state: RefState,
    /// Novo caminho provável quando `state = moved`.
    pub moved_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDetail {
    pub memory: Memory,
    pub revisions: Vec<MemoryRevision>,
    pub ref_checks: Vec<RefCheck>,
    /// Título da memória substituída / substituta, para navegação.
    pub supersedes_title: Option<String>,
    pub superseded_by_title: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewMemory {
    pub project_id: i64,
    pub kind: MemoryKind,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub refs: Vec<MemoryRefInput>,
    /// Confirmar já na criação (só para quem revisa).
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryUpdate {
    /// Revisão que o editor viu; diferente da atual ⇒ conflito.
    pub expected_revision: i64,
    #[serde(default)]
    #[ts(optional)]
    pub kind: Option<MemoryKind>,
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub body: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub refs: Option<Vec<MemoryRefInput>>,
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
}

/// Mudança de estado pedida pelo usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    Confirm,
    Archive,
    Restore,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryQuery {
    pub project_id: i64,
    #[serde(default)]
    #[ts(optional)]
    pub statuses: Option<Vec<MemoryStatus>>,
    #[serde(default)]
    #[ts(optional)]
    pub kinds: Option<Vec<MemoryKind>>,
    /// Busca textual (título e corpo).
    #[serde(default)]
    #[ts(optional)]
    pub text: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCounts {
    pub proposed: i64,
    pub confirmed: i64,
    pub archived: i64,
    pub superseded: i64,
}

impl MemoryCounts {
    pub fn total(&self) -> i64 {
        self.proposed + self.confirmed + self.archived + self.superseded
    }
}

// ---- Validação ---------------------------------------------------------------------------

fn check_title(title: &str) -> Result<String> {
    let t = title.trim();
    let n = t.chars().count();
    if !(MIN_TITLE_CHARS..=MAX_TITLE_CHARS).contains(&n) {
        return Err(Error::InvalidInput(format!(
            "o título deve ter entre {MIN_TITLE_CHARS} e {MAX_TITLE_CHARS} caracteres"
        )));
    }
    Ok(t.replace(['\r', '\n'], " "))
}

fn check_body(body: &str) -> Result<String> {
    let b = body.trim().replace("\r\n", "\n");
    if b.chars().count() > MAX_BODY_CHARS {
        return Err(Error::InvalidInput(format!(
            "o texto pode ter no máximo {MAX_BODY_CHARS} caracteres"
        )));
    }
    Ok(b)
}

fn check_note(note: Option<&str>) -> Result<Option<String>> {
    let Some(n) = note.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    if n.chars().count() > MAX_NOTE_CHARS {
        return Err(Error::InvalidInput(format!(
            "a nota pode ter no máximo {MAX_NOTE_CHARS} caracteres"
        )));
    }
    Ok(Some(n.to_owned()))
}

/// Normaliza um caminho relativo vindo de fora (`\` → `/`, sem `./`, sem `..`).
pub fn normalize_rel_path(path: &str) -> Result<String> {
    let p = path.trim().replace('\\', "/");
    let parts: Vec<&str> = p
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|s| *s == ".." || s.contains(':') || s.contains('\0'))
    {
        return Err(Error::InvalidInput(format!("caminho inválido: {path}")));
    }
    Ok(parts.join("/"))
}

fn indexed_hash(conn: &Connection, project_id: i64, rel_path: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT content_hash FROM files
             WHERE project_id = ?1 AND rel_path = ?2 AND status = 'indexed'",
            params![project_id, rel_path],
            |r| r.get(0),
        )
        .optional()?)
}

/// Valida as referências contra o índice e preenche o hash atual de cada arquivo.
fn resolve_refs(
    conn: &Connection,
    project_id: i64,
    inputs: &[MemoryRefInput],
) -> Result<Vec<MemoryRef>> {
    if inputs.len() > MAX_REFS {
        return Err(Error::InvalidInput(format!(
            "no máximo {MAX_REFS} referências por memória"
        )));
    }
    let mut out: Vec<MemoryRef> = Vec::new();
    for input in inputs {
        let rel_path = normalize_rel_path(&input.rel_path)?;
        let (start, end) = match (input.start_line, input.end_line) {
            (None, None) => (None, None),
            (Some(s), None) => (Some(s), Some(s)),
            (None, Some(_)) => {
                return Err(Error::InvalidInput(
                    "informe a linha inicial junto com a final".into(),
                ));
            }
            (Some(s), Some(e)) => (Some(s), Some(e)),
        };
        if let (Some(s), Some(e)) = (start, end)
            && (s < 1 || e < s)
        {
            return Err(Error::InvalidInput(format!(
                "intervalo de linhas inválido em {rel_path}"
            )));
        }
        let hash = indexed_hash(conn, project_id, &rel_path)?.ok_or_else(|| {
            Error::InvalidInput(format!(
                "\"{rel_path}\" não é um arquivo indexado deste projeto"
            ))
        })?;
        let r = MemoryRef {
            rel_path,
            start_line: start,
            end_line: end,
            content_hash: hash,
        };
        if !out
            .iter()
            .any(|o| o.rel_path == r.rel_path && o.start_line == r.start_line)
        {
            out.push(r);
        }
    }
    Ok(out)
}

// ---- Leitura -----------------------------------------------------------------------------

const MEMORY_COLUMNS: &str =
    "id, project_id, kind, status, title, body, origin, origin_path, origin_hash,
     supersedes, superseded_by, revision, created_at, updated_at, confirmed_at";

fn memory_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    let kind: String = r.get(2)?;
    let status: String = r.get(3)?;
    let origin: String = r.get(6)?;
    Ok(Memory {
        id: r.get(0)?,
        project_id: r.get(1)?,
        kind: MemoryKind::parse(&kind).unwrap_or(MemoryKind::Lesson),
        status: MemoryStatus::parse(&status).unwrap_or(MemoryStatus::Proposed),
        title: r.get(4)?,
        body: r.get(5)?,
        origin: MemoryOrigin::parse(&origin),
        origin_path: r.get(7)?,
        origin_hash: r.get(8)?,
        supersedes: r.get(9)?,
        superseded_by: r.get(10)?,
        revision: r.get(11)?,
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
        confirmed_at: r.get(14)?,
        refs: Vec::new(),
    })
}

fn load_refs(conn: &Connection, memory_id: &str) -> Result<Vec<MemoryRef>> {
    let mut stmt = conn.prepare_cached(
        "SELECT rel_path, start_line, end_line, content_hash FROM memory_refs
         WHERE memory_id = ?1 ORDER BY ordinal",
    )?;
    let rows = stmt.query_map([memory_id], |r| {
        Ok(MemoryRef {
            rel_path: r.get(0)?,
            start_line: r.get(1)?,
            end_line: r.get(2)?,
            content_hash: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Resolve um ID completo ou um trecho único (≥ 6 caracteres) do início ou do fim, como na CLI.
/// IDs são UUIDv7: o início é o carimbo de tempo, então o fim (aleatório) é o que distingue.
pub fn resolve_id(conn: &Connection, reference: &str) -> Result<String> {
    let r = reference.trim().to_ascii_lowercase();
    if r.len() < 6 || !r.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(Error::InvalidInput(format!(
            "identificador de memória inválido: \"{reference}\" (use o ID ou 6+ caracteres dele)"
        )));
    }
    let mut stmt = conn
        .prepare("SELECT id FROM memories WHERE id LIKE ?1 || '%' OR id LIKE '%' || ?1 LIMIT 2")?;
    let ids: Vec<String> = stmt
        .query_map([&r], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    match ids.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(Error::NotFound(format!("memória {reference}"))),
        _ => Err(Error::InvalidInput(format!(
            "\"{reference}\" corresponde a mais de uma memória; use mais caracteres"
        ))),
    }
}

pub fn get(conn: &Connection, id: &str) -> Result<Memory> {
    let mut m = conn
        .query_row(
            &format!("SELECT {MEMORY_COLUMNS} FROM memories WHERE id = ?1"),
            [id],
            memory_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("memória {id}")))?;
    m.refs = load_refs(conn, id)?;
    Ok(m)
}

fn title_of(conn: &Connection, id: Option<&str>) -> Result<Option<String>> {
    let Some(id) = id else { return Ok(None) };
    Ok(conn
        .query_row("SELECT title FROM memories WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn revisions(conn: &Connection, id: &str) -> Result<Vec<MemoryRevision>> {
    let mut stmt = conn.prepare(
        "SELECT revision, kind, status, title, body, refs_json, actor, change, note, changed_at
         FROM memory_revisions WHERE memory_id = ?1 ORDER BY revision DESC",
    )?;
    let rows = stmt.query_map([id], |r| {
        let kind: String = r.get(1)?;
        let status: String = r.get(2)?;
        let refs: String = r.get(5)?;
        let actor: String = r.get(6)?;
        let change: String = r.get(7)?;
        Ok(MemoryRevision {
            revision: r.get(0)?,
            kind: MemoryKind::parse(&kind).unwrap_or(MemoryKind::Lesson),
            status: MemoryStatus::parse(&status).unwrap_or(MemoryStatus::Proposed),
            title: r.get(3)?,
            body: r.get(4)?,
            refs: serde_json::from_str(&refs).unwrap_or_default(),
            actor: Actor::parse(&actor),
            change: RevisionChange::parse(&change),
            note: r.get(8)?,
            changed_at: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Estado de cada referência em relação ao índice atual.
pub fn check_refs(
    conn: &Connection,
    project: &crate::projects::Project,
    memory: &Memory,
) -> Result<Vec<RefCheck>> {
    let root = std::path::Path::new(&project.root_path);
    memory
        .refs
        .iter()
        .map(|r| {
            let (state, moved_to) = ref_state(conn, root, memory.project_id, r)?;
            Ok(RefCheck {
                reference: r.clone(),
                state,
                moved_to,
            })
        })
        .collect()
}

/// Compara a referência com o índice: mesmo hash, hash diferente, movido (mesmo conteúdo em
/// outro caminho), fora do índice ou ausente.
pub fn ref_state(
    conn: &Connection,
    root: &std::path::Path,
    project_id: i64,
    r: &MemoryRef,
) -> Result<(RefState, Option<String>)> {
    if let Some(hash) = indexed_hash(conn, project_id, &r.rel_path)? {
        let state = if hash == r.content_hash {
            RefState::Unchanged
        } else {
            RefState::Changed
        };
        return Ok((state, None));
    }
    let moved: Option<String> = conn
        .query_row(
            "SELECT rel_path FROM files WHERE project_id = ?1 AND content_hash = ?2 AND status = 'indexed'
             ORDER BY rel_path LIMIT 1",
            params![project_id, r.content_hash],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(to) = moved {
        return Ok((RefState::Moved, Some(to)));
    }
    let on_disk = crate::scan::resolve_rel(root, &r.rel_path)
        .ok()
        .is_some_and(|p| p.is_file());
    Ok((
        if on_disk {
            RefState::NotIndexed
        } else {
            RefState::Missing
        },
        None,
    ))
}

pub fn detail(
    conn: &Connection,
    project: &crate::projects::Project,
    id: &str,
) -> Result<MemoryDetail> {
    let memory = get(conn, id)?;
    if memory.project_id != project.id {
        return Err(Error::NotFound(format!("memória {id}")));
    }
    Ok(MemoryDetail {
        revisions: revisions(conn, id)?,
        ref_checks: check_refs(conn, project, &memory)?,
        supersedes_title: title_of(conn, memory.supersedes.as_deref())?,
        superseded_by_title: title_of(conn, memory.superseded_by.as_deref())?,
        memory,
    })
}

pub fn counts(conn: &Connection, project_id: i64) -> Result<MemoryCounts> {
    let mut stmt = conn
        .prepare("SELECT status, COUNT(*) FROM memories WHERE project_id = ?1 GROUP BY status")?;
    let mut c = MemoryCounts::default();
    let rows = stmt.query_map([project_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (status, n) = row?;
        match MemoryStatus::parse(&status) {
            Some(MemoryStatus::Proposed) => c.proposed = n,
            Some(MemoryStatus::Confirmed) => c.confirmed = n,
            Some(MemoryStatus::Archived) => c.archived = n,
            Some(MemoryStatus::Superseded) => c.superseded = n,
            None => {}
        }
    }
    Ok(c)
}

/// Lista com filtros; com `text`, ordena por relevância (bm25 sobre título e corpo).
pub fn list(conn: &Connection, q: &MemoryQuery) -> Result<Vec<Memory>> {
    let limit = q.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    if limit == 0 || limit > MAX_LIST_LIMIT {
        return Err(Error::InvalidInput(format!(
            "limit deve estar entre 1 e {MAX_LIST_LIMIT}"
        )));
    }
    let mut sql = format!(
        "SELECT {} FROM memories m",
        MEMORY_COLUMNS
            .split(',')
            .map(|c| format!("m.{}", c.trim()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut args: Vec<rusqlite::types::Value> = vec![q.project_id.into()];
    let text = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let fts = match text {
        Some(t) => {
            if t.chars().count() > MAX_QUERY_CHARS {
                return Err(Error::InvalidInput(format!(
                    "a busca pode ter no máximo {MAX_QUERY_CHARS} caracteres"
                )));
            }
            match fts_expression(&analyze(t).all_terms()) {
                Some(expr) => Some(expr),
                None => return Ok(Vec::new()),
            }
        }
        None => None,
    };
    if let Some(expr) = &fts {
        sql.push_str(
            " JOIN (SELECT memory_id, bm25(memories_fts, 0.0, 3.0, 1.0) AS rank
                    FROM memories_fts WHERE memories_fts MATCH ?2) f ON f.memory_id = m.id",
        );
        args.push(expr.clone().into());
    }
    sql.push_str(" WHERE m.project_id = ?1");
    let mut push_in =
        |column: &str, values: Vec<String>, args: &mut Vec<rusqlite::types::Value>| {
            if values.is_empty() {
                return;
            }
            let start = args.len() + 1;
            let marks: Vec<String> = (start..start + values.len())
                .map(|i| format!("?{i}"))
                .collect();
            sql.push_str(&format!(" AND m.{column} IN ({})", marks.join(", ")));
            args.extend(values.into_iter().map(rusqlite::types::Value::from));
        };
    if let Some(statuses) = &q.statuses {
        push_in(
            "status",
            statuses.iter().map(|s| s.as_str().to_owned()).collect(),
            &mut args,
        );
    }
    if let Some(kinds) = &q.kinds {
        push_in(
            "kind",
            kinds.iter().map(|k| k.as_str().to_owned()).collect(),
            &mut args,
        );
    }
    if fts.is_some() {
        sql.push_str(" ORDER BY f.rank, m.updated_at DESC");
    } else {
        sql.push_str(" ORDER BY m.updated_at DESC, m.id DESC");
    }
    sql.push_str(&format!(" LIMIT {limit}"));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args), memory_from_row)?;
    let mut out: Vec<Memory> = rows.collect::<std::result::Result<_, _>>()?;
    for m in &mut out {
        m.refs = load_refs(conn, &m.id)?;
    }
    Ok(out)
}

/// Memórias **confirmadas** relevantes para uma tarefa: correspondência textual (bm25) e/ou
/// referência a um dos arquivos selecionados. Devolve (memória, pontuação, motivos).
pub fn relevant_confirmed(
    conn: &Connection,
    project_id: i64,
    terms: &[String],
    paths: &[String],
    limit: usize,
) -> Result<Vec<(Memory, f64, Vec<String>)>> {
    let mut scored: HashMap<String, (f64, Vec<String>)> = HashMap::new();
    if let Some(expr) = fts_expression(terms) {
        let mut stmt = conn.prepare(
            "SELECT m.id, bm25(memories_fts, 0.0, 3.0, 1.0) FROM memories_fts
             JOIN memories m ON m.id = memories_fts.memory_id
             WHERE memories_fts MATCH ?1 AND m.project_id = ?2 AND m.status = 'confirmed'
             ORDER BY 2 LIMIT 50",
        )?;
        let rows = stmt.query_map(params![expr, project_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })?;
        let hits: Vec<(String, f64)> = rows.collect::<std::result::Result<_, _>>()?;
        let best = hits.iter().map(|(_, s)| s.abs()).fold(0.0_f64, f64::max);
        for (id, rank) in hits {
            let norm = if best > 0.0 { rank.abs() / best } else { 0.0 };
            scored.insert(
                id,
                (
                    norm,
                    vec!["Texto da memória corresponde à tarefa".to_owned()],
                ),
            );
        }
    }
    if !paths.is_empty() {
        let mut stmt = conn.prepare_cached(
            "SELECT DISTINCT r.memory_id FROM memory_refs r JOIN memories m ON m.id = r.memory_id
             WHERE m.project_id = ?1 AND m.status = 'confirmed' AND r.rel_path = ?2",
        )?;
        for path in paths {
            let ids: Vec<String> = stmt
                .query_map(params![project_id, path], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            for id in ids {
                let entry = scored.entry(id).or_insert((0.0, Vec::new()));
                entry.0 += 0.5;
                let reason = format!("Referencia {path}, incluído no pack");
                if !entry.1.contains(&reason) {
                    entry.1.push(reason);
                }
            }
        }
    }
    let mut ranked: Vec<(String, f64, Vec<String>)> =
        scored.into_iter().map(|(id, (s, r))| (id, s, r)).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(limit);
    ranked
        .into_iter()
        .map(|(id, score, reasons)| Ok((get(conn, &id)?, score, reasons)))
        .collect()
}

// ---- Escrita -----------------------------------------------------------------------------

fn write_refs(tx: &Transaction<'_>, memory_id: &str, refs: &[MemoryRef]) -> Result<()> {
    tx.execute("DELETE FROM memory_refs WHERE memory_id = ?1", [memory_id])?;
    for (i, r) in refs.iter().enumerate() {
        tx.execute(
            "INSERT INTO memory_refs (memory_id, ordinal, rel_path, start_line, end_line, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![memory_id, i as i64, r.rel_path, r.start_line, r.end_line, r.content_hash],
        )?;
    }
    Ok(())
}

fn snapshot(
    tx: &Transaction<'_>,
    m: &Memory,
    actor: Actor,
    change: RevisionChange,
    note: Option<&str>,
    at: i64,
) -> Result<()> {
    let refs = serde_json::to_string(&m.refs)
        .map_err(|e| Error::Internal(format!("serializar referências: {e}")))?;
    tx.execute(
        "INSERT INTO memory_revisions (memory_id, revision, kind, status, title, body, refs_json,
                                       actor, change, note, changed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            m.id,
            m.revision,
            m.kind.as_str(),
            m.status.as_str(),
            m.title,
            m.body,
            refs,
            actor.as_str(),
            change.as_str(),
            note,
            at
        ],
    )?;
    Ok(())
}

fn store(tx: &Transaction<'_>, m: &Memory) -> Result<()> {
    tx.execute(
        "UPDATE memories SET kind = ?2, status = ?3, title = ?4, body = ?5, origin_path = ?6,
                origin_hash = ?7, supersedes = ?8, superseded_by = ?9, revision = ?10,
                updated_at = ?11, confirmed_at = ?12
         WHERE id = ?1",
        params![
            m.id,
            m.kind.as_str(),
            m.status.as_str(),
            m.title,
            m.body,
            m.origin_path,
            m.origin_hash,
            m.supersedes,
            m.superseded_by,
            m.revision,
            m.updated_at,
            m.confirmed_at
        ],
    )?;
    write_refs(tx, &m.id, &m.refs)
}

/// Dados de uma memória criada pelo core (importação), já validados.
pub(crate) struct Draft {
    /// Id já existente em outro banco (memória compartilhada); `None` gera um novo.
    pub id: Option<String>,
    /// Revisão inicial: 1, ou a do arquivo compartilhado (para comparar edições entre bancos).
    pub revision: i64,
    pub project_id: i64,
    pub kind: MemoryKind,
    pub title: String,
    pub body: String,
    pub refs: Vec<MemoryRef>,
    pub origin_path: Option<String>,
    pub origin_hash: Option<String>,
    pub supersedes: Option<String>,
}

pub(crate) fn insert(
    tx: &Transaction<'_>,
    d: Draft,
    actor: Actor,
    confirm: bool,
    note: Option<&str>,
) -> Result<Memory> {
    if confirm && !actor.can_review() {
        return Err(Error::Conflict(
            "somente o usuário pode confirmar memórias; esta fonte só cria propostas".into(),
        ));
    }
    let now = now_ms();
    let m = Memory {
        id: d.id.unwrap_or_else(crate::ops::new_op_id),
        project_id: d.project_id,
        kind: d.kind,
        status: if confirm {
            MemoryStatus::Confirmed
        } else {
            MemoryStatus::Proposed
        },
        title: d.title,
        body: d.body,
        origin: actor.origin(),
        origin_path: d.origin_path,
        origin_hash: d.origin_hash,
        supersedes: d.supersedes,
        superseded_by: None,
        revision: d.revision.max(1),
        created_at: now,
        updated_at: now,
        confirmed_at: confirm.then_some(now),
        refs: d.refs,
    };
    tx.execute(
        "INSERT INTO memories (id, project_id, kind, status, title, body, origin, origin_path,
                               origin_hash, supersedes, revision, created_at, updated_at, confirmed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            m.id,
            m.project_id,
            m.kind.as_str(),
            m.status.as_str(),
            m.title,
            m.body,
            m.origin.as_str(),
            m.origin_path,
            m.origin_hash,
            m.supersedes,
            m.revision,
            m.created_at,
            m.updated_at,
            m.confirmed_at
        ],
    )?;
    write_refs(tx, &m.id, &m.refs)?;
    snapshot(tx, &m, actor, RevisionChange::Created, note, now)?;
    Ok(m)
}

pub fn create(conn: &mut Connection, req: &NewMemory, actor: Actor) -> Result<Memory> {
    let title = check_title(&req.title)?;
    let body = check_body(&req.body)?;
    let note = check_note(req.note.as_deref())?;
    let refs = resolve_refs(conn, req.project_id, &req.refs)?;
    let tx = conn.transaction()?;
    let m = insert(
        &tx,
        Draft {
            id: None,
            revision: 1,
            project_id: req.project_id,
            kind: req.kind,
            title,
            body,
            refs,
            origin_path: None,
            origin_hash: None,
            supersedes: None,
        },
        actor,
        req.confirm,
        note.as_deref(),
    )?;
    tx.commit()?;
    Ok(m)
}

fn check_revision(m: &Memory, expected: i64) -> Result<()> {
    if m.revision != expected {
        return Err(Error::Conflict(format!(
            "a memória foi alterada em outro lugar (revisão atual {}, sua revisão {expected}); recarregue antes de salvar",
            m.revision
        )));
    }
    Ok(())
}

pub fn update(conn: &mut Connection, id: &str, req: &MemoryUpdate, actor: Actor) -> Result<Memory> {
    let mut m = get(conn, id)?;
    check_revision(&m, req.expected_revision)?;
    if !m.status.is_active() {
        return Err(Error::InvalidInput(format!(
            "memórias {} não podem ser editadas; restaure-a antes",
            m.status.label()
        )));
    }
    if m.status == MemoryStatus::Confirmed && !actor.can_review() {
        return Err(Error::Conflict(
            "memórias confirmadas só podem ser alteradas pelo usuário; crie uma proposta".into(),
        ));
    }
    let note = check_note(req.note.as_deref())?;
    let mut changed = false;
    if let Some(kind) = req.kind
        && kind != m.kind
    {
        m.kind = kind;
        changed = true;
    }
    if let Some(title) = &req.title {
        let t = check_title(title)?;
        if t != m.title {
            m.title = t;
            changed = true;
        }
    }
    if let Some(body) = &req.body {
        let b = check_body(body)?;
        if b != m.body {
            m.body = b;
            changed = true;
        }
    }
    if let Some(refs) = &req.refs {
        let r = resolve_refs(conn, m.project_id, refs)?;
        if r != m.refs {
            m.refs = r;
            changed = true;
        }
    }
    if !changed {
        return Ok(m);
    }
    let now = now_ms();
    m.revision += 1;
    m.updated_at = now;
    let tx = conn.transaction()?;
    store(&tx, &m)?;
    snapshot(&tx, &m, actor, RevisionChange::Edited, note.as_deref(), now)?;
    tx.commit()?;
    Ok(m)
}

/// Confirma, arquiva ou restaura. Confirmar uma proposta que substitui outra memória marca a
/// anterior como `superseded` na mesma transação.
pub fn transition(
    conn: &mut Connection,
    id: &str,
    to: Transition,
    expected_revision: Option<i64>,
    note: Option<&str>,
    actor: Actor,
) -> Result<Memory> {
    if !actor.can_review() {
        return Err(Error::Conflict(
            "somente o usuário pode confirmar, arquivar ou restaurar memórias".into(),
        ));
    }
    let note = check_note(note)?;
    let mut m = get(conn, id)?;
    if let Some(expected) = expected_revision {
        check_revision(&m, expected)?;
    }
    let (status, change) = match (to, m.status) {
        (Transition::Confirm, MemoryStatus::Proposed) => {
            (MemoryStatus::Confirmed, RevisionChange::Confirmed)
        }
        (Transition::Archive, MemoryStatus::Proposed | MemoryStatus::Confirmed) => {
            (MemoryStatus::Archived, RevisionChange::Archived)
        }
        (Transition::Restore, MemoryStatus::Archived) => {
            (MemoryStatus::Proposed, RevisionChange::Restored)
        }
        (t, s) => {
            let action = match t {
                Transition::Confirm => "confirmar",
                Transition::Archive => "arquivar",
                Transition::Restore => "restaurar",
            };
            return Err(Error::InvalidInput(format!(
                "não é possível {action} uma memória {}",
                s.label()
            )));
        }
    };
    let now = now_ms();
    let tx = conn.transaction()?;
    if status == MemoryStatus::Confirmed
        && let Some(old_id) = m.supersedes.clone()
    {
        let mut old = get(&tx, &old_id)?;
        if old.status == MemoryStatus::Confirmed {
            old.status = MemoryStatus::Superseded;
            old.superseded_by = Some(m.id.clone());
            old.revision += 1;
            old.updated_at = now;
            store(&tx, &old)?;
            let n = format!("Substituída por \"{}\"", m.title);
            snapshot(&tx, &old, actor, RevisionChange::Superseded, Some(&n), now)?;
        }
    }
    m.status = status;
    m.revision += 1;
    m.updated_at = now;
    if status == MemoryStatus::Confirmed {
        m.confirmed_at = Some(now);
    }
    store(&tx, &m)?;
    snapshot(&tx, &m, actor, change, note.as_deref(), now)?;
    tx.commit()?;
    Ok(m)
}

/// Revisão das referências: fixa o hash atual de cada arquivo e corrige caminhos movidos
/// (mesmo conteúdo em outro caminho). Feita pelo usuário depois de conferir a memória.
pub fn review_refs(
    conn: &mut Connection,
    root: &std::path::Path,
    id: &str,
    expected_revision: i64,
    actor: Actor,
) -> Result<Memory> {
    let mut m = get(conn, id)?;
    check_revision(&m, expected_revision)?;
    if m.status == MemoryStatus::Confirmed && !actor.can_review() {
        return Err(Error::Conflict(
            "memórias confirmadas só podem ser revisadas pelo usuário".into(),
        ));
    }
    let mut refs = Vec::with_capacity(m.refs.len());
    let mut notes = Vec::new();
    for r in &m.refs {
        let (state, moved_to) = ref_state(conn, root, m.project_id, r)?;
        let mut r = r.clone();
        match state {
            RefState::Changed => {
                if let Some(h) = indexed_hash(conn, m.project_id, &r.rel_path)? {
                    r.content_hash = h;
                    notes.push(format!("{} revisado", r.rel_path));
                }
            }
            RefState::Moved => {
                if let Some(to) = moved_to {
                    notes.push(format!("{} → {to}", r.rel_path));
                    r.rel_path = to;
                }
            }
            RefState::Unchanged | RefState::Missing | RefState::NotIndexed => {}
        }
        refs.push(r);
    }
    if refs == m.refs {
        return Ok(m);
    }
    let now = now_ms();
    m.refs = refs;
    m.revision += 1;
    m.updated_at = now;
    let tx = conn.transaction()?;
    store(&tx, &m)?;
    let note = notes.join("; ");
    snapshot(
        &tx,
        &m,
        actor,
        RevisionChange::RefsReviewed,
        Some(&note),
        now,
    )?;
    tx.commit()?;
    Ok(m)
}

/// Tokens normalizados (sem acento, ≥ 3 caracteres, sem stopwords comuns) para comparar textos.
pub(crate) fn token_set(text: &str) -> BTreeSet<String> {
    const STOP: &[&str] = &[
        "que", "para", "com", "por", "uma", "dos", "das", "nos", "nas", "the", "and", "for",
        "with", "este", "esta", "isso", "como", "mais", "não", "nao", "sem", "são", "sao", "ser",
        "deve", "devem", "from", "this", "that", "are", "use", "usar",
    ];
    crate::search::query::normalize(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(w))
        .map(str::to_owned)
        .collect()
}

/// Similaridade de Jaccard entre conjuntos de tokens.
pub(crate) fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    inter / union
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(
            normalize_rel_path("./src\\auth/session.ts").unwrap(),
            "src/auth/session.ts"
        );
        assert_eq!(normalize_rel_path("/docs//a.md").unwrap(), "docs/a.md");
        assert!(normalize_rel_path("../fora").is_err());
        assert!(normalize_rel_path("C:/x").is_err());
        assert!(normalize_rel_path("  ").is_err());
    }

    #[test]
    fn validates_text() {
        assert!(check_title("ab").is_err());
        assert_eq!(
            check_title("  Sessões\nno servidor ").unwrap(),
            "Sessões no servidor"
        );
        assert!(check_body(&"x".repeat(MAX_BODY_CHARS + 1)).is_err());
        assert_eq!(check_note(Some("   ")).unwrap(), None);
    }

    #[test]
    fn only_reviewers_review() {
        assert!(Actor::User.can_review());
        assert!(Actor::Cli.can_review());
        assert!(!Actor::AdrImport.can_review());
        assert!(!Actor::Agent.can_review());
        assert!(!Actor::System.can_review());
    }

    #[test]
    fn similarity_ignores_accents_and_stopwords() {
        let a = token_set("Dinheiro é sempre um inteiro em centavos");
        let b = token_set("dinheiro sempre inteiro em centavos!");
        assert!(jaccard(&a, &b) > 0.99);
        let c = token_set("Sessões no servidor em vez de JWT");
        assert!(jaccard(&a, &c) < 0.1);
    }
}
