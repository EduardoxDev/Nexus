//! Importação de ADRs (registros de decisão) como **propostas** de memória.
//!
//! Heurística documentada: um ADR é um Markdown indexado cujo caminho indica decisões
//! (`decisions/`, `adr/`, `ADR-…`). Do texto são extraídos o título (primeiro `# `), o estado
//! (`Estado:`/`Status:`) e a seção de decisão (`## Decisão`/`## Decision`); sem essa seção, o
//! início do documento. A importação é idempotente: o mesmo conteúdo (hash) nunca gera outra
//! proposta, e uma decisão já confirmada nunca é sobrescrita — a mudança vira nova proposta.

use std::path::Path;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use super::{Actor, Draft, Memory, MemoryKind, MemoryRef, MemoryStatus, RevisionChange};
use crate::error::Result;
use crate::search::evidence::{self, Freshness};
use crate::time_util::now_ms;

/// Tamanho máximo do texto importado de um ADR.
const MAX_IMPORTED_CHARS: usize = 4_000;
/// Máximo de documentos examinados por importação.
const MAX_DOCS: usize = 500;

/// Situação declarada no próprio ADR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AdrState {
    Accepted,
    Proposed,
    /// Substituído, obsoleto, rejeitado ou descontinuado.
    Inactive,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAdr {
    pub title: String,
    pub body: String,
    pub state: AdrState,
    pub state_text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// ADRs examinados.
    pub scanned: u32,
    /// Novas propostas.
    pub created: u32,
    /// Propostas existentes atualizadas porque o documento mudou.
    pub updated: u32,
    /// Propostas novas que substituiriam uma decisão confirmada (o documento mudou depois da confirmação).
    pub superseding: u32,
    /// Já importados com o mesmo conteúdo.
    pub unchanged: u32,
    /// Ignorados porque o ADR está marcado como substituído/rejeitado.
    pub skipped_inactive: u32,
    /// Ignorados porque o arquivo mudou desde a indexação (atualize o índice).
    pub skipped_stale: Vec<String>,
    /// Memórias criadas ou atualizadas.
    pub memory_ids: Vec<String>,
}

/// O caminho parece um registro de decisão?
pub fn is_adr_path(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    // A memória compartilhada (`.nexus/memory/`) tem leitura própria (`memory::shared`).
    if lower.starts_with(".nexus/") {
        return false;
    }
    let is_markdown = [".md", ".markdown", ".mdx"]
        .iter()
        .any(|ext| lower.ends_with(ext));
    if !is_markdown {
        return false;
    }
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    if matches!(name, "readme.md" | "index.md" | "template.md") || name.starts_with("0000-template")
    {
        return false;
    }
    lower.contains("decision")
        || lower.split('/').any(|seg| seg == "adr" || seg == "adrs")
        || name.starts_with("adr")
}

fn classify_state(text: &str) -> AdrState {
    let t = crate::search::query::normalize(text);
    const INACTIVE: &[&str] = &[
        "substitu",
        "supersed",
        "obsolet",
        "deprecat",
        "descontinu",
        "rejeit",
        "rejected",
        "revogad",
        "abandon",
    ];
    if INACTIVE.iter().any(|w| t.contains(w)) {
        AdrState::Inactive
    } else if ["aceit", "accepted", "aprovad", "approved", "vigente"]
        .iter()
        .any(|w| t.contains(w))
    {
        AdrState::Accepted
    } else if ["propost", "proposed", "rascunho", "draft"]
        .iter()
        .any(|w| t.contains(w))
    {
        AdrState::Proposed
    } else {
        AdrState::Unknown
    }
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    rest.starts_with(' ').then(|| (level, rest.trim()))
}

