//! Busca híbrida: FTS5 (bm25) + caminho + cobertura dos termos + declarações + intenção por tipo
//! de arquivo + recência. Cada componente da pontuação vira um motivo legível no resultado.
//! O modo `plain` (só bm25 com os termos digitados) existe como referência para a avaliação.

pub mod evidence;
pub mod query;

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Instant;

use regex::Regex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::classify::FileKind;
use crate::error::{Error, Result};
use crate::projects::Project;
use crate::time_util::now_ms;
use evidence::{Freshness, verify};
use query::{
    Concept, Intent, QueryAnalysis, analyze, concept_in, fts_expression, matched_form, normalize,
};

pub const MAX_LIMIT: u32 = 50;
const DEFAULT_LIMIT: u32 = 10;
const MAX_QUERY_CHARS: usize = 500;
/// Linhas máximas de um trecho de resultado.
const SNIPPET_LINES: i64 = 14;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Ranking completo (padrão).
    #[default]
    Hybrid,
    /// Somente bm25 com os termos digitados (referência para avaliação).
    Plain,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    pub project_id: i64,
    pub query: String,
    /// 1..=50; padrão 10.
    #[serde(default)]
    #[ts(optional)]
    pub limit: Option<u32>,
    /// Restringe a tipos de arquivo.
    #[serde(default)]
    #[ts(optional)]
    pub kinds: Option<Vec<FileKind>>,
    /// Restringe a uma pasta (relativa, com `/`).
    #[serde(default)]
    #[ts(optional)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub mode: Option<SearchMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReasonKind {
    Content,
    Path,
    Mentioned,
    Definition,
    Coverage,
    FileKind,
    Recency,
}

/// Um componente da pontuação, explicado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    pub kind: ReasonKind,
    pub label: String,
    /// Contribuição para a pontuação final (pode ser negativa).
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub rel_path: String,
    pub language: Option<String>,
    pub kind: FileKind,
    /// Intervalo do trecho exibido (1-based, inclusivo).
    pub start_line: i64,
    pub end_line: i64,
    pub snippet: String,
    /// Linhas do trecho que contêm termos da consulta.
    pub match_lines: Vec<i64>,
    pub score: f64,
    pub reasons: Vec<Reason>,
    pub freshness: Freshness,
    pub content_hash: String,
    pub indexed_at: i64,
    /// Outros caminhos com conteúdo idêntico (deduplicados).
    pub also_in: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub mode: SearchMode,
    pub analysis: QueryAnalysis,
    pub hits: Vec<SearchHit>,
    /// Arquivos avaliados antes do corte.
    pub candidates: i64,
    pub took_ms: i64,
    pub index_generation: i64,
    pub warnings: Vec<String>,
}

struct FileRow {
    rel_path: String,
    hash: String,
    kind: FileKind,
    language: Option<String>,
    mtime_ns: i64,
    indexed_at: i64,
}

struct Candidate {
    file: usize,
    /// |bm25| do melhor chunk (0 = sem correspondência de conteúdo).
    rank: f64,
    chunk: Option<(i64, i64)>,
    score: f64,
    reasons: Vec<Reason>,
}

static DECLARATION: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:export\s+)?(?:default\s+)?(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:static\s+)?(?:fn|function|def|class|struct|enum|trait|interface|type|const|let|var|impl|func|module|record)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .ok()
});

/// Nome declarado na linha, se ela parece uma declaração.
fn declared_name(line: &str) -> Option<&str> {
    DECLARATION
        .as_ref()?
        .captures(line)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

fn validate(req: &SearchRequest) -> Result<(u32, Option<String>)> {
    let q = req.query.trim();
    if q.is_empty() {
        return Err(Error::InvalidInput("a consulta está vazia".into()));
    }
    if q.chars().count() > MAX_QUERY_CHARS {
        return Err(Error::InvalidInput(format!(
            "a consulta pode ter no máximo {MAX_QUERY_CHARS} caracteres"
        )));
    }
    let limit = req.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(Error::InvalidInput(format!(
            "limit deve estar entre 1 e {MAX_LIMIT}"
        )));
    }
    let prefix = match &req.path_prefix {
        None => None,
        Some(p) => {
            let p = p.replace('\\', "/").trim_matches('/').to_owned();
            if p.split('/').any(|s| s == ".." || s.contains(':')) {
                return Err(Error::InvalidInput("pathPrefix inválido".into()));
            }
            (!p.is_empty()).then(|| format!("{p}/"))
        }
    };
    Ok((limit, prefix))
}

