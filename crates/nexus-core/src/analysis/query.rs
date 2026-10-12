//! Consultas sobre a análise: símbolos e dependências de um arquivo, visão geral por linguagem
//! e busca de símbolos por nome.

use std::collections::BTreeMap;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use super::graph::{self, Edge, GraphStats};
use super::{Coverage, Symbol, SymbolKind, analyzer_for, coverage, normalize_name};
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileAnalysisView {
    pub rel_path: String,
    pub language: Option<String>,
    /// Há analisador para a linguagem do arquivo.
    pub analyzed: bool,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Edge>,
    pub imported_by: Vec<Edge>,
}

fn symbol_from_row(r: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<Symbol> {
    let kind: String = r.get(offset + 1)?;
    Ok(Symbol {
        name: r.get(offset)?,
        kind: SymbolKind::parse(&kind).unwrap_or(SymbolKind::Function),
        line: r.get(offset + 2)?,
        end_line: r.get(offset + 3)?,
        exported: r.get(offset + 4)?,
        container: r.get(offset + 5)?,
    })
}

pub fn symbols_of(conn: &Connection, project_id: i64, rel_path: &str) -> Result<Vec<Symbol>> {
    let mut stmt = conn.prepare_cached(
        "SELECT s.name, s.kind, s.line, s.end_line, s.exported, s.container
         FROM files f JOIN symbols s ON s.project_id = f.project_id AND s.content_hash = f.content_hash
         WHERE f.project_id = ?1 AND f.rel_path = ?2 AND f.status = 'indexed'
         ORDER BY s.line, s.id",
    )?;
    let rows = stmt.query_map(params![project_id, rel_path], |r| symbol_from_row(r, 0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn file_view(conn: &Connection, project_id: i64, rel_path: &str) -> Result<FileAnalysisView> {
    let file = crate::search::evidence::indexed_file(conn, project_id, rel_path)?;
    Ok(FileAnalysisView {
        analyzed: analyzer_for(file.language.as_deref()).is_some(),
        symbols: symbols_of(conn, project_id, &file.rel_path)?,
        imports: graph::imports_of(conn, project_id, &file.rel_path)?,
        imported_by: graph::importers_of(conn, project_id, &file.rel_path)?,
        language: file.language,
        rel_path: file.rel_path,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SymbolHit {
    pub rel_path: String,
    pub language: Option<String>,
    pub symbol: Symbol,
}

/// Declarações com o nome (comparação normalizada); as de grafia exata vêm primeiro.
pub fn find_symbols(
    conn: &Connection,
    project_id: i64,
    name: &str,
    limit: usize,
) -> Result<Vec<SymbolHit>> {
    let norm = normalize_name(name.trim());
    if norm.is_empty() {
        return Err(Error::InvalidInput("informe o nome de um símbolo".into()));
    }
    let mut stmt = conn.prepare_cached(
        "SELECT f.rel_path, f.language, s.name, s.kind, s.line, s.end_line, s.exported, s.container
         FROM symbols s JOIN files f ON f.project_id = s.project_id AND f.content_hash = s.content_hash
         WHERE s.project_id = ?1 AND s.name_norm = ?2 AND f.status = 'indexed'
         ORDER BY (s.name = ?3) DESC, f.kind = 'test', s.exported DESC, f.rel_path, s.line
         LIMIT ?4",
    )?;
    let rows = stmt.query_map(params![project_id, norm, name.trim(), limit as i64], |r| {
        Ok(SymbolHit {
            rel_path: r.get(0)?,
            language: r.get(1)?,
            symbol: symbol_from_row(r, 2)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageAnalysis {
    pub language: String,
    pub files: i64,
    pub symbols: i64,
    pub imports: i64,
    pub resolved: i64,
    pub external: i64,
    pub unresolved: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CountItem {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisOverview {
    pub graph: Option<GraphStats>,
    /// Só linguagens com analisador.
    pub languages: Vec<LanguageAnalysis>,
    /// Arquivos de código em linguagens sem analisador (por linguagem).
    pub not_analyzed: Vec<CountItem>,
    /// Arquivos mais importados dentro do projeto.
    pub most_imported: Vec<CountItem>,
    pub external_packages: Vec<CountItem>,
    pub coverage: Vec<Coverage>,
}

pub fn overview(conn: &Connection, project_id: i64) -> Result<AnalysisOverview> {
    let mut langs: BTreeMap<String, LanguageAnalysis> = BTreeMap::new();
    let mut not_analyzed: BTreeMap<String, i64> = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT f.language, COUNT(*),
                (SELECT COUNT(*) FROM files g JOIN symbols s ON s.project_id = g.project_id AND s.content_hash = g.content_hash
                  WHERE g.project_id = ?1 AND g.status = 'indexed' AND g.language = f.language),
                (SELECT COUNT(*) FROM files g JOIN imports i ON i.project_id = g.project_id AND i.content_hash = g.content_hash
                  WHERE g.project_id = ?1 AND g.status = 'indexed' AND g.language = f.language)
         FROM files f WHERE f.project_id = ?1 AND f.status = 'indexed' AND f.language IS NOT NULL
           AND f.kind IN ('code', 'test')
         GROUP BY f.language",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (language, files, symbols, imports) = row?;
        if analyzer_for(Some(&language)).is_some() {
            langs.insert(
                language.clone(),
                LanguageAnalysis {
                    language,
                    files,
                    symbols,
                    imports,
                    ..Default::default()
                },
            );
        } else {
            not_analyzed.insert(language, files);
        }
    }
    let mut stmt = conn.prepare(
        "SELECT f.language,
                SUM(e.to_path IS NOT NULL), SUM(e.external IS NOT NULL),
                SUM(e.to_path IS NULL AND e.external IS NULL)
         FROM edges e JOIN files f ON f.project_id = e.project_id AND f.rel_path = e.from_path
         WHERE e.project_id = ?1 GROUP BY f.language",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (language, resolved, external, unresolved) = row?;
        if let Some(l) = language.and_then(|l| langs.get_mut(&l)) {
            l.resolved = resolved;
            l.external = external;
            l.unresolved = unresolved;
        }
    }
    let counted = |sql: &str| -> Result<Vec<CountItem>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([project_id], |r| {
            Ok(CountItem {
                name: r.get(0)?,
                count: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    };
    Ok(AnalysisOverview {
        graph: graph::state(conn, project_id)?,
        languages: langs.into_values().collect(),
        not_analyzed: not_analyzed
            .into_iter()
            .map(|(name, count)| CountItem { name, count })
            .collect(),
        most_imported: counted(
            "SELECT to_path, COUNT(DISTINCT from_path) AS n FROM edges
             WHERE project_id = ?1 AND to_path IS NOT NULL GROUP BY to_path ORDER BY n DESC, to_path LIMIT 10",
        )?,
        external_packages: counted(
            "SELECT external, COUNT(DISTINCT from_path) AS n FROM edges
             WHERE project_id = ?1 AND external IS NOT NULL GROUP BY external ORDER BY n DESC, external LIMIT 15",
        )?,
        coverage: coverage(),
    })
}
