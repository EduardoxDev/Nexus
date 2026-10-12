//! Compilador de Context Packs: a partir de uma tarefa, reúne arquivos relevantes, testes,
//! decisões, convenções e manifestos, aplica um orçamento de tokens e registra as fontes.
//!
//! O plano persistido contém apenas referências (caminho, linhas, hash). O texto é lido do disco
//! ao abrir/exportar e comparado com o hash da criação: nada é inventado e mudanças são sinalizadas.

pub mod export;

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::classify::FileKind;
use crate::error::{Error, Result};
use crate::freshness::{self, FreshnessIssue};
use crate::memory::{self, Memory, MemoryKind, MemoryStatus};
use crate::projects::Project;
use crate::search::evidence::{self, Freshness};
use crate::search::query::{QueryAnalysis, normalize};
use crate::search::{self, Reason, ReasonKind, SearchHit, SearchMode, SearchRequest};
use crate::time_util::now_ms;

/// 2: memórias confirmadas no plano; alertas de atualidade e contradições no pack materializado.
pub const SCHEMA_VERSION: u32 = 2;
/// Memórias confirmadas no máximo por pack.
const MAX_MEMORIES: usize = 6;
/// Fração do orçamento reservada, no máximo, para memórias.
const MEMORY_BUDGET_SHARE: u32 = 4;
const DEFAULT_BUDGET: u32 = 8_000;
const MIN_BUDGET: u32 = 1_000;
const MAX_BUDGET: u32 = 50_000;
const DEFAULT_MAX_FILES: u32 = 12;
const MAX_FILES: u32 = 40;
const MAX_TASK_CHARS: usize = 2_000;
/// Linhas máximas por trecho incluído no pack.
const EXCERPT_LINES: i64 = 50;
const PACKS_KEPT_PER_PROJECT: i64 = 100;
/// Máximo de documentos genéricos (não decisões/convenções) por pack.
const MAX_GENERIC_DOCS: usize = 2;
/// Dependências diretas (pelo grafo) acrescentadas no máximo por pack.
const MAX_GRAPH_DEPENDENCIES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// Arquivo central para a tarefa.
    Primary,
    /// Relevante, mas com pontuação menor.
    Related,
    /// Teste relacionado.
    Test,
    /// Registro de decisão (ADR) ou documentação que decide algo.
    Decision,
    /// Documentação geral.
    Doc,
    /// Guia de contribuição/convenções.
    Convention,
    /// Manifesto de dependências.
    Dependency,
    /// Configuração.
    Config,
}

impl SourceRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Related => "related",
            Self::Test => "test",
            Self::Decision => "decision",
            Self::Doc => "doc",
            Self::Convention => "convention",
            Self::Dependency => "dependency",
            Self::Config => "config",
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackRequest {
    pub project_id: i64,
    pub task: String,
    /// Orçamento estimado de tokens (1000..=50000; padrão 8000).
    #[serde(default)]
    #[ts(optional)]
    pub token_budget: Option<u32>,
    /// Máximo de arquivos (1..=40; padrão 12).
    #[serde(default)]
    #[ts(optional)]
    pub max_files: Option<u32>,
}

/// Uma fonte do pack: referência verificável, sem texto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackSource {
    pub rel_path: String,
    pub role: SourceRole,
    pub start_line: i64,
    pub end_line: i64,
    /// Hash do arquivo no momento da criação do pack.
    pub content_hash: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub language: Option<String>,
    pub kind: FileKind,
    pub estimated_tokens: u32,
}

/// Plano persistido do pack (sem texto de código).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackPlan {
    pub schema_version: u32,
    pub id: String,
    pub project_id: i64,
    pub project_name: String,
    pub task: String,
    pub created_at: i64,
    pub index_generation: i64,
    pub head_ref: Option<String>,
    pub head_commit: Option<String>,
    pub analysis: QueryAnalysisSnapshot,
    pub summary: String,
    pub sources: Vec<PackSource>,
    /// Caminhos relevantes que ficaram de fora pelo orçamento.
    pub omitted: Vec<String>,
    pub risks: Vec<String>,
    pub limitations: Vec<String>,
    pub token_budget: u32,
    pub estimated_tokens: u32,
    /// Memórias confirmadas incluídas (referência: id + revisão). Packs v1 não têm.
    #[serde(default)]
    pub memories: Vec<PackMemoryRef>,
}