fn load_files(conn: &Connection, project_id: i64) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, content_hash, kind, language, mtime_ns, indexed_at
         FROM files WHERE project_id = ?1 AND status = 'indexed' AND content_hash IS NOT NULL",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(FileRow {
            rel_path: r.get(0)?,
            hash: r.get(1)?,
            kind: FileKind::parse(&r.get::<_, String>(2)?),
            language: r.get(3)?,
            mtime_ns: r.get(4)?,
            indexed_at: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// (hash, início, fim, |bm25|) dos chunks que casam com a expressão.
fn fts_chunks(
    conn: &Connection,
    project_id: i64,
    expr: &str,
) -> Result<Vec<(String, i64, i64, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT c.content_hash, c.start_line, c.end_line, bm25(chunks_fts) AS rank
         FROM chunks_fts JOIN chunks c ON c.id = chunks_fts.rowid
         WHERE chunks_fts MATCH ?1 AND c.project_id = ?2
         ORDER BY rank LIMIT 600",
    )?;
    let rows = stmt.query_map(params![expr, project_id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, f64>(3)?.abs()))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn first_chunk(conn: &Connection, project_id: i64, hash: &str) -> Result<Option<(i64, i64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT start_line, end_line FROM chunks WHERE project_id = ?1 AND content_hash = ?2
         ORDER BY ordinal LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![project_id, hash], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.next().transpose()?)
}

fn kind_adjustment(
    kind: FileKind,
    rel_path: &str,
    analysis: &QueryAnalysis,
) -> Option<(f64, String)> {
    let lower = rel_path.to_ascii_lowercase();
    let decision_dir =
        lower.contains("decision") || lower.contains("/adr") || lower.starts_with("adr");
    match kind {
        FileKind::Lockfile => Some((-0.3, "Lockfile (raramente relevante)".into())),
        FileKind::Test if analysis.has(Intent::Tests) => {
            Some((0.2, "Arquivo de teste (a consulta pede testes)".into()))
        }
        FileKind::Test => Some((-0.05, "Arquivo de teste".into())),
        FileKind::Doc if analysis.has(Intent::Docs) && decision_dir => Some((
            0.25,
            "Registro de decisão (a consulta pede decisões/documentação)".into(),
        )),
        FileKind::Doc if analysis.has(Intent::Docs) => {
            Some((0.15, "Documentação (a consulta pede documentação)".into()))
        }
        // Sem pedido explícito de documentação, perguntas tendem a buscar código.
        FileKind::Doc => Some((-0.05, "Documentação".into())),
        FileKind::Config | FileKind::Manifest if analysis.has(Intent::Config) => {
            Some((0.12, "Configuração (a consulta pede configuração)".into()))
        }
        FileKind::Ci if analysis.has(Intent::Ci) => {
            Some((0.15, "Pipeline de CI (a consulta pede CI)".into()))
        }
        FileKind::Ci => Some((-0.1, "Pipeline de CI".into())),
        _ => None,
    }
}

fn join_terms(terms: &[String]) -> String {
    terms.iter().take(6).cloned().collect::<Vec<_>>().join(", ")
}