fn state_line(line: &str) -> Option<String> {
    let l = line
        .trim()
        .trim_start_matches(['-', '*', '>', ' '])
        .replace("**", "")
        .replace('_', " ");
    let (key, value) = l.split_once(':')?;
    let key = crate::search::query::normalize(key.trim());
    matches!(key.as_str(), "estado" | "status" | "situacao")
        .then(|| value.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    // Corta no último fim de linha para não partir uma frase ao meio.
    match cut.rfind('\n') {
        Some(i) if i > max / 2 => format!("{}\n…", cut[..i].trim_end()),
        _ => format!("{}…", cut.trim_end()),
    }
}

/// Extrai título, estado e texto da decisão de um ADR em Markdown.
pub fn parse(text: &str, rel_path: &str) -> ParsedAdr {
    let lines: Vec<&str> = text.lines().collect();
    let mut title = None;
    let mut state_text = None;
    let mut in_fence = false;
    let mut section: Option<(usize, Vec<&str>)> = None;
    let mut decision: Option<Vec<&str>> = None;
    let mut status_section = false;
    let mut after_title: Vec<&str> = Vec::new();

    for line in &lines {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            in_fence = !in_fence;
        }
        if !in_fence && let Some((level, h)) = heading(line) {
            if level == 1 && title.is_none() {
                title = Some(h.to_owned());
                continue;
            }
            // Subtítulos dentro da seção de decisão fazem parte dela.
            if let Some((lvl, body)) = section.as_mut()
                && level > *lvl
            {
                body.push(line);
                continue;
            }
            if let Some((_, body)) = section.take()
                && decision.is_none()
            {
                decision = Some(body);
            }
            let key = crate::search::query::normalize(h);
            status_section = matches!(key.as_str(), "estado" | "status" | "situacao");
            if decision.is_none()
                && section.is_none()
                && (key.starts_with("decis") || key == "decision outcome" || key == "resultado")
            {
                section = Some((level, Vec::new()));
            }
            continue;
        }
        if state_text.is_none() && !in_fence {
            if let Some(s) = state_line(line) {
                state_text = Some(s);
                continue;
            }
            if status_section && !line.trim().is_empty() {
                state_text = Some(line.trim().trim_start_matches(['-', '*', ' ']).to_owned());
                status_section = false;
                continue;
            }
        }
        if let Some((_, body)) = section.as_mut() {
            body.push(line);
        } else if title.is_some() && decision.is_none() {
            after_title.push(line);
        }
    }
    if decision.is_none() {
        decision = section.map(|(_, b)| b);
    }

    let title = title.unwrap_or_else(|| {
        rel_path
            .rsplit('/')
            .next()
            .unwrap_or(rel_path)
            .trim_end_matches(".md")
            .replace(['-', '_'], " ")
    });
    let body_lines = decision.filter(|d| d.iter().any(|l| !l.trim().is_empty()));
    let body = body_lines
        .unwrap_or(after_title)
        .join("\n")
        .trim()
        .to_owned();
    ParsedAdr {
        title: title.chars().take(200).collect(),
        body: truncate_chars(&body, MAX_IMPORTED_CHARS),
        state: state_text
            .as_deref()
            .map_or(AdrState::Unknown, classify_state),
        state_text,
    }
}

/// ADRs indexados do projeto, em ordem de caminho.
pub fn adr_files(conn: &Connection, project_id: i64) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, content_hash FROM files
         WHERE project_id = ?1 AND status = 'indexed' AND kind = 'doc' ORDER BY rel_path",
    )?;
    let rows = stmt.query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let all: Vec<(String, String)> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(all
        .into_iter()
        .filter(|(p, _)| is_adr_path(p))
        .take(MAX_DOCS)
        .collect())
}

/// Lê e interpreta um ADR, desde que esteja igual ao indexado.
pub fn read_adr(root: &Path, rel_path: &str, hash: &str) -> Result<Option<ParsedAdr>> {
    let v = evidence::verify(root, rel_path, hash)?;
    if v.freshness != Freshness::Fresh {
        return Ok(None);
    }
    Ok(Some(parse(&v.lines.join("\n"), rel_path)))
}

fn memories_from(conn: &Connection, project_id: i64, rel_path: &str) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM memories WHERE project_id = ?1 AND origin_path = ?2
         ORDER BY created_at DESC, id DESC",
    )?;
    let ids: Vec<String> = stmt
        .query_map(params![project_id, rel_path], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ids.iter().map(|id| super::get(conn, id)).collect()
}

/// Importa os ADRs do projeto como propostas (ver regras no topo do módulo).
pub fn import(conn: &mut Connection, root: &Path, project_id: i64) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    for (rel_path, hash) in adr_files(conn, project_id)? {
        report.scanned += 1;
        let Some(adr) = read_adr(root, &rel_path, &hash)? else {
            report.skipped_stale.push(rel_path);
            continue;
        };
        let existing = memories_from(conn, project_id, &rel_path)?;
        if existing
            .iter()
            .any(|m| m.origin_hash.as_deref() == Some(hash.as_str()))
        {
            report.unchanged += 1;
            continue;
        }
        let same_text = |m: &Memory| m.title == adr.title && m.body == adr.body;
        let reference = MemoryRef {
            rel_path: rel_path.clone(),
            start_line: None,
            end_line: None,
            content_hash: hash.clone(),
        };
        let tx = conn.transaction()?;
        let now = now_ms();
        let proposal = existing
            .iter()
            .find(|m| m.status == MemoryStatus::Proposed)
            .cloned();
        let confirmed = existing
            .iter()
            .find(|m| m.status == MemoryStatus::Confirmed)
            .cloned();
        let note = format!("Importado de {rel_path}");

        if let Some(mut p) = proposal {
            // Proposta ainda não revisada: acompanha o documento.
            p.title = adr.title.clone();
            p.body = adr.body.clone();
            p.origin_hash = Some(hash.clone());
            replace_origin_ref(&mut p.refs, &reference);
            p.revision += 1;
            p.updated_at = now;
            super::store(&tx, &p)?;
            super::snapshot(
                &tx,
                &p,
                Actor::AdrImport,
                RevisionChange::Edited,
                Some("O documento de origem mudou"),
                now,
            )?;
            report.updated += 1;
            report.memory_ids.push(p.id);
        } else if let Some(mut c) = confirmed.clone().filter(same_text) {
            // Mesmo texto de decisão (só outras partes do documento mudaram): apenas metadados.
            c.origin_hash = Some(hash.clone());
            replace_origin_ref(&mut c.refs, &reference);
            c.revision += 1;
            c.updated_at = now;
            super::store(&tx, &c)?;
            super::snapshot(
                &tx,
                &c,
                Actor::System,
                RevisionChange::RefsReviewed,
                Some("Documento de origem mudou fora da seção de decisão"),
                now,
            )?;
            report.unchanged += 1;
        } else if adr.state == AdrState::Inactive && confirmed.is_none() {
            report.skipped_inactive += 1;
        } else {
            let supersedes = confirmed.as_ref().map(|c| c.id.clone());
            let m = super::insert(
                &tx,
                Draft {
                    id: None,
                    revision: 1,
                    project_id,
                    kind: MemoryKind::Decision,
                    title: adr.title.clone(),
                    body: adr.body.clone(),
                    refs: vec![reference],
                    origin_path: Some(rel_path.clone()),
                    origin_hash: Some(hash.clone()),
                    supersedes: supersedes.clone(),
                },
                Actor::AdrImport,
                false,
                Some(&note),
            )?;
            if supersedes.is_some() {
                report.superseding += 1;
            } else {
                report.created += 1;
            }
            report.memory_ids.push(m.id);
        }
        tx.commit()?;
    }
    Ok(report)
}