/// Memória confirmada incluída no pack, por referência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackMemoryRef {
    pub memory_id: String,
    /// Revisão incluída; outra revisão ao reabrir ⇒ a memória mudou desde a criação do pack.
    pub revision: i64,
    pub kind: MemoryKind,
    pub title: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub estimated_tokens: u32,
}

/// Estado de uma memória do pack ao reabrir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PackMemoryState {
    /// Mesma revisão e ainda confirmada.
    Unchanged,
    /// Editada depois da criação do pack (o texto mostrado é o atual).
    Revised,
    /// Não está mais confirmada (arquivada ou substituída).
    Withdrawn,
    /// Não existe mais.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackMemory {
    pub reference: PackMemoryRef,
    pub state: PackMemoryState,
    /// Versão atual (texto lido do banco agora).
    pub current: Option<Memory>,
}

/// Versão serializável e estável da análise da tarefa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QueryAnalysisSnapshot {
    pub terms: Vec<String>,
    pub expanded_terms: Vec<String>,
    pub intents: Vec<String>,
    pub mentioned_paths: Vec<String>,
}

impl From<&QueryAnalysis> for QueryAnalysisSnapshot {
    fn from(a: &QueryAnalysis) -> Self {
        let terms = a.original_terms();
        let expanded = a
            .all_terms()
            .into_iter()
            .filter(|t| !terms.contains(t))
            .collect();
        Self {
            terms,
            expanded_terms: expanded,
            intents: a
                .intents
                .iter()
                .map(|i| {
                    serde_json::to_value(i)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default()
                })
                .collect(),
            mentioned_paths: a.mentioned_paths.clone(),
        }
    }
}

/// Trecho materializado (lido do disco agora).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackExcerpt {
    pub start_line: i64,
    pub end_line: i64,
    pub text: String,
    /// Em relação ao momento da criação do pack.
    pub freshness: Freshness,
}

/// Pack pronto para exibir/exportar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextPack {
    pub plan: ContextPackPlan,
    /// Mesma ordem de `plan.sources`.
    pub excerpts: Vec<PackExcerpt>,
    pub stale_sources: u32,
    /// Mesma ordem de `plan.memories`.
    pub memories: Vec<PackMemory>,
    /// Alertas de atualidade abertos ligados às fontes ou memórias do pack (inclui contradições).
    pub issues: Vec<FreshnessIssue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackSummary {
    pub id: String,
    pub project_id: i64,
    pub task: String,
    pub created_at: i64,
    pub index_generation: i64,
    pub file_count: i64,
    pub estimated_tokens: i64,
    pub token_budget: i64,
}

/// Estimativa grosseira (caracteres ÷ 4), sempre rotulada como estimativa.
pub fn estimate_tokens(text: &str) -> u32 {
    u32::try_from(text.chars().count().div_ceil(4)).unwrap_or(u32::MAX)
}

fn validate(req: &ContextPackRequest) -> Result<(u32, u32)> {
    let chars = req.task.trim().chars().count();
    if chars < 3 {
        return Err(Error::InvalidInput(
            "descreva a tarefa (mínimo de 3 caracteres)".into(),
        ));
    }
    if chars > MAX_TASK_CHARS {
        return Err(Error::InvalidInput(format!(
            "a tarefa pode ter no máximo {MAX_TASK_CHARS} caracteres"
        )));
    }
    let budget = req.token_budget.unwrap_or(DEFAULT_BUDGET);
    if !(MIN_BUDGET..=MAX_BUDGET).contains(&budget) {
        return Err(Error::InvalidInput(format!(
            "tokenBudget deve estar entre {MIN_BUDGET} e {MAX_BUDGET}"
        )));
    }
    let max_files = req.max_files.unwrap_or(DEFAULT_MAX_FILES);
    if max_files == 0 || max_files > MAX_FILES {
        return Err(Error::InvalidInput(format!(
            "maxFiles deve estar entre 1 e {MAX_FILES}"
        )));
    }
    Ok((budget, max_files))
}