/// Pontuação inicial (sem ler arquivos): conteúdo, caminho, citação, tipo, recência.
fn prescore(
    c: &mut Candidate,
    file: &FileRow,
    max_rank: f64,
    analysis: &QueryAnalysis,
    now_ns: i64,
) {
    if c.rank > 0.0 {
        let text = c.rank / max_rank;
        c.score += 0.40 * text;
        c.reasons.push(Reason {
            kind: ReasonKind::Content,
            label: "Conteúdo corresponde à consulta".into(),
            weight: 0.40 * text,
        });
    }
    let path = normalize(&file.rel_path);
    let file_name = path.rsplit('/').next().unwrap_or(&path).to_owned();
    let (mut path_score, mut matched) = (0.0, Vec::new());
    for concept in &analysis.concepts {
        if let Some(form) = matched_form(concept, &file_name) {
            path_score += 1.0;
            matched.push(form);
        } else if let Some(form) = matched_form(concept, &path) {
            path_score += 0.5;
            matched.push(form);
        }
    }
    if !analysis.concepts.is_empty() && path_score > 0.0 {
        let s = (path_score / analysis.concepts.len() as f64).min(1.0);
        c.score += 0.25 * s;
        c.reasons.push(Reason {
            kind: ReasonKind::Path,
            label: format!("Caminho corresponde a: {}", join_terms(&matched)),
            weight: 0.25 * s,
        });
    }
    if analysis
        .mentioned_paths
        .iter()
        .any(|m| file.rel_path.ends_with(m.trim_start_matches("./")))
    {
        c.score += 0.5;
        c.reasons.push(Reason {
            kind: ReasonKind::Mentioned,
            label: "Citado diretamente na consulta".into(),
            weight: 0.5,
        });
    }
    if let Some((w, label)) = kind_adjustment(file.kind, &file.rel_path, analysis) {
        c.score += w;
        c.reasons.push(Reason {
            kind: ReasonKind::FileKind,
            label,
            weight: w,
        });
    }
    let age_days = (now_ns - file.mtime_ns) as f64 / 86_400e9;
    let recency = if age_days < 7.0 {
        0.03
    } else if age_days < 30.0 {
        0.015
    } else {
        0.0
    };
    if recency > 0.0 {
        c.score += recency;
        c.reasons.push(Reason {
            kind: ReasonKind::Recency,
            label: if age_days < 7.0 {
                "Alterado nos últimos 7 dias".into()
            } else {
                "Alterado nos últimos 30 dias".into()
            },
            weight: recency,
        });
    }
}

/// Força da correspondência de uma linha: 2 por termo exato, 1 por variante.
fn line_strength(line: &str, concepts: &[&Concept]) -> u32 {
    let norm = normalize(line);
    concepts
        .iter()
        .map(|c| {
            if norm.contains(c.term.as_str()) {
                2
            } else if concept_in(c, &norm) {
                1
            } else {
                0
            }
        })
        .sum()
}