fn replace_origin_ref(refs: &mut Vec<MemoryRef>, reference: &MemoryRef) {
    match refs.iter_mut().find(|r| r.rel_path == reference.rel_path) {
        Some(r) => r.content_hash.clone_from(&reference.content_hash),
        None => refs.insert(0, reference.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_adr_paths() {
        assert!(is_adr_path("docs/decisions/0001-sessions.md"));
        assert!(is_adr_path("docs/adr/0002-x.md"));
        assert!(is_adr_path("ADR-0003-cache.md"));
        assert!(is_adr_path("docs/decisions/ADR-0010-busca.md"));
        assert!(!is_adr_path("docs/decisions/README.md"));
        assert!(!is_adr_path("docs/architecture.md"));
        assert!(!is_adr_path("src/decisions.ts"));
        assert!(!is_adr_path(".nexus/memory/decision-cache-1234abcd.md"));
    }

    #[test]
    fn parses_portuguese_adr() {
        let text = "# ADR 0001 — Sessões no servidor\n\nEstado: aceita\n\n## Contexto\nPrecisamos revogar.\n\n## Decisão\nUsar sessões com cookie `HttpOnly`.\nSem JWT.\n\n## Consequências\nNada.\n";
        let a = parse(text, "docs/decisions/0001.md");
        assert_eq!(a.title, "ADR 0001 — Sessões no servidor");
        assert_eq!(a.state, AdrState::Accepted);
        assert_eq!(a.body, "Usar sessões com cookie `HttpOnly`.\nSem JWT.");
    }

    #[test]
    fn parses_list_status_and_subsections() {
        let text = "# ADR-0009 — Indexação\n\n- Estado: substituída por ADR-0012\n- Data: 2026\n\n## Decisão\n\n1. Um.\n\n### Detalhe\nDois.\n\n## Consequências\nTrês.\n";
        let a = parse(text, "docs/decisions/ADR-0009.md");
        assert_eq!(a.state, AdrState::Inactive);
        assert!(a.body.contains("1. Um."));
        assert!(a.body.contains("### Detalhe\nDois."));
        assert!(!a.body.contains("Três"));
    }

    #[test]
    fn english_status_section_and_fallback_body() {
        let text =
            "# Use Postgres\n\n## Status\n\nSuperseded by 0004\n\n## Context\nWe need SQL.\n";
        let a = parse(text, "adr/0003.md");
        assert_eq!(a.state, AdrState::Inactive);
        assert_eq!(a.state_text.as_deref(), Some("Superseded by 0004"));
        // Sem seção de decisão: usa o texto após o título.
        assert!(a.body.contains("We need SQL."));
    }

    #[test]
    fn ignores_headings_inside_fences_and_truncates() {
        let text = format!(
            "# T\n\n## Decisão\n```\n# não é título\n```\n{}\n",
            "linha longa\n".repeat(1000)
        );
        let a = parse(&text, "docs/decisions/x.md");
        assert_eq!(a.title, "T");
        assert!(a.body.chars().count() <= MAX_IMPORTED_CHARS + 2);
        assert!(a.body.ends_with('…'));
    }

    #[test]
    fn title_falls_back_to_file_name() {
        let a = parse("Sem título.\n", "docs/decisions/0007-cache-local.md");
        assert_eq!(a.title, "0007 cache local");
    }
}