fn is_decision_path(lower: &str) -> bool {
    lower.contains("decision") || lower.contains("/adr") || lower.starts_with("adr")
}

fn is_convention_path(lower: &str) -> bool {
    let name = lower.rsplit('/').next().unwrap_or(lower);
    name.starts_with("contributing")
        || name.contains("convention")
        || name.contains("style")
        || name.contains("guideline")
        || name.starts_with("coding")
}

fn role_for(hit: &SearchHit) -> SourceRole {
    let lower = hit.rel_path.to_ascii_lowercase();
    match hit.kind {
        FileKind::Test => SourceRole::Test,
        FileKind::Doc if is_decision_path(&lower) => SourceRole::Decision,
        FileKind::Doc if is_convention_path(&lower) => SourceRole::Convention,
        FileKind::Doc => SourceRole::Doc,
        FileKind::Manifest | FileKind::Lockfile => SourceRole::Dependency,
        FileKind::Config | FileKind::Ci => SourceRole::Config,
        _ => SourceRole::Primary,
    }
}

fn reason_labels(reasons: &[Reason]) -> Vec<String> {
    reasons
        .iter()
        .filter(|r| r.weight > 0.0 || r.kind == ReasonKind::FileKind)
        .map(|r| r.label.clone())
        .collect()
}

struct FileInfo {
    hash: String,
    kind: FileKind,
    language: Option<String>,
}

fn file_info(conn: &Connection, project_id: i64, rel_path: &str) -> Result<Option<FileInfo>> {
    Ok(conn
        .query_row(
            "SELECT content_hash, kind, language FROM files
             WHERE project_id = ?1 AND rel_path = ?2 AND status = 'indexed'",
            params![project_id, rel_path],
            |r| {
                Ok(FileInfo {
                    hash: r.get(0)?,
                    kind: FileKind::parse(&r.get::<_, String>(1)?),
                    language: r.get(2)?,
                })
            },
        )
        .optional()?)
}

pub(crate) fn stem(rel_path: &str) -> String {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let base = name.split('.').next().unwrap_or(name);
    normalize(base.trim_start_matches("test_").trim_end_matches("_test"))
}

