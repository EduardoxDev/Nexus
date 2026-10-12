//! Atualidade: detecta contexto desatualizado ou contraditório e mantém alertas estáveis.
//!
//! Verificações (cada alerta traz fontes, motivo, data, recomendação e se requer revisão humana):
//! - **Documentos**: caminhos citados que não existem; identificadores citados que existiam no
//!   código numa verificação anterior e foram removidos (nomes nunca vistos no código não geram
//!   alerta: costumam ser APIs externas).
//! - **Memória**: referências cujo arquivo sumiu, mudou de caminho ou mudou de conteúdo;
//!   decisões importadas cujo documento de origem diz outra coisa agora ou foi marcado como
//!   substituído; memórias duplicadas ou possivelmente contraditórias.
//! - **Índice**: arquivos alterados ou apagados desde a última indexação.
//!
//! "Requer revisão humana" (`needs_review`) marca o que o sistema não consegue decidir sozinho
//! (ex.: um caminho em código inline pode ser só um exemplo; uma mudança no arquivo pode ou não
//! invalidar a memória). Alertas são identificados por `fingerprint`: um alerta dispensado
//! continua dispensado nas próximas verificações, e um alerta que deixa de ocorrer é resolvido.

pub mod doc_refs;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::memory::adr::{self, AdrState};
use crate::memory::{self, Memory, MemoryStatus, RefState};
use crate::projects::Project;
use crate::search::evidence::{self, Freshness};
use crate::search::query::normalize;
use crate::time_util::now_ms;
use doc_refs::{PathRef, RefSyntax};

/// Alertas resolvidos são removidos depois deste prazo.
const RESOLVED_RETENTION_MS: i64 = 30 * 24 * 3_600_000;
const MAX_DOCS: usize = 2_000;
const MAX_SYMBOLS: usize = 5_000;
const MAX_MEMORIES_COMPARED: usize = 2_000;
const MAX_CHANGED_LISTED: usize = 20;
const MAX_SOURCES: usize = 5;
const DEFAULT_LIMIT: u32 = 200;
const MAX_LIMIT: u32 = 1_000;
/// Similaridade (Jaccard) a partir da qual duas memórias são consideradas duplicadas.
const DUPLICATE_THRESHOLD: f64 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    /// Documento cita caminho inexistente.
    MissingPath,
    /// Documento cita identificador que foi removido do código.
    MissingSymbol,
    /// Memória referencia arquivo que sumiu (ou mudou de caminho).
    MemoryRefMissing,
    /// Memória referencia arquivo cujo conteúdo mudou.
    MemoryRefChanged,
    /// O documento de origem de uma memória importada diz outra coisa agora.
    MemorySourceChanged,
    /// O documento de origem foi marcado como substituído/rejeitado, mas a memória segue confirmada.
    SupersededSource,
    /// Memórias duplicadas ou possivelmente contraditórias.
    DuplicateMemory,
    /// Arquivos alterados ou apagados desde a última indexação.
    FilesChanged,
}

impl IssueKind {
    pub const ALL: [Self; 8] = [
        Self::MissingPath,
        Self::MissingSymbol,
        Self::MemoryRefMissing,
        Self::MemoryRefChanged,
        Self::MemorySourceChanged,
        Self::SupersededSource,
        Self::DuplicateMemory,
        Self::FilesChanged,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingPath => "missing_path",
            Self::MissingSymbol => "missing_symbol",
            Self::MemoryRefMissing => "memory_ref_missing",
            Self::MemoryRefChanged => "memory_ref_changed",
            Self::MemorySourceChanged => "memory_source_changed",
            Self::SupersededSource => "superseded_source",
            Self::DuplicateMemory => "duplicate_memory",
            Self::FilesChanged => "files_changed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// Rótulo pt-BR (CLI e exportação).
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingPath => "Caminho inexistente citado",
            Self::MissingSymbol => "Símbolo removido ainda citado",
            Self::MemoryRefMissing => "Referência de memória ausente",
            Self::MemoryRefChanged => "Arquivo de memória alterado",
            Self::MemorySourceChanged => "Origem da memória mudou",
            Self::SupersededSource => "Decisão substituída na origem",
            Self::DuplicateMemory => "Memórias duplicadas ou conflitantes",
            Self::FilesChanged => "Arquivos alterados desde a indexação",
        }
    }

    fn scope(self) -> Scope {
        match self {
            Self::MissingPath | Self::MissingSymbol => Scope::Docs,
            Self::FilesChanged => Scope::Files,
            _ => Scope::Memories,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Docs,
    Memories,
    Files,
}

/// Quais verificações executar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanScope {
    pub docs: bool,
    pub memories: bool,
    pub files: bool,
}

impl ScanScope {
    pub const FULL: Self = Self {
        docs: true,
        memories: true,
        files: true,
    };
    /// Depois de mudanças na memória: só as verificações de memória.
    pub const MEMORIES: Self = Self {
        docs: false,
        memories: true,
        files: false,
    };

    fn includes(self, kind: IssueKind) -> bool {
        match kind.scope() {
            Scope::Docs => self.docs,
            Scope::Memories => self.memories,
            Scope::Files => self.files,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    Dismissed,
    Resolved,
}

impl IssueStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Dismissed => "dismissed",
            Self::Resolved => "resolved",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "dismissed" => Self::Dismissed,
            "resolved" => Self::Resolved,
            _ => Self::Open,
        }
    }
}