/// Escolhe a janela do trecho dentro do chunk (ancorada na linha de correspondência mais forte)
/// e as linhas com correspondência.
fn snippet_window(
    lines: &[String],
    chunk: (i64, i64),
    concepts: &[&Concept],
) -> (i64, i64, String, Vec<i64>) {
    let (start, end, _) = evidence::slice(lines, chunk.0, chunk.1);
    let strengths: Vec<(i64, u32)> = (start..=end)
        .filter_map(|n| {
            let line = lines.get(usize::try_from(n - 1).ok()?)?;
            Some((n, line_strength(line, concepts)))
        })
        .collect();
    let matches: Vec<i64> = strengths
        .iter()
        .filter(|(_, s)| *s > 0)
        .map(|(n, _)| *n)
        .collect();
    let (ws, we) = if end - start < SNIPPET_LINES {
        (start, end)
    } else {
        let anchor = strengths
            .iter()
            .filter(|(_, s)| *s > 0)
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .map_or(start, |(n, _)| *n);
        let ws = (anchor - 3).max(start);
        let we = (ws + SNIPPET_LINES - 1).min(end);
        ((we - SNIPPET_LINES + 1).max(start).min(ws), we)
    };
    let (ws, we, text) = evidence::slice(lines, ws, we);
    let in_window = matches
        .into_iter()
        .filter(|n| *n >= ws && *n <= we)
        .collect();
    (ws, we, text, in_window)
}
pub fn search(conn: &Connection, project: &Project, req: &SearchRequest) -> Result<SearchResponse> {
    let started = Instant::now();
    let (limit, prefix) = validate(req)?;
    let mode = req.mode.unwrap_or_default();
    let analysis = analyze(&req.query);
    let mut warnings = Vec::new();
    let mut response = SearchResponse {
        query: req.query.trim().to_owned(),
        mode,
        analysis: analysis.clone(),
        hits: Vec::new(),
        candidates: 0,
        took_ms: 0,
        index_generation: project.index_generation,
        warnings: Vec::new(),
    };
    if analysis.is_empty() {
        response.warnings.push(
            "A consulta só tem palavras genéricas; inclua termos do domínio (ex.: \"sessão\", \"checkout\").".into(),
        );
        return Ok(response);
    }

    let files: Vec<FileRow> = load_files(conn, project.id)?
        .into_iter()
        .filter(|f| {
            prefix
                .as_ref()
                .is_none_or(|p| f.rel_path.starts_with(p.as_str()))
        })
        .filter(|f| req.kinds.as_ref().is_none_or(|k| k.contains(&f.kind)))
        .collect();
    let mut by_hash: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        by_hash.entry(f.hash.as_str()).or_default().push(i);
    }

    let terms = match mode {
        SearchMode::Plain => analysis.original_terms(),
        SearchMode::Hybrid => analysis.all_terms(),
    };
    let mut candidates: HashMap<usize, Candidate> = HashMap::new();
    if let Some(expr) = fts_expression(&terms) {
        for (hash, start, end, rank) in fts_chunks(conn, project.id, &expr)? {
            for &fi in by_hash.get(hash.as_str()).map(Vec::as_slice).unwrap_or(&[]) {
                let entry = candidates.entry(fi).or_insert(Candidate {
                    file: fi,
                    rank: 0.0,
                    chunk: None,
                    score: 0.0,
                    reasons: Vec::new(),
                });
                if rank > entry.rank {
                    entry.rank = rank;
                    entry.chunk = Some((start, end));
                }
            }
        }
    }
    if mode == SearchMode::Hybrid {
        // Correspondência só pelo caminho (ex.: `checkout` na pasta) também conta.
        for (fi, f) in files.iter().enumerate() {
            let path = normalize(&f.rel_path);
            let by_path = analysis.concepts.iter().any(|c| concept_in(c, &path))
                || analysis
                    .mentioned_paths
                    .iter()
                    .any(|m| f.rel_path.ends_with(m.as_str()));
            if by_path && !candidates.contains_key(&fi) {
                candidates.insert(
                    fi,
                    Candidate {
                        file: fi,
                        rank: 0.0,
                        chunk: None,
                        score: 0.0,
                        reasons: Vec::new(),
                    },
                );
            }
        }
    }
    response.candidates = i64::try_from(candidates.len()).unwrap_or(i64::MAX);

    let max_rank = candidates
        .values()
        .map(|c| c.rank)
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let now_ns = now_ms().saturating_mul(1_000_000);
    let mut ranked: Vec<Candidate> = candidates.into_values().collect();
    for c in &mut ranked {
        let file = &files[c.file];
        match mode {
            SearchMode::Plain => {
                c.score = c.rank / max_rank;
                c.reasons.push(Reason {
                    kind: ReasonKind::Content,
                    label: "Relevância textual (bm25)".into(),
                    weight: c.score,
                });
            }
            SearchMode::Hybrid => prescore(c, file, max_rank, &analysis, now_ns),
        }
    }
    ranked.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| files[a.file].rel_path.cmp(&files[b.file].rel_path))
    });

    // Lê do disco só os melhores: cobertura, declarações, trecho e atualidade.
    let root = Path::new(&project.root_path);
    let examine = ranked.len().min((limit as usize) * 3).max(limit as usize);
    let concept_refs: Vec<&Concept> = analysis.concepts.iter().collect();
    let mut hits: Vec<SearchHit> = Vec::new();
    for c in ranked.iter_mut().take(examine) {
        let file = &files[c.file];
        let verified = verify(root, &file.rel_path, &file.hash)?;
        if mode == SearchMode::Hybrid && !verified.lines.is_empty() {
            let norm = normalize(&verified.lines.join("\n"));
            let covered: Vec<String> = analysis
                .concepts
                .iter()
                .filter_map(|cc| matched_form(cc, &norm))
                .collect();
            if !analysis.concepts.is_empty() && !covered.is_empty() {
                let cov = covered.len() as f64 / analysis.concepts.len() as f64;
                c.score += 0.20 * cov;
                c.reasons.push(Reason {
                    kind: ReasonKind::Coverage,
                    label: format!(
                        "Cobre {} de {} termos: {}",
                        covered.len(),
                        analysis.concepts.len(),
                        join_terms(&covered)
                    ),
                    weight: 0.20 * cov,
                });
            }
            // A declaração mais específica vence: nome igual ao termo > contém o termo > igual a uma variante.
            let declared: Option<String> = verified
                .lines
                .iter()
                .filter_map(|l| {
                    let name = declared_name(l)?;
                    let n = normalize(name);
                    let rank = analysis
                        .concepts
                        .iter()
                        .filter_map(|cc| {
                            if n == cc.term {
                                Some(0)
                            } else if cc.term.len() >= 4 && n.contains(cc.term.as_str()) {
                                Some(1)
                            } else if cc
                                .variants
                                .iter()
                                .any(|v| v.len() >= 4 && n.contains(v.as_str()))
                            {
                                Some(2)
                            } else {
                                None
                            }
                        })
                        .min()?;
                    Some((rank, name.to_owned()))
                })
                .min_by_key(|(rank, _)| *rank)
                .map(|(_, name)| name);
            if let Some(name) = declared {
                c.score += 0.10;
                c.reasons.push(Reason {
                    kind: ReasonKind::Definition,
                    label: format!("Declara `{name}`"),
                    weight: 0.10,
                });
            }
        }
        let chunk = match c.chunk {
            Some(ch) => Some(ch),
            None => first_chunk(conn, project.id, &file.hash)?,
        };
        let (start, end, snippet, match_lines) = match chunk {
            Some(ch) if !verified.lines.is_empty() => {
                snippet_window(&verified.lines, ch, &concept_refs)
            }
            Some(ch) => (ch.0, ch.1, String::new(), Vec::new()),
            None => (1, 0, String::new(), Vec::new()),
        };
        hits.push(SearchHit {
            rel_path: file.rel_path.clone(),
            language: file.language.clone(),
            kind: file.kind,
            start_line: start,
            end_line: end,
            snippet,
            match_lines,
            score: (c.score * 1000.0).round() / 1000.0,
            reasons: std::mem::take(&mut c.reasons),
            freshness: verified.freshness,
            content_hash: file.hash.clone(),
            indexed_at: file.indexed_at,
            also_in: Vec::new(),
        });
    }
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.rel_path.cmp(&b.rel_path))
    });

    // Deduplicação: conteúdo idêntico aparece uma vez, com os demais caminhos em `alsoIn`.
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<SearchHit> = Vec::new();
    for hit in hits {
        match seen.get(&hit.content_hash) {
            Some(&i) => deduped[i].also_in.push(hit.rel_path),
            None => {
                seen.insert(hit.content_hash.clone(), deduped.len());
                deduped.push(hit);
            }
        }
    }
    deduped.truncate(limit as usize);

    if deduped.is_empty() {
        warnings.push(format!(
            "Nenhum resultado para: {}",
            join_terms(&analysis.original_terms())
        ));
    } else if mode == SearchMode::Hybrid {
        let missing: Vec<String> = analysis
            .concepts
            .iter()
            .filter(|cc| {
                !deduped.iter().any(|h| {
                    concept_in(cc, &normalize(&h.snippet))
                        || concept_in(cc, &normalize(&h.rel_path))
                        || h.reasons.iter().any(|r| {
                            r.kind == ReasonKind::Coverage && r.label.contains(cc.term.as_str())
                        })
                })
            })
            .map(|cc| cc.term.clone())
            .collect();
        if !missing.is_empty() {
            warnings.push(format!(
                "Sem correspondência nos resultados: {}",
                join_terms(&missing)
            ));
        }
    }
    if deduped.iter().any(|h| h.freshness != Freshness::Fresh) {
        warnings.push(
            "Alguns arquivos mudaram desde a indexação; atualize o índice para trechos exatos."
                .into(),
        );
    }
    response.hits = deduped;
    response.warnings = warnings;
    response.took_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_declarations_across_languages() {
        assert_eq!(
            declared_name("export function createSession(userId) {"),
            Some("createSession")
        );
        assert_eq!(
            declared_name("pub(crate) fn parse_line(raw: &str)"),
            Some("parse_line")
        );
        assert_eq!(
            declared_name("    def charge_with_retry(self):"),
            Some("charge_with_retry")
        );
        assert_eq!(
            declared_name("class PaymentService:"),
            Some("PaymentService")
        );
        assert_eq!(
            declared_name("export async function checkoutCart("),
            Some("checkoutCart")
        );
        assert_eq!(
            declared_name("const SESSION_TTL_MS = 1"),
            Some("SESSION_TTL_MS")
        );
        assert_eq!(declared_name("return createSession(user.id);"), None);
    }

    #[test]
    fn rejects_invalid_requests() {
        let base = SearchRequest {
            project_id: 1,
            query: "auth".into(),
            ..Default::default()
        };
        assert!(
            validate(&SearchRequest {
                query: "  ".into(),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&SearchRequest {
                limit: Some(0),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&SearchRequest {
                limit: Some(51),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&SearchRequest {
                path_prefix: Some("../x".into()),
                ..base.clone()
            })
            .is_err()
        );
        assert_eq!(
            validate(&SearchRequest {
                path_prefix: Some("src\\auth\\".into()),
                ..base
            })
            .unwrap()
            .1
            .as_deref(),
            Some("src/auth/")
        );
    }
}