/// Testes cujo nome corresponde ao arquivo (`payment.ts` ↔ `payment.test.ts`, `service.py` ↔ `test_service.py`).
pub(crate) fn test_counterparts(
    conn: &Connection,
    project_id: i64,
    rel_path: &str,
) -> Result<Vec<String>> {
    let s = stem(rel_path);
    if s.chars().count() < 4 || matches!(s.as_str(), "index" | "main" | "init" | "lib" | "mod") {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT rel_path FROM files WHERE project_id = ?1 AND status = 'indexed' AND kind = 'test'
         ORDER BY rel_path",
    )?;
    let tests: Vec<String> = stmt
        .query_map([project_id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    Ok(tests
        .into_iter()
        .filter(|t| {
            let ts = stem(t);
            ts == s || ts.contains(&s) || (ts.len() >= 4 && s.contains(&ts))
        })
        .collect())
}

/// Manifesto mais próximo de cada arquivo (mesma pasta ou ancestral), mais o da raiz.
pub(crate) fn nearest_manifests(
    conn: &Connection,
    project_id: i64,
    paths: &[String],
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path FROM files WHERE project_id = ?1 AND status = 'indexed' AND kind = 'manifest'",
    )?;
    let manifests: Vec<String> = stmt
        .query_map([project_id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let dir_of = |p: &str| {
        p.rsplit_once('/')
            .map_or(String::new(), |(d, _)| d.to_owned())
    };
    let mut found = BTreeSet::new();
    for path in paths {
        let mut dir = dir_of(path);
        loop {
            if let Some(m) = manifests.iter().find(|m| dir_of(m) == dir) {
                found.insert(m.clone());
                break;
            }
            if dir.is_empty() {
                break;
            }
            dir = dir_of(&dir);
        }
    }
    Ok(found.into_iter().take(2).collect())
}

fn sensitive_in_dirs(conn: &Connection, project_id: i64, dirs: &HashSet<String>) -> Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT rel_path FROM files WHERE project_id = ?1 AND status = 'skipped_sensitive'",
    )?;
    let paths: Vec<String> = stmt
        .query_map([project_id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    Ok(paths
        .iter()
        .filter(|p| {
            let d = p.rsplit_once('/').map_or("", |(d, _)| d);
            dirs.contains(d)
        })
        .count() as i64)
}

struct Draft {
    rel_path: String,
    role: SourceRole,
    start: i64,
    end: i64,
    score: f64,
    reasons: Vec<String>,
}

/// Ordem das seções no pack. A intenção da tarefa promove o que foi pedido explicitamente.
fn role_priority(role: SourceRole, analysis: &QueryAnalysis) -> u8 {
    use crate::search::query::Intent;
    let asked = match role {
        SourceRole::Test => analysis.has(Intent::Tests),
        // Documentação genérica não é promovida: decisões e convenções sim.
        SourceRole::Decision | SourceRole::Convention => analysis.has(Intent::Docs),
        SourceRole::Config => analysis.has(Intent::Config) || analysis.has(Intent::Ci),
        SourceRole::Dependency => analysis.has(Intent::Config),
        _ => false,
    };
    if asked {
        return 0;
    }
    1 + match role {
        SourceRole::Primary => 0,
        SourceRole::Test => 1,
        SourceRole::Decision => 2,
        SourceRole::Convention => 3,
        SourceRole::Related => 4,
        SourceRole::Doc => 5,
        SourceRole::Dependency => 6,
        SourceRole::Config => 7,
    }
}

/// Compila o plano do pack. Lê os arquivos do disco para estimar tokens, sem guardar o texto.
pub fn compile(
    conn: &Connection,
    project: &Project,
    req: &ContextPackRequest,
) -> Result<ContextPackPlan> {
    let (budget, max_files) = validate(req)?;
    let task = req.task.trim().to_owned();
    let root = Path::new(&project.root_path);
    let main = search::search(
        conn,
        project,
        &SearchRequest {
            project_id: project.id,
            query: task.clone(),
            limit: Some((max_files * 2).min(search::MAX_LIMIT)),
            ..Default::default()
        },
    )?;
    let analysis = main.analysis.clone();

    let mut drafts: Vec<Draft> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let top_code = main
        .hits
        .iter()
        .filter(|h| role_for(h) == SourceRole::Primary)
        .map(|h| h.score)
        .fold(0.0_f64, f64::max);
    for hit in &main.hits {
        let mut role = role_for(hit);
        if role == SourceRole::Primary && hit.score < top_code * 0.5 {
            role = SourceRole::Related;
        }
        if seen.insert(hit.rel_path.clone()) {
            drafts.push(Draft {
                rel_path: hit.rel_path.clone(),
                role,
                start: (hit.start_line - 10).max(1),
                end: hit.end_line + 20,
                score: hit.score,
                reasons: reason_labels(&hit.reasons),
            });
        }
    }

    // Documentação: busca dedicada para não perder decisões que usam outro vocabulário.
    if !analysis.is_empty() {
        let docs = search::search(
            conn,
            project,
            &SearchRequest {
                project_id: project.id,
                query: task.clone(),
                limit: Some(5),
                kinds: Some(vec![FileKind::Doc]),
                mode: Some(SearchMode::Hybrid),
                ..Default::default()
            },
        )?;
        for hit in docs.hits.iter().take(3) {
            if seen.insert(hit.rel_path.clone()) {
                drafts.push(Draft {
                    rel_path: hit.rel_path.clone(),
                    role: role_for(hit),
                    start: 1,
                    end: EXCERPT_LINES,
                    score: hit.score,
                    reasons: reason_labels(&hit.reasons),
                });
            }
        }
    }

    // Testes com nome correspondente aos arquivos principais e relacionados.
    let primaries: Vec<String> = drafts
        .iter()
        .filter(|d| d.role == SourceRole::Primary)
        .map(|d| d.rel_path.clone())
        .collect();
    let code_files: Vec<String> = drafts
        .iter()
        .filter(|d| matches!(d.role, SourceRole::Primary | SourceRole::Related))
        .map(|d| d.rel_path.clone())
        .collect();
    for primary in &code_files {
        for test in test_counterparts(conn, project.id, primary)? {
            if seen.insert(test.clone()) {
                drafts.push(Draft {
                    rel_path: test,
                    role: SourceRole::Test,
                    start: 1,
                    end: EXCERPT_LINES,
                    score: 0.0,
                    reasons: vec![format!("Teste com nome correspondente a {primary}")],
                });
            }
        }
    }

    // Expansão 1-hop pelo grafo: testes que importam os arquivos principais e dependências
    // diretas (internas) deles. Cada fonte diz de qual import veio.
    let mut added_deps = 0;
    for primary in &primaries {
        for edge in crate::analysis::graph::importers_of(conn, project.id, primary)? {
            let is_test = file_info(conn, project.id, &edge.from_path)?
                .is_some_and(|i| i.kind == FileKind::Test);
            if is_test && seen.insert(edge.from_path.clone()) {
                drafts.push(Draft {
                    rel_path: edge.from_path.clone(),
                    role: SourceRole::Test,
                    start: 1,
                    end: EXCERPT_LINES,
                    score: 0.0,
                    reasons: vec![format!("Teste que importa {primary} (linha {})", edge.line)],
                });
            }
        }
        for edge in crate::analysis::graph::imports_of(conn, project.id, primary)? {
            if added_deps >= MAX_GRAPH_DEPENDENCIES {
                break;
            }
            let Some(to) = edge.to_path else { continue };
            let is_code =
                file_info(conn, project.id, &to)?.is_some_and(|i| i.kind == FileKind::Code);
            if is_code && seen.insert(to.clone()) {
                added_deps += 1;
                drafts.push(Draft {
                    rel_path: to,
                    role: SourceRole::Related,
                    start: 1,
                    end: EXCERPT_LINES,
                    score: 0.0,
                    reasons: vec![format!(
                        "Importado por {primary} (linha {}: `{}`)",
                        edge.line, edge.specifier
                    )],
                });
            }
        }
    }

    // Guia de contribuição/convenções do projeto, mesmo sem correspondência textual —
    // mas só quando há evidência para a tarefa (sem fontes, o pack não inventa contexto).
    let has_evidence = !drafts.is_empty();
    let mut stmt = conn.prepare(
        "SELECT rel_path FROM files WHERE project_id = ?1 AND status = 'indexed' AND kind = 'doc'
         ORDER BY length(rel_path), rel_path",
    )?;
    let docs: Vec<String> = stmt
        .query_map([project.id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    if let Some(guide) = docs
        .iter()
        .find(|d| is_convention_path(&d.to_ascii_lowercase()))
        && has_evidence
        && seen.insert(guide.clone())
    {
        drafts.push(Draft {
            rel_path: guide.clone(),
            role: SourceRole::Convention,
            start: 1,
            end: EXCERPT_LINES,
            score: 0.0,
            reasons: vec!["Guia de contribuição/convenções do projeto".into()],
        });
    }

    // Manifestos mais próximos dos arquivos principais.
    for manifest in nearest_manifests(conn, project.id, &primaries)? {
        if seen.insert(manifest.clone()) {
            drafts.push(Draft {
                rel_path: manifest.clone(),
                role: SourceRole::Dependency,
                start: 1,
                end: 40,
                score: 0.0,
                reasons: vec![
                    "Manifesto de dependências mais próximo dos arquivos principais".into(),
                ],
            });
        }
    }

    // Documentação genérica (README, guias) é limitada para não ocupar o lugar do código.
    let mut generic_docs = 0;
    drafts.retain(|d| {
        if d.role != SourceRole::Doc {
            return true;
        }
        generic_docs += 1;
        generic_docs <= MAX_GENERIC_DOCS
    });

    // Prioridade estável: papel, depois pontuação.
    drafts.sort_by(|a, b| {
        role_priority(a.role, &analysis)
            .cmp(&role_priority(b.role, &analysis))
            .then(b.score.total_cmp(&a.score))
            .then(a.rel_path.cmp(&b.rel_path))
    });

    // Memória confirmada: entra primeiro (é curta e foi revisada pelo usuário), limitada a uma
    // fração do orçamento. Propostas nunca entram.
    let draft_paths: Vec<String> = drafts.iter().map(|d| d.rel_path.clone()).collect();
    let mut memories = Vec::new();
    let mut used: u32 = 0;
    if has_evidence || !analysis.is_empty() {
        for (m, score, reasons) in memory::relevant_confirmed(
            conn,
            project.id,
            &analysis.all_terms(),
            &draft_paths,
            MAX_MEMORIES,
        )? {
            let tokens = estimate_tokens(&m.title) + estimate_tokens(&m.body) + 12;
            if used + tokens > budget / MEMORY_BUDGET_SHARE {
                continue;
            }
            used += tokens;
            memories.push(PackMemoryRef {
                memory_id: m.id,
                revision: m.revision,
                kind: m.kind,
                title: m.title,
                score,
                reasons,
                estimated_tokens: tokens,
            });
        }
    }

    // Orçamento: lê cada fonte, estima tokens, inclui enquanto couber.
    let mut sources = Vec::new();
    let mut omitted = Vec::new();
    let mut stale = 0;
    for d in drafts {
        if sources.len() as u32 >= max_files {
            omitted.push(d.rel_path);
            continue;
        }
        let Some(info) = file_info(conn, project.id, &d.rel_path)? else {
            continue;
        };
        let verified = evidence::verify(root, &d.rel_path, &info.hash)?;
        if verified.freshness == Freshness::Missing {
            stale += 1;
            continue;
        }
        if verified.freshness == Freshness::Modified {
            stale += 1;
        }
        let end = d.end.min(d.start + EXCERPT_LINES - 1);
        let (s, e, text) = evidence::slice(&verified.lines, d.start, end);
        let tokens = estimate_tokens(&text) + estimate_tokens(&d.rel_path) + 12;
        if used + tokens > budget {
            omitted.push(d.rel_path);
            continue;
        }
        used += tokens;
        sources.push(PackSource {
            rel_path: d.rel_path,
            role: d.role,
            start_line: s,
            end_line: e,
            content_hash: info.hash,
            score: d.score,
            reasons: d.reasons,
            language: info.language,
            kind: info.kind,
            estimated_tokens: tokens,
        });
    }

    let mut risks = Vec::new();
    if stale > 0 {
        risks.push(format!(
            "{stale} arquivo(s) mudaram desde a última indexação; atualize o índice para conferir as linhas."
        ));
    }
    let has_primary = sources.iter().any(|s| s.role == SourceRole::Primary);
    if has_primary && !sources.iter().any(|s| s.role == SourceRole::Test) {
        risks.push("Nenhum teste relacionado foi encontrado para os arquivos principais.".into());
    }
    let dirs: HashSet<String> = sources
        .iter()
        .filter(|s| matches!(s.role, SourceRole::Primary | SourceRole::Related))
        .map(|s| {
            s.rel_path
                .rsplit_once('/')
                .map_or(String::new(), |(d, _)| d.to_owned())
        })
        .collect();
    let sensitive = sensitive_in_dirs(conn, project.id, &dirs)?;
    if sensitive > 0 {
        risks.push(format!(
            "{sensitive} arquivo(s) sensível(is) nas mesmas pastas foram excluídos do índice e não aparecem aqui."
        ));
    }
    if let Some(last) = project.last_indexed_at
        && now_ms() - last > 24 * 3_600_000
    {
        risks.push(
            "O índice tem mais de 24 horas; mudanças recentes podem não estar refletidas.".into(),
        );
    }

    let mut limitations = vec![
        "Recuperação por texto, caminho, declarações e grafo de imports (análise léxica de TS/JS, Python e Rust); sem análise semântica nem de tipos.".to_owned(),
        "Só memórias confirmadas pelo usuário entram no pack; propostas (ex.: ADRs importados ainda não revisados) ficam de fora. Documentos de decisão listados como fontes são arquivos encontrados, não decisões confirmadas.".to_owned(),
        "Contradições são detectadas por heurística (memórias parecidas com textos diferentes, decisões cuja origem mudou ou foi substituída) e exigem revisão humana.".to_owned(),
        "Tokens estimados por caracteres ÷ 4; o número real depende do modelo.".to_owned(),
    ];
    if sources.is_empty() {
        limitations.insert(
            0,
            "Evidência insuficiente: nenhum arquivo do índice corresponde à tarefa. Reformule com termos do código (nomes de módulos, funções, pastas).".into(),
        );
    }
    let unmatched: Vec<String> = main
        .warnings
        .iter()
        .filter(|w| w.starts_with("Sem correspondência"))
        .cloned()
        .collect();
    limitations.extend(unmatched);
    if !omitted.is_empty() {
        limitations.push(format!(
            "{} arquivo(s) relevante(s) ficaram de fora pelo orçamento ou limite de arquivos.",
            omitted.len()
        ));
    }

    let mut summary = summarize(&task, &sources, &omitted);
    if !memories.is_empty() {
        summary.push_str(&format!(
            " {} memória(s) confirmada(s) do projeto.",
            memories.len()
        ));
    }
    let plan = ContextPackPlan {
        schema_version: SCHEMA_VERSION,
        id: crate::ops::new_op_id(),
        project_id: project.id,
        project_name: project.name.clone(),
        task,
        created_at: now_ms(),
        index_generation: project.index_generation,
        head_ref: project.head_ref.clone(),
        head_commit: project.head_commit.clone(),
        analysis: QueryAnalysisSnapshot::from(&analysis),
        summary,
        sources,
        omitted,
        risks,
        limitations,
        token_budget: budget,
        estimated_tokens: used,
        memories,
    };
    Ok(plan)
}

fn summarize(task: &str, sources: &[PackSource], omitted: &[String]) -> String {
    if sources.is_empty() {
        return format!("Nenhuma fonte encontrada para \"{task}\".");
    }
    let count = |r: SourceRole| sources.iter().filter(|s| s.role == r).count();
    let mut dirs: Vec<(String, usize)> = Vec::new();
    for s in sources
        .iter()
        .filter(|s| matches!(s.role, SourceRole::Primary | SourceRole::Related))
    {
        let d = s
            .rel_path
            .rsplit_once('/')
            .map_or("(raiz)".to_owned(), |(d, _)| d.to_owned());
        match dirs.iter_mut().find(|(x, _)| *x == d) {
            Some((_, n)) => *n += 1,
            None => dirs.push((d, 1)),
        }
    }
    dirs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let langs: BTreeSet<&str> = sources
        .iter()
        .filter_map(|s| s.language.as_deref())
        .collect();
    let mut parts = vec![format!(
        "{} fonte(s): {} principal(is), {} relacionada(s), {} teste(s), {} decisão(ões)/documento(s), {} convenção(ões), {} manifesto(s).",
        sources.len(),
        count(SourceRole::Primary),
        count(SourceRole::Related),
        count(SourceRole::Test),
        count(SourceRole::Decision) + count(SourceRole::Doc),
        count(SourceRole::Convention),
        count(SourceRole::Dependency)
    )];
    if !dirs.is_empty() {
        let top: Vec<String> = dirs.iter().take(3).map(|(d, _)| format!("{d}/")).collect();
        parts.push(format!("Código concentrado em {}.", top.join(", ")));
    }
    if !langs.is_empty() {
        parts.push(format!(
            "Linguagens: {}.",
            langs.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !omitted.is_empty() {
        parts.push(format!(
            "{} fonte(s) omitida(s) pelo orçamento.",
            omitted.len()
        ));
    }
    parts.join(" ")
}

/// Lê os trechos do disco agora, comparando com o hash da criação do pack.
pub fn materialize(root: &Path, plan: ContextPackPlan) -> Result<ContextPack> {
    let mut excerpts = Vec::with_capacity(plan.sources.len());
    let mut stale = 0;
    for s in &plan.sources {
        let v = evidence::verify(root, &s.rel_path, &s.content_hash)?;
        if v.freshness != Freshness::Fresh {
            stale += 1;
        }
        let (start, end, text) = evidence::slice(&v.lines, s.start_line, s.end_line);
        excerpts.push(PackExcerpt {
            start_line: start,
            end_line: end,
            text,
            freshness: v.freshness,
        });
    }
    Ok(ContextPack {
        plan,
        excerpts,
        stale_sources: stale,
        memories: Vec::new(),
        issues: Vec::new(),
    })
}

/// Completa o pack com o estado atual das memórias incluídas e os alertas abertos ligados a ele.
pub fn attach_memory(conn: &Connection, mut pack: ContextPack) -> Result<ContextPack> {
    let mut memories = Vec::with_capacity(pack.plan.memories.len());
    for r in &pack.plan.memories {
        let current = match memory::get(conn, &r.memory_id) {
            Ok(m) => Some(m),
            Err(Error::NotFound(_)) => None,
            Err(e) => return Err(e),
        };
        let state = match &current {
            None => PackMemoryState::Missing,
            Some(m) if m.status != MemoryStatus::Confirmed => PackMemoryState::Withdrawn,
            Some(m) if m.revision != r.revision => PackMemoryState::Revised,
            Some(_) => PackMemoryState::Unchanged,
        };
        memories.push(PackMemory {
            reference: r.clone(),
            state,
            current,
        });
    }
    let paths: Vec<String> = pack
        .plan
        .sources
        .iter()
        .map(|s| s.rel_path.clone())
        .collect();
    let ids: Vec<String> = pack
        .plan
        .memories
        .iter()
        .map(|m| m.memory_id.clone())
        .collect();
    pack.issues = freshness::related(conn, pack.plan.project_id, &paths, &ids)?;
    pack.memories = memories;
    Ok(pack)
}

pub fn save(conn: &mut Connection, plan: &ContextPackPlan) -> Result<()> {
    let json = serde_json::to_string(plan)
        .map_err(|e| Error::Internal(format!("serializar pack: {e}")))?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO context_packs (id, project_id, task, created_at, index_generation, head_commit,
                                    token_budget, estimated_tokens, file_count, plan_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            plan.id,
            plan.project_id,
            plan.task,
            plan.created_at,
            plan.index_generation,
            plan.head_commit,
            plan.token_budget,
            plan.estimated_tokens,
            plan.sources.len() as i64,
            json
        ],
    )?;
    for (i, s) in plan.sources.iter().enumerate() {
        tx.execute(
            "INSERT INTO context_sources (pack_id, ordinal, rel_path, role, start_line, end_line, content_hash, score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![plan.id, i as i64, s.rel_path, s.role.as_str(), s.start_line, s.end_line, s.content_hash, s.score],
        )?;
    }
    tx.execute(
        "DELETE FROM context_packs WHERE project_id = ?1 AND id NOT IN (
            SELECT id FROM context_packs WHERE project_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2)",
        params![plan.project_id, PACKS_KEPT_PER_PROJECT],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn load(conn: &Connection, id: &str) -> Result<ContextPackPlan> {
    let json: String = conn
        .query_row(
            "SELECT plan_json FROM context_packs WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("context pack {id}")))?;
    serde_json::from_str(&json).map_err(|e| Error::Storage(format!("pack {id} ilegível: {e}")))
}

pub fn list(conn: &Connection, project_id: i64, limit: u32) -> Result<Vec<ContextPackSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, task, created_at, index_generation, file_count, estimated_tokens, token_budget
         FROM context_packs WHERE project_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![project_id, limit], |r| {
        Ok(ContextPackSummary {
            id: r.get(0)?,
            project_id: r.get(1)?,
            task: r.get(2)?,
            created_at: r.get(3)?,
            index_generation: r.get(4)?,
            file_count: r.get(5)?,
            estimated_tokens: r.get(6)?,
            token_budget: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM context_packs WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(Error::NotFound(format!("context pack {id}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_match_test_conventions() {
        assert_eq!(stem("src/checkout/payment.ts"), "payment");
        assert_eq!(stem("src/checkout/payment.test.ts"), "payment");
        assert_eq!(stem("tests/test_payments.py"), "payments");
        assert_eq!(stem("pkg/cart/cart_test.go"), "cart");
    }

    #[test]
    fn estimate_is_chars_over_four() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn validates_requests() {
        let ok = ContextPackRequest {
            project_id: 1,
            task: "adicionar login".into(),
            token_budget: None,
            max_files: None,
        };
        assert_eq!(validate(&ok).unwrap(), (DEFAULT_BUDGET, DEFAULT_MAX_FILES));
        assert!(
            validate(&ContextPackRequest {
                task: "a".into(),
                ..ok.clone()
            })
            .is_err()
        );
        assert!(
            validate(&ContextPackRequest {
                token_budget: Some(10),
                ..ok.clone()
            })
            .is_err()
        );
        assert!(
            validate(&ContextPackRequest {
                max_files: Some(0),
                ..ok
            })
            .is_err()
        );
    }
}