/// Onde o problema aparece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueSource {
    pub rel_path: String,
    pub line: Option<i64>,
    /// O que foi citado (caminho ou nome), nunca trechos de código.
    pub cited: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    title: String,
    reason: String,
    recommendation: String,
    #[serde(default)]
    sources: Vec<IssueSource>,
    #[serde(default)]
    suggestion: Option<String>,
    #[serde(default)]
    contradiction: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessIssue {
    pub id: i64,
    pub project_id: i64,
    pub kind: IssueKind,
    pub severity: Severity,
    pub status: IssueStatus,
    /// O sistema não consegue decidir sozinho: uma pessoa precisa avaliar.
    pub needs_review: bool,
    /// É uma contradição (duas fontes dizem coisas diferentes), não só algo desatualizado.
    pub contradiction: bool,
    pub title: String,
    pub reason: String,
    pub recommendation: String,
    pub sources: Vec<IssueSource>,
    /// Correção provável (ex.: novo caminho de um arquivo movido).
    pub suggestion: Option<String>,
    pub subject_path: Option<String>,
    pub memory_id: Option<String>,
    pub related_memory_id: Option<String>,
    pub first_detected_at: i64,
    pub last_detected_at: i64,
    pub resolved_at: Option<i64>,
    pub dismissed_at: Option<i64>,
    pub dismiss_note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueQuery {
    pub project_id: i64,
    /// Padrão: só abertos.
    #[serde(default)]
    #[ts(optional)]
    pub statuses: Option<Vec<IssueStatus>>,
    #[serde(default)]
    #[ts(optional)]
    pub kinds: Option<Vec<IssueKind>>,
    #[serde(default)]
    #[ts(optional)]
    pub memory_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanInfo {
    pub scanned_at: i64,
    pub index_generation: i64,
    pub duration_ms: i64,
    pub docs_checked: i64,
    pub memories_checked: i64,
    pub open_issues: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KindCount {
    pub kind: IssueKind,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub project_id: i64,
    pub info: ScanInfo,
    /// Alertas encontrados nesta verificação (abertos ou dispensados).
    pub detected: u32,
    /// Alertas que não existiam (ou estavam resolvidos) antes.
    pub new_issues: u32,
    pub resolved: u32,
    pub by_kind: Vec<KindCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessSummary {
    pub project_id: i64,
    pub last_scan: Option<ScanInfo>,
    /// O índice mudou depois da última verificação.
    pub scan_outdated: bool,
    pub open: i64,
    pub warnings: i64,
    pub needs_review: i64,
    pub contradictions: i64,
    pub dismissed: i64,
    pub by_kind: Vec<KindCount>,
}

/// Um alerta detectado nesta verificação, antes de ser gravado.
#[derive(Debug, Clone)]
struct Detected {
    kind: IssueKind,
    fingerprint: String,
    severity: Severity,
    needs_review: bool,
    subject_path: Option<String>,
    memory_id: Option<String>,
    related_memory_id: Option<String>,
    detail: Detail,
}

/// Resultado das verificações (somente leitura), aplicado depois numa transação de escrita.
pub struct ScanOutcome {
    project_id: i64,
    index_generation: i64,
    scope: ScanScope,
    detected: Vec<Detected>,
    docs_checked: i64,
    memories_checked: i64,
    seen_symbols: Option<Vec<String>>,
    started: Instant,
}

// ---- Verificações (leitura) --------------------------------------------------------------

struct PathIndex {
    files: HashSet<String>,
    dirs: HashSet<String>,
    by_name: HashMap<String, Vec<String>>,
}

impl PathIndex {
    fn load(conn: &Connection, project_id: i64) -> Result<Self> {
        let mut stmt = conn.prepare("SELECT rel_path FROM files WHERE project_id = ?1")?;
        let paths: Vec<String> = stmt
            .query_map([project_id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let mut idx = Self {
            files: HashSet::with_capacity(paths.len()),
            dirs: HashSet::new(),
            by_name: HashMap::new(),
        };
        for p in paths {
            let mut dir = p.as_str();
            while let Some((d, _)) = dir.rsplit_once('/') {
                if !idx.dirs.insert(d.to_owned()) {
                    break;
                }
                dir = d;
            }
            let name = p.rsplit('/').next().unwrap_or(&p).to_ascii_lowercase();
            idx.by_name.entry(name).or_default().push(p.clone());
            idx.files.insert(p);
        }
        Ok(idx)
    }

    fn exists(&self, root: &Path, rel: &str) -> bool {
        let trimmed = rel.trim_end_matches('/');
        self.files.contains(trimmed)
            || self.dirs.contains(trimmed)
            // Arquivos fora do índice (ignorados pelo .gitignore etc.) ainda existem no disco.
            || crate::scan::resolve_rel(root, trimmed)
                .ok()
                .is_some_and(|p| p.exists())
    }

    fn same_name(&self, rel: &str) -> Vec<String> {
        let name = rel
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(rel)
            .to_ascii_lowercase();
        self.by_name.get(&name).cloned().unwrap_or_default()
    }

    /// Algum arquivo ou pasta termina com `/rel`? Em monorepos, documentos citam caminhos
    /// relativos a um subprojeto (`tests/scale.rs` para `crates/core/tests/scale.rs`).
    fn has_suffix(&self, rel: &str) -> bool {
        let trimmed = rel.trim_end_matches('/');
        if trimmed.is_empty() {
            return false;
        }
        let suffix = format!("/{trimmed}");
        self.files
            .iter()
            .chain(self.dirs.iter())
            .any(|p| p.ends_with(&suffix))
    }
}

fn is_markdown(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    [".md", ".markdown", ".mdx"]
        .iter()
        .any(|e| lower.ends_with(e))
}

/// Históricos citam caminhos antigos por definição.
fn is_history_doc(rel_path: &str) -> bool {
    let name = rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path)
        .to_ascii_lowercase();
    ["changelog", "history", "releases", "release-notes", "news"]
        .iter()
        .any(|p| name.starts_with(p))
}

fn path_ref_exists(idx: &PathIndex, root: &Path, doc_dir: &str, r: &PathRef) -> bool {
    if r.bare_name {
        let name = r.raw.trim_start_matches("./");
        return !idx.same_name(name).is_empty()
            || doc_refs::join_rel(doc_dir, name).is_some_and(|p| idx.exists(root, &p));
    }
    r.resolved.as_deref().is_some_and(|p| idx.exists(root, p))
        || r.alternative
            .as_deref()
            .is_some_and(|p| idx.exists(root, p))
        // Links precisam apontar exatamente; código inline pode ser relativo a um subprojeto.
        || (r.syntax == RefSyntax::Code
            && r.resolved.as_deref().is_some_and(|p| idx.has_suffix(p)))
}

fn symbol_in_code(conn: &Connection, project_id: i64, name: &str) -> Result<bool> {
    // Frase exata: `createSession` vira o token `createsession`; `MAX_RETRIES`, `max retries`.
    let phrase = format!("\"{}\"", name.replace('"', ""));
    let found: Option<i64> = conn
        .prepare_cached(
            "SELECT 1 FROM chunks_fts JOIN chunks c ON c.id = chunks_fts.rowid
             WHERE chunks_fts MATCH ?1 AND c.project_id = ?2
               AND EXISTS (SELECT 1 FROM files f
                           WHERE f.project_id = c.project_id AND f.content_hash = c.content_hash
                             AND f.status = 'indexed' AND f.kind <> 'doc')
             LIMIT 1",
        )?
        .query_row(params![phrase, project_id], |r| r.get(0))
        .optional()?;
    Ok(found.is_some())
}

fn previous_seen_symbols(conn: &Connection, project_id: i64) -> Result<HashSet<String>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT seen_symbols_json FROM freshness_scans WHERE project_id = ?1",
            [project_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(json
        .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
        .unwrap_or_default()
        .into_iter()
        .collect())
}

fn check_docs(
    conn: &Connection,
    project: &Project,
    out: &mut Vec<Detected>,
) -> Result<(i64, Vec<String>)> {
    let root = Path::new(&project.root_path);
    let mut stmt = conn.prepare(
        "SELECT rel_path, content_hash FROM files
         WHERE project_id = ?1 AND status = 'indexed' AND kind = 'doc' ORDER BY rel_path",
    )?;
    let docs: Vec<(String, String)> = stmt
        .query_map([project.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let idx = PathIndex::load(conn, project.id)?;
    let mut checked = 0;
    // nome → (doc, linha) das primeiras citações
    let mut symbols: BTreeMap<String, Vec<IssueSource>> = BTreeMap::new();

    for (doc, hash) in docs
        .into_iter()
        .filter(|(p, _)| is_markdown(p) && !is_history_doc(p))
        .take(MAX_DOCS)
    {
        let v = evidence::verify(root, &doc, &hash)?;
        if matches!(v.freshness, Freshness::Missing | Freshness::Blocked) {
            continue;
        }
        let text = v.lines.join("\n");
        // ADRs substituídos/rejeitados descrevem o passado: suas citações não são cobradas.
        if adr::is_adr_path(&doc) && adr::parse(&text, &doc).state == AdrState::Inactive {
            continue;
        }
        checked += 1;
        let doc_dir = doc.rsplit_once('/').map_or("", |(d, _)| d);
        let refs = doc_refs::extract(&text, &doc);
        let mut reported: HashSet<String> = HashSet::new();
        for r in &refs.paths {
            if path_ref_exists(&idx, root, doc_dir, r) || !reported.insert(r.raw.clone()) {
                continue;
            }
            let candidates: Vec<String> = idx
                .same_name(&r.raw)
                .into_iter()
                .filter(|c| !c.eq_ignore_ascii_case(&doc))
                .collect();
            let suggestion = (candidates.len() == 1).then(|| candidates[0].clone());
            let link = r.syntax == RefSyntax::Link;
            let mut recommendation = if link {
                "Corrija o link ou remova-o do documento.".to_owned()
            } else {
                "Confira se o caminho mudou e atualize o documento; se for apenas um exemplo, dispense o alerta.".to_owned()
            };
            if let Some(s) = &suggestion {
                recommendation = format!("Possivelmente movido para {s}. {recommendation}");
            } else if candidates.len() > 1 {
                recommendation = format!(
                    "Há {} arquivos com o mesmo nome (ex.: {}). {recommendation}",
                    candidates.len(),
                    candidates
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            out.push(Detected {
                kind: IssueKind::MissingPath,
                fingerprint: format!("missing_path|{doc}|{}", r.raw),
                severity: if link {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                // Um link quebrado é inequívoco; código inline pode ser só um exemplo.
                needs_review: !link,
                subject_path: Some(doc.clone()),
                memory_id: None,
                related_memory_id: None,
                detail: Detail {
                    title: format!("{doc} cita um caminho que não existe: {}", r.raw),
                    reason: format!(
                        "{} na linha {} aponta para \"{}\", que não existe no projeto (nem no índice, nem no disco).",
                        if link { "Um link" } else { "Um trecho de código inline" },
                        r.line,
                        r.raw
                    ),
                    recommendation,
                    sources: vec![IssueSource {
                        rel_path: doc.clone(),
                        line: Some(r.line),
                        cited: Some(r.raw.clone()),
                    }],
                    suggestion,
                    contradiction: false,
                },
            });
        }
        for s in refs.symbols {
            let entry = symbols.entry(s.name).or_default();
            if entry.len() < MAX_SOURCES && !entry.iter().any(|e| e.rel_path == doc) {
                entry.push(IssueSource {
                    rel_path: doc.clone(),
                    line: Some(s.line),
                    cited: None,
                });
            }
        }
    }

    let previously_seen = previous_seen_symbols(conn, project.id)?;
    let mut seen = Vec::new();
    for (name, sources) in symbols.into_iter().take(MAX_SYMBOLS) {
        if symbol_in_code(conn, project.id, &name)? {
            seen.push(name);
            continue;
        }
        // Nomes que nunca apareceram no código não geram alerta: nas fixtures e no repositório
        // do NEXUS eram quase sempre APIs externas (`localStorage`, `HttpOnly`). Só a remoção
        // de algo que existia é evidência de documentação desatualizada.
        if !previously_seen.contains(&name) {
            continue;
        }
        // Continua "visto": enquanto for citado e não existir, segue como removido.
        seen.push(name.clone());
        let docs_list: Vec<&str> = sources.iter().map(|s| s.rel_path.as_str()).collect();
        out.push(Detected {
            kind: IssueKind::MissingSymbol,
            fingerprint: format!("missing_symbol|{name}"),
            severity: Severity::Warning,
            // Fato verificado (existia e não existe mais); o novo nome, se houver, o usuário sabe.
            needs_review: false,
            subject_path: sources.first().map(|s| s.rel_path.clone()),
            memory_id: None,
            related_memory_id: None,
            detail: Detail {
                title: format!("`{name}` foi removido do código, mas continua citado"),
                reason: format!(
                    "O identificador existia no código numa verificação anterior e não existe mais em nenhum arquivo de código indexado; ele é citado em {}.",
                    docs_list.join(", ")
                ),
                recommendation:
                    "Atualize a documentação para o nome atual ou remova a citação.".into(),
                sources: sources
                    .into_iter()
                    .map(|s| IssueSource {
                        cited: Some(name.clone()),
                        ..s
                    })
                    .collect(),
                suggestion: None,
                contradiction: false,
            },
        });
    }
    Ok((checked, seen))
}

fn active_memories(conn: &Connection, project_id: i64) -> Result<Vec<Memory>> {
    memory::list(
        conn,
        &memory::MemoryQuery {
            project_id,
            statuses: Some(vec![MemoryStatus::Proposed, MemoryStatus::Confirmed]),
            limit: Some(memory::MAX_LIST_LIMIT),
            ..Default::default()
        },
    )
    .and_then(|mut first| {
        // `list` é limitado; para projetos com muitas memórias, busca o restante em páginas.
        if first.len() < memory::MAX_LIST_LIMIT as usize {
            return Ok(first);
        }
        let mut stmt = conn.prepare(
            "SELECT id FROM memories WHERE project_id = ?1 AND status IN ('proposed', 'confirmed')
             ORDER BY updated_at DESC, id DESC LIMIT -1 OFFSET ?2",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![project_id, first.len() as i64], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        for id in ids {
            first.push(memory::get(conn, &id)?);
        }
        Ok(first)
    })
}

fn quote(title: &str) -> String {
    format!("“{title}”")
}

fn check_memories(conn: &Connection, project: &Project, out: &mut Vec<Detected>) -> Result<i64> {
    let root = Path::new(&project.root_path);
    let memories = active_memories(conn, project.id)?;
    for m in &memories {
        for r in &m.refs {
            let is_origin = m.origin_path.as_deref() == Some(r.rel_path.as_str());
            let (state, moved_to) = memory::ref_state(conn, root, project.id, r)?;
            let source = IssueSource {
                rel_path: r.rel_path.clone(),
                line: r.start_line,
                cited: None,
            };
            match state {
                RefState::Unchanged => {}
                // A mudança no documento de origem é avaliada pelo conteúdo, abaixo.
                RefState::Changed if is_origin => {}
                RefState::Changed => out.push(Detected {
                    kind: IssueKind::MemoryRefChanged,
                    fingerprint: format!(
                        "memory_ref_changed|{}|{}|{}",
                        m.id, r.rel_path, r.content_hash
                    ),
                    severity: Severity::Info,
                    needs_review: true,
                    subject_path: Some(r.rel_path.clone()),
                    memory_id: Some(m.id.clone()),
                    related_memory_id: None,
                    detail: Detail {
                        title: format!("{} mudou depois de ser citado pela memória {}", r.rel_path, quote(&m.title)),
                        reason: format!(
                            "O conteúdo indexado de {} é diferente do registrado quando a referência foi criada ou revisada; a memória ({}) pode não refletir mais o código.",
                            r.rel_path,
                            m.status.label()
                        ),
                        recommendation: "Confira se a memória continua válida e use “Revisar referências” para registrar a versão atual; se não for mais válida, edite ou arquive.".into(),
                        sources: vec![source],
                        suggestion: None,
                        contradiction: false,
                    },
                }),
                RefState::Moved | RefState::Missing | RefState::NotIndexed => {
                    let (reason, recommendation, review) = match state {
                        RefState::Moved => (
                            format!(
                                "{} não existe mais, mas o mesmo conteúdo está em {}.",
                                r.rel_path,
                                moved_to.as_deref().unwrap_or("?")
                            ),
                            "Use “Revisar referências” para apontar para o novo caminho.".to_owned(),
                            false,
                        ),
                        RefState::NotIndexed => (
                            format!("{} existe no disco, mas não está mais no índice (passou a ser ignorado ou deixou de ser legível).", r.rel_path),
                            "Confira as regras de exclusão do projeto ou edite a referência.".to_owned(),
                            true,
                        ),
                        _ => (
                            format!("{} não existe mais no projeto.", r.rel_path),
                            "Edite a memória para apontar para o arquivo correto ou arquive-a se não for mais válida.".to_owned(),
                            true,
                        ),
                    };
                    out.push(Detected {
                        kind: IssueKind::MemoryRefMissing,
                        fingerprint: format!("memory_ref_missing|{}|{}", m.id, r.rel_path),
                        severity: Severity::Warning,
                        needs_review: review,
                        subject_path: Some(r.rel_path.clone()),
                        memory_id: Some(m.id.clone()),
                        related_memory_id: None,
                        detail: Detail {
                            title: format!(
                                "A memória {} referencia {}, que não está mais lá",
                                quote(&m.title),
                                r.rel_path
                            ),
                            reason,
                            recommendation,
                            sources: vec![source],
                            suggestion: moved_to,
                            contradiction: false,
                        },
                    });
                }
            }
        }
        check_origin(conn, root, project.id, m, out)?;
    }
    check_duplicates(&memories, out);
    Ok(memories.len() as i64)
}

/// Memórias importadas: o documento de origem mudou de conteúdo ou foi marcado como substituído?
fn check_origin(
    conn: &Connection,
    root: &Path,
    project_id: i64,
    m: &Memory,
    out: &mut Vec<Detected>,
) -> Result<()> {
    let Some(origin) = m.origin_path.as_deref() else {
        return Ok(());
    };
    let current: Option<String> = conn
        .query_row(
            "SELECT content_hash FROM files WHERE project_id = ?1 AND rel_path = ?2 AND status = 'indexed'",
            params![project_id, origin],
            |r| r.get(0),
        )
        .optional()?;
    let Some(hash) = current else {
        return Ok(()); // ausente: coberto pela verificação de referências
    };
    let Some(parsed) = adr::read_adr(root, origin, &hash)? else {
        return Ok(()); // mudou desde a indexação: espera a próxima indexação
    };
    let source = IssueSource {
        rel_path: origin.to_owned(),
        line: None,
        cited: parsed.state_text.clone(),
    };
    let changed_text = m.origin_hash.as_deref() != Some(hash.as_str())
        && (parsed.title != m.title || parsed.body != m.body);
    if changed_text {
        let pending: Option<String> = conn
            .query_row(
                "SELECT id FROM memories WHERE supersedes = ?1 AND status = 'proposed'
                 ORDER BY created_at DESC LIMIT 1",
                [&m.id],
                |r| r.get(0),
            )
            .optional()?;
        let recommendation = if pending.is_some() {
            "Já existe uma proposta de atualização vinda do documento: revise-a e confirme ou arquive."
        } else {
            "Importe os ADRs novamente para gerar uma proposta de atualização, ou edite a memória."
        };
        out.push(Detected {
            kind: IssueKind::MemorySourceChanged,
            fingerprint: format!("memory_source_changed|{}|{hash}", m.id),
            severity: Severity::Warning,
            needs_review: true,
            subject_path: Some(origin.to_owned()),
            memory_id: Some(m.id.clone()),
            related_memory_id: pending,
            detail: Detail {
                title: format!("{origin} diz outra coisa agora que a memória {}", quote(&m.title)),
                reason: format!(
                    "O texto da decisão em {origin} mudou depois da importação; a memória ({}) ainda tem a versão anterior.",
                    m.status.label()
                ),
                recommendation: recommendation.into(),
                sources: vec![source.clone()],
                suggestion: None,
                contradiction: m.status == MemoryStatus::Confirmed,
            },
        });
    }
    if m.status == MemoryStatus::Confirmed && parsed.state == AdrState::Inactive {
        let state = parsed.state_text.clone().unwrap_or_default();
        out.push(Detected {
            kind: IssueKind::SupersededSource,
            fingerprint: format!("superseded_source|{}|{hash}", m.id),
            severity: Severity::Warning,
            needs_review: true,
            subject_path: Some(origin.to_owned()),
            memory_id: Some(m.id.clone()),
            related_memory_id: None,
            detail: Detail {
                title: format!(
                    "A decisão {} foi marcada como “{state}” na origem",
                    quote(&m.title)
                ),
                reason: format!(
                    "{origin} declara o estado “{state}”, mas a memória continua confirmada."
                ),
                recommendation: "Arquive a memória ou registre a decisão que a substitui.".into(),
                sources: vec![source],
                suggestion: None,
                contradiction: true,
            },
        });
    }
    Ok(())
}

fn check_duplicates(memories: &[Memory], out: &mut Vec<Detected>) {
    let items: Vec<(&Memory, BTreeSet<String>, BTreeSet<String>, String)> = memories
        .iter()
        .take(MAX_MEMORIES_COMPARED)
        .map(|m| {
            let all = memory::token_set(&format!("{} {}", m.title, m.body));
            let title = memory::token_set(&m.title);
            (m, all, title, normalize(m.title.trim()))
        })
        .collect();
    for (i, (a, a_all, a_title, a_norm)) in items.iter().enumerate() {
        for (b, b_all, b_title, b_norm) in &items[i + 1..] {
            if a.supersedes.as_deref() == Some(b.id.as_str())
                || b.supersedes.as_deref() == Some(a.id.as_str())
            {
                continue; // proposta de atualização: esperado
            }
            let same_title =
                a_norm == b_norm || memory::jaccard(a_title, b_title) >= DUPLICATE_THRESHOLD;
            let similar = memory::jaccard(a_all, b_all);
            if !same_title && similar < DUPLICATE_THRESHOLD {
                continue;
            }
            let body_sim =
                memory::jaccard(&memory::token_set(&a.body), &memory::token_set(&b.body));
            let contradiction = same_title
                && a.kind == b.kind
                && a.status == MemoryStatus::Confirmed
                && b.status == MemoryStatus::Confirmed
                && !a.body.is_empty()
                && !b.body.is_empty()
                && body_sim < 0.5;
            let (first, second) = if a.id < b.id { (a, b) } else { (b, a) };
            out.push(Detected {
                kind: IssueKind::DuplicateMemory,
                fingerprint: format!("duplicate_memory|{}|{}", first.id, second.id),
                severity: if contradiction {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                needs_review: true,
                subject_path: None,
                memory_id: Some(first.id.clone()),
                related_memory_id: Some(second.id.clone()),
                detail: Detail {
                    title: if contradiction {
                        format!(
                            "Possível contradição: {} e {}",
                            quote(&first.title),
                            quote(&second.title)
                        )
                    } else {
                        format!(
                            "Memórias possivelmente duplicadas: {} e {}",
                            quote(&first.title),
                            quote(&second.title)
                        )
                    },
                    reason: if contradiction {
                        format!(
                            "As duas estão confirmadas, têm o mesmo assunto e textos diferentes (similaridade do texto {:.0}%).",
                            body_sim * 100.0
                        )
                    } else {
                        format!(
                            "Títulos ou textos muito parecidos (similaridade {:.0}%).",
                            similar.max(body_sim) * 100.0
                        )
                    },
                    recommendation: if contradiction {
                        "Decida qual vale: edite uma delas ou arquive a que não vale mais.".into()
                    } else {
                        "Se dizem a mesma coisa, arquive uma; se são diferentes, ajuste os títulos."
                            .into()
                    },
                    sources: Vec::new(),
                    suggestion: None,
                    contradiction,
                },
            });
        }
    }
}

fn check_files(conn: &Connection, project: &Project, out: &mut Vec<Detected>) -> Result<()> {
    let root = Path::new(&project.root_path);
    let mut stmt = conn.prepare(
        "SELECT rel_path, size, mtime_ns FROM files WHERE project_id = ?1 AND status = 'indexed'
         ORDER BY rel_path",
    )?;
    let rows: Vec<(String, i64, i64)> = stmt
        .query_map([project.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let mut changed = Vec::new();
    let mut deleted = Vec::new();
    for (rel, size, mtime) in rows {
        let Ok(path) = crate::scan::resolve_rel(root, &rel) else {
            continue;
        };
        match std::fs::metadata(&path) {
            Ok(meta) => {
                if i64::try_from(meta.len()).unwrap_or(i64::MAX) != size
                    || crate::scan::mtime_ns(&meta) != mtime
                {
                    changed.push(rel);
                }
            }
            Err(_) => deleted.push(rel),
        }
    }
    let total = changed.len() + deleted.len();
    if total == 0 {
        return Ok(());
    }
    let mut sources: Vec<IssueSource> = deleted
        .iter()
        .map(|p| IssueSource {
            rel_path: p.clone(),
            line: None,
            cited: Some("apagado".into()),
        })
        .chain(changed.iter().map(|p| IssueSource {
            rel_path: p.clone(),
            line: None,
            cited: Some("alterado".into()),
        }))
        .collect();
    sources.truncate(MAX_CHANGED_LISTED);
    out.push(Detected {
        kind: IssueKind::FilesChanged,
        fingerprint: format!("files_changed|{}", project.index_generation),
        severity: Severity::Info,
        needs_review: false,
        subject_path: None,
        memory_id: None,
        related_memory_id: None,
        detail: Detail {
            title: format!("{total} arquivo(s) mudaram desde a última indexação"),
            reason: format!(
                "{} alterado(s) e {} apagado(s) no disco depois da indexação (geração {}). Buscas e packs podem mostrar linhas desatualizadas.",
                changed.len(),
                deleted.len(),
                project.index_generation
            ),
            recommendation: "Atualize o índice do projeto. Com “Acompanhar alterações” ligado, isso acontece automaticamente.".into(),
            sources,
            suggestion: None,
            contradiction: false,
        },
    });
    Ok(())
}

/// Executa as verificações (somente leitura). O resultado é gravado com [`apply`].
pub fn scan(conn: &Connection, project: &Project, scope: ScanScope) -> Result<ScanOutcome> {
    let started = Instant::now();
    let mut detected = Vec::new();
    let (docs_checked, seen_symbols) = if scope.docs {
        let (n, seen) = check_docs(conn, project, &mut detected)?;
        (n, Some(seen))
    } else {
        (0, None)
    };
    let memories_checked = if scope.memories {
        check_memories(conn, project, &mut detected)?
    } else {
        0
    };
    if scope.files {
        check_files(conn, project, &mut detected)?;
    }
    // Uma impressão digital por alerta (ex.: o mesmo link citado duas vezes).
    let mut seen = HashSet::new();
    detected.retain(|d| seen.insert(d.fingerprint.clone()));
    Ok(ScanOutcome {
        project_id: project.id,
        index_generation: project.index_generation,
        scope,
        detected,
        docs_checked,
        memories_checked,
        seen_symbols,
        started,
    })
}

// ---- Gravação ----------------------------------------------------------------------------

/// Grava o resultado: cria/atualiza alertas, reabre os que voltaram, resolve os que sumiram.
pub fn apply(conn: &mut Connection, outcome: ScanOutcome) -> Result<ScanReport> {
    let now = now_ms();
    let tx = conn.transaction()?;
    let mut new_issues = 0;
    let mut by_kind: BTreeMap<IssueKind, i64> = BTreeMap::new();
    let fingerprints: HashSet<&str> = outcome
        .detected
        .iter()
        .map(|d| d.fingerprint.as_str())
        .collect();
    for d in &outcome.detected {
        *by_kind.entry(d.kind).or_default() += 1;
        let previous: Option<String> = tx
            .query_row(
                "SELECT status FROM freshness_issues WHERE project_id = ?1 AND fingerprint = ?2",
                params![outcome.project_id, d.fingerprint],
                |r| r.get(0),
            )
            .optional()?;
        if previous.as_deref().is_none_or(|s| s == "resolved") {
            new_issues += 1;
        }
        let detail = serde_json::to_string(&d.detail)
            .map_err(|e| Error::Internal(format!("serializar alerta: {e}")))?;
        tx.execute(
            "INSERT INTO freshness_issues (project_id, kind, fingerprint, severity, status, needs_review,
                 subject_path, memory_id, related_memory_id, detail_json, first_detected_at, last_detected_at)
             VALUES (?1, ?2, ?3, ?4, 'open', ?5, ?6, ?7, ?8, ?9, ?10, ?10)
             ON CONFLICT (project_id, fingerprint) DO UPDATE SET
                 kind = excluded.kind,
                 severity = excluded.severity,
                 needs_review = excluded.needs_review,
                 subject_path = excluded.subject_path,
                 memory_id = excluded.memory_id,
                 related_memory_id = excluded.related_memory_id,
                 detail_json = excluded.detail_json,
                 last_detected_at = excluded.last_detected_at,
                 first_detected_at = CASE WHEN freshness_issues.status = 'resolved'
                                          THEN excluded.first_detected_at
                                          ELSE freshness_issues.first_detected_at END,
                 status = CASE WHEN freshness_issues.status = 'resolved' THEN 'open'
                               ELSE freshness_issues.status END,
                 resolved_at = CASE WHEN freshness_issues.status = 'resolved' THEN NULL
                                    ELSE freshness_issues.resolved_at END",
            params![
                outcome.project_id,
                d.kind.as_str(),
                d.fingerprint,
                d.severity.as_str(),
                d.needs_review,
                d.subject_path,
                d.memory_id,
                d.related_memory_id,
                detail,
                now
            ],
        )?;
    }

    // Abertos/dispensados do mesmo escopo que não apareceram nesta verificação: resolvidos.
    let mut stale = Vec::new();
    {
        let mut stmt = tx.prepare(
            "SELECT id, kind, fingerprint FROM freshness_issues
             WHERE project_id = ?1 AND status IN ('open', 'dismissed')",
        )?;
        let rows = stmt.query_map([outcome.project_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (id, kind, fp) = row?;
            let in_scope = IssueKind::parse(&kind).is_some_and(|k| outcome.scope.includes(k));
            if in_scope && !fingerprints.contains(fp.as_str()) {
                stale.push(id);
            }
        }
    }
    for id in &stale {
        tx.execute(
            "UPDATE freshness_issues SET status = 'resolved', resolved_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
    }
    tx.execute(
        "DELETE FROM freshness_issues WHERE project_id = ?1 AND status = 'resolved' AND resolved_at < ?2",
        params![outcome.project_id, now - RESOLVED_RETENTION_MS],
    )?;
    let open: i64 = tx.query_row(
        "SELECT COUNT(*) FROM freshness_issues WHERE project_id = ?1 AND status = 'open'",
        [outcome.project_id],
        |r| r.get(0),
    )?;
    let duration_ms = i64::try_from(outcome.started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let info = ScanInfo {
        scanned_at: now,
        index_generation: outcome.index_generation,
        duration_ms,
        docs_checked: outcome.docs_checked,
        memories_checked: outcome.memories_checked,
        open_issues: open,
    };
    // Só a verificação completa conta como "última verificação".
    if outcome.scope == ScanScope::FULL {
        let seen = serde_json::to_string(&outcome.seen_symbols.unwrap_or_default())
            .map_err(|e| Error::Internal(format!("serializar símbolos: {e}")))?;
        tx.execute(
            "INSERT INTO freshness_scans (project_id, scanned_at, index_generation, duration_ms,
                                          docs_checked, memories_checked, open_issues, seen_symbols_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (project_id) DO UPDATE SET
                 scanned_at = excluded.scanned_at, index_generation = excluded.index_generation,
                 duration_ms = excluded.duration_ms, docs_checked = excluded.docs_checked,
                 memories_checked = excluded.memories_checked, open_issues = excluded.open_issues,
                 seen_symbols_json = excluded.seen_symbols_json",
            params![
                outcome.project_id,
                info.scanned_at,
                info.index_generation,
                info.duration_ms,
                info.docs_checked,
                info.memories_checked,
                info.open_issues,
                seen
            ],
        )?;
    }
    tx.commit()?;
    Ok(ScanReport {
        project_id: outcome.project_id,
        info,
        detected: u32::try_from(outcome.detected.len()).unwrap_or(u32::MAX),
        new_issues,
        resolved: u32::try_from(stale.len()).unwrap_or(u32::MAX),
        by_kind: by_kind
            .into_iter()
            .map(|(kind, count)| KindCount { kind, count })
            .collect(),
    })
}

// ---- Consulta e ações --------------------------------------------------------------------

fn issue_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<FreshnessIssue> {
    let kind: String = r.get(2)?;
    let severity: String = r.get(3)?;
    let status: String = r.get(4)?;
    let detail: String = r.get(10)?;
    let d: Detail = serde_json::from_str(&detail).unwrap_or(Detail {
        title: kind.clone(),
        reason: String::new(),
        recommendation: String::new(),
        sources: Vec::new(),
        suggestion: None,
        contradiction: false,
    });
    Ok(FreshnessIssue {
        id: r.get(0)?,
        project_id: r.get(1)?,
        kind: IssueKind::parse(&kind).unwrap_or(IssueKind::FilesChanged),
        severity: if severity == "warning" {
            Severity::Warning
        } else {
            Severity::Info
        },
        status: IssueStatus::parse(&status),
        needs_review: r.get(5)?,
        contradiction: d.contradiction,
        title: d.title,
        reason: d.reason,
        recommendation: d.recommendation,
        sources: d.sources,
        suggestion: d.suggestion,
        subject_path: r.get(6)?,
        memory_id: r.get(7)?,
        related_memory_id: r.get(8)?,
        first_detected_at: r.get(11)?,
        last_detected_at: r.get(12)?,
        resolved_at: r.get(13)?,
        dismissed_at: r.get(14)?,
        dismiss_note: r.get(15)?,
    })
}

const ISSUE_COLUMNS: &str = "id, project_id, kind, severity, status, needs_review, subject_path,
     memory_id, related_memory_id, fingerprint, detail_json, first_detected_at, last_detected_at,
     resolved_at, dismissed_at, dismiss_note";

/// Ordem: avisos antes de informações, contradições primeiro, depois os mais recentes.
fn sort_issues(issues: &mut [FreshnessIssue]) {
    issues.sort_by(|a, b| {
        (b.severity == Severity::Warning)
            .cmp(&(a.severity == Severity::Warning))
            .then(b.contradiction.cmp(&a.contradiction))
            .then(b.last_detected_at.cmp(&a.last_detected_at))
            .then(a.id.cmp(&b.id))
    });
}

pub fn list(conn: &Connection, q: &IssueQuery) -> Result<Vec<FreshnessIssue>> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(Error::InvalidInput(format!(
            "limit deve estar entre 1 e {MAX_LIMIT}"
        )));
    }
    let statuses = q
        .statuses
        .clone()
        .unwrap_or_else(|| vec![IssueStatus::Open]);
    let mut stmt = conn.prepare(&format!(
        "SELECT {ISSUE_COLUMNS} FROM freshness_issues WHERE project_id = ?1
         ORDER BY last_detected_at DESC, id"
    ))?;
    let rows = stmt.query_map([q.project_id], issue_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        let issue = row?;
        if !statuses.contains(&issue.status) {
            continue;
        }
        if q.kinds.as_ref().is_some_and(|k| !k.contains(&issue.kind)) {
            continue;
        }
        if let Some(mid) = &q.memory_id
            && issue.memory_id.as_ref() != Some(mid)
            && issue.related_memory_id.as_ref() != Some(mid)
        {
            continue;
        }
        out.push(issue);
    }
    sort_issues(&mut out);
    out.truncate(limit as usize);
    Ok(out)
}

/// Alertas abertos ligados a arquivos ou memórias (usados nos Context Packs).
pub fn related(
    conn: &Connection,
    project_id: i64,
    paths: &[String],
    memory_ids: &[String],
) -> Result<Vec<FreshnessIssue>> {
    let paths: HashSet<&str> = paths.iter().map(String::as_str).collect();
    let ids: HashSet<&str> = memory_ids.iter().map(String::as_str).collect();
    let mut out: Vec<FreshnessIssue> = list(
        conn,
        &IssueQuery {
            project_id,
            limit: Some(MAX_LIMIT),
            ..Default::default()
        },
    )?
    .into_iter()
    .filter(|i| {
        i.kind != IssueKind::FilesChanged
            && (i.subject_path.as_deref().is_some_and(|p| paths.contains(p))
                || i.sources
                    .iter()
                    .any(|s| paths.contains(s.rel_path.as_str()))
                || i.memory_id.as_deref().is_some_and(|m| ids.contains(m))
                || i.related_memory_id
                    .as_deref()
                    .is_some_and(|m| ids.contains(m)))
    })
    .collect();
    sort_issues(&mut out);
    out.truncate(20);
    Ok(out)
}

pub fn get(conn: &Connection, id: i64) -> Result<FreshnessIssue> {
    conn.query_row(
        &format!("SELECT {ISSUE_COLUMNS} FROM freshness_issues WHERE id = ?1"),
        [id],
        issue_from_row,
    )
    .optional()?
    .ok_or_else(|| Error::NotFound(format!("alerta {id}")))
}

/// Dispensa (ou reabre) um alerta. Dispensado continua dispensado enquanto o problema persistir.
pub fn set_dismissed(
    conn: &Connection,
    id: i64,
    dismissed: bool,
    note: Option<&str>,
) -> Result<FreshnessIssue> {
    let issue = get(conn, id)?;
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    if note.is_some_and(|n| n.chars().count() > 1_000) {
        return Err(Error::InvalidInput(
            "a nota pode ter no máximo 1000 caracteres".into(),
        ));
    }
    match (dismissed, issue.status) {
        (true, IssueStatus::Open) => {
            conn.execute(
                "UPDATE freshness_issues SET status = 'dismissed', dismissed_at = ?2, dismiss_note = ?3
                 WHERE id = ?1",
                params![id, now_ms(), note],
            )?;
        }
        (false, IssueStatus::Dismissed) => {
            conn.execute(
                "UPDATE freshness_issues SET status = 'open', dismissed_at = NULL, dismiss_note = NULL
                 WHERE id = ?1",
                [id],
            )?;
        }
        (true, s) | (false, s) => {
            return Err(Error::InvalidInput(format!(
                "o alerta está {} e não pode ser {}",
                match s {
                    IssueStatus::Open => "aberto",
                    IssueStatus::Dismissed => "dispensado",
                    IssueStatus::Resolved => "resolvido",
                },
                if dismissed { "dispensado" } else { "reaberto" }
            )));
        }
    }
    get(conn, id)
}

pub fn last_scan(conn: &Connection, project_id: i64) -> Result<Option<ScanInfo>> {
    Ok(conn
        .query_row(
            "SELECT scanned_at, index_generation, duration_ms, docs_checked, memories_checked, open_issues
             FROM freshness_scans WHERE project_id = ?1",
            [project_id],
            |r| {
                Ok(ScanInfo {
                    scanned_at: r.get(0)?,
                    index_generation: r.get(1)?,
                    duration_ms: r.get(2)?,
                    docs_checked: r.get(3)?,
                    memories_checked: r.get(4)?,
                    open_issues: r.get(5)?,
                })
            },
        )
        .optional()?)
}

pub fn summary(conn: &Connection, project: &Project) -> Result<FreshnessSummary> {
    let last = last_scan(conn, project.id)?;
    let open = list(
        conn,
        &IssueQuery {
            project_id: project.id,
            limit: Some(MAX_LIMIT),
            ..Default::default()
        },
    )?;
    let dismissed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM freshness_issues WHERE project_id = ?1 AND status = 'dismissed'",
        [project.id],
        |r| r.get(0),
    )?;
    let mut by_kind: BTreeMap<IssueKind, i64> = BTreeMap::new();
    for i in &open {
        *by_kind.entry(i.kind).or_default() += 1;
    }
    Ok(FreshnessSummary {
        project_id: project.id,
        scan_outdated: last
            .as_ref()
            .is_none_or(|s| s.index_generation < project.index_generation),
        last_scan: last,
        open: open.len() as i64,
        warnings: open
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count() as i64,
        needs_review: open.iter().filter(|i| i.needs_review).count() as i64,
        contradictions: open.iter().filter(|i| i.contradiction).count() as i64,
        dismissed,
        by_kind: by_kind
            .into_iter()
            .map(|(kind, count)| KindCount { kind, count })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_and_scope() {
        for k in IssueKind::ALL {
            assert_eq!(IssueKind::parse(k.as_str()), Some(k));
        }
        assert!(ScanScope::MEMORIES.includes(IssueKind::DuplicateMemory));
        assert!(!ScanScope::MEMORIES.includes(IssueKind::MissingPath));
        assert!(!ScanScope::MEMORIES.includes(IssueKind::FilesChanged));
    }

    #[test]
    fn history_docs_are_skipped() {
        assert!(is_history_doc("CHANGELOG.md"));
        assert!(is_history_doc("docs/release-notes-2026.md"));
        assert!(!is_history_doc("docs/architecture.md"));
    }
}
