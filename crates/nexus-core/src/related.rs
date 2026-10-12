//! Arquivos relacionados e contexto de símbolos — heurísticas por nome, pasta e texto.
//!
//! Não há grafo de imports nem análise por linguagem (Fase 6): cada relação diz de onde veio
//! (`relation` + `reason`) e as limitações acompanham o resultado.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use crate::classify::FileKind;
use crate::context::{nearest_manifests, stem, test_counterparts};
use crate::error::{Error, Result};
use crate::projects::Project;
use crate::search::evidence::{self, Freshness};
use crate::search::{self, ReasonKind, SearchRequest, declared_name};

const MAX_SAME_DIR: usize = 10;
const MAX_MENTIONS: usize = 10;
const MAX_DEFINITIONS: usize = 5;
const MAX_REFERENCES: usize = 15;
/// Linhas mostradas a partir de uma declaração.
const DEFINITION_LINES: i64 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Teste com nome correspondente.
    Test,
    /// Arquivo testado (quando o arquivo pedido é um teste).
    TestedFile,
    SameDirectory,
    /// Manifesto de dependências mais próximo.
    Manifest,
    /// O arquivo cita o nome do módulo pedido.
    MentionsThis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RelatedFile {
    pub rel_path: String,
    pub relation: Relation,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RelatedMemory {
    pub memory_id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RelatedFiles {
    pub rel_path: String,
    pub related: Vec<RelatedFile>,
    /// Memórias (propostas ou confirmadas) que referenciam o arquivo.
    pub memories: Vec<RelatedMemory>,
    pub limitations: Vec<String>,
}

fn indexed_paths(conn: &Connection, project_id: i64) -> Result<Vec<(String, FileKind)>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, kind FROM files WHERE project_id = ?1 AND status = 'indexed'
         ORDER BY rel_path",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            FileKind::parse(&r.get::<_, String>(1)?),
        ))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Arquivos cujo conteúdo cita `term` (frase exata no FTS), exceto `exclude`.
fn files_mentioning(
    conn: &Connection,
    project_id: i64,
    term: &str,
    exclude: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let phrase = format!("\"{}\"", term.replace('"', ""));
    let mut stmt = conn.prepare(
        "SELECT DISTINCT f.rel_path FROM chunks_fts
         JOIN chunks c ON c.id = chunks_fts.rowid
         JOIN files f ON f.project_id = c.project_id AND f.content_hash = c.content_hash
         WHERE chunks_fts MATCH ?1 AND c.project_id = ?2 AND f.status = 'indexed'
           AND f.rel_path <> ?3
         ORDER BY f.rel_path LIMIT ?4",
    )?;
    let rows = stmt.query_map(params![phrase, project_id, exclude, limit as i64], |r| {
        r.get(0)
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

pub fn related_files(conn: &Connection, project: &Project, rel_path: &str) -> Result<RelatedFiles> {
    let rel_path = crate::memory::normalize_rel_path(rel_path)?;
    let file = evidence::indexed_file(conn, project.id, &rel_path)?;
    let kind = FileKind::parse(&file.kind);
    let all = indexed_paths(conn, project.id)?;
    let mut related: Vec<RelatedFile> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::from([rel_path.clone()]);
    let mut push = |related: &mut Vec<RelatedFile>, path: String, relation, reason: String| {
        if seen.insert(path.clone()) {
            related.push(RelatedFile {
                rel_path: path,
                relation,
                reason,
            });
        }
    };

    let s = stem(&rel_path);
    if kind == FileKind::Test {
        for (p, k) in &all {
            if *k == FileKind::Code && s.len() >= 4 && stem(p) == s {
                push(
                    &mut related,
                    p.clone(),
                    Relation::TestedFile,
                    format!("Nome corresponde ao teste ({s})"),
                );
            }
        }
    } else {
        for t in test_counterparts(conn, project.id, &rel_path)? {
            push(
                &mut related,
                t,
                Relation::Test,
                format!("Teste com nome correspondente ({s})"),
            );
        }
    }

    let dir = rel_path.rsplit_once('/').map_or("", |(d, _)| d);
    let mut same_dir = 0;
    for (p, k) in &all {
        let pdir = p.rsplit_once('/').map_or("", |(d, _)| d);
        if pdir == dir && matches!(k, FileKind::Code | FileKind::Test) && same_dir < MAX_SAME_DIR {
            same_dir += 1;
            push(
                &mut related,
                p.clone(),
                Relation::SameDirectory,
                format!(
                    "Mesma pasta ({})",
                    if dir.is_empty() { "raiz" } else { dir }
                ),
            );
        }
    }

    for m in nearest_manifests(conn, project.id, std::slice::from_ref(&rel_path))? {
        push(
            &mut related,
            m,
            Relation::Manifest,
            "Manifesto de dependências mais próximo".into(),
        );
    }

    let generic = matches!(
        s.as_str(),
        "index" | "main" | "init" | "lib" | "mod" | "app"
    );
    if s.chars().count() >= 4 && !generic {
        let module = rel_path
            .rsplit('/')
            .next()
            .unwrap_or(&rel_path)
            .split('.')
            .next()
            .unwrap_or("")
            .to_owned();
        for p in files_mentioning(conn, project.id, &module, &rel_path, MAX_MENTIONS)? {
            push(
                &mut related,
                p,
                Relation::MentionsThis,
                format!("Cita `{module}`"),
            );
        }
    }

    let mut stmt = conn.prepare(
        "SELECT DISTINCT m.id, m.title, m.status FROM memory_refs r JOIN memories m ON m.id = r.memory_id
         WHERE m.project_id = ?1 AND r.rel_path = ?2 AND m.status IN ('proposed', 'confirmed')
         ORDER BY m.status, m.title",
    )?;
    let memories = stmt
        .query_map(params![project.id, rel_path], |r| {
            Ok(RelatedMemory {
                memory_id: r.get(0)?,
                title: r.get(1)?,
                status: r.get(2)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    Ok(RelatedFiles {
        rel_path,
        related,
        memories,
        limitations: vec![
            "Relações por nome de arquivo, pasta, manifesto e citação textual do nome do módulo; sem grafo de imports (previsto para a Fase 6).".into(),
        ],
    })
}

// ---- Símbolos ----------------------------------------------------------------------------

static IDENT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]{1,127}$").ok());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SymbolDefinition {
    pub rel_path: String,
    pub language: Option<String>,
    /// Linha da declaração (1-based).
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub text: String,
    pub freshness: Freshness,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SymbolContext {
    pub symbol: String,
    pub definitions: Vec<SymbolDefinition>,
    /// Outros arquivos que citam o nome (prováveis usos).
    pub references: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn symbol_context(conn: &Connection, project: &Project, symbol: &str) -> Result<SymbolContext> {
    let symbol = symbol.trim();
    if !IDENT.as_ref().is_some_and(|re| re.is_match(symbol)) {
        return Err(Error::InvalidInput(
            "informe um identificador (letras, números e _; até 128 caracteres)".into(),
        ));
    }
    let root = Path::new(&project.root_path);
    let response = search::search(
        conn,
        project,
        &SearchRequest {
            project_id: project.id,
            query: symbol.to_owned(),
            limit: Some(20),
            ..Default::default()
        },
    )?;
    let mut definitions = Vec::new();
    let mut references = Vec::new();
    for hit in &response.hits {
        let declares = hit.reasons.iter().any(|r| r.kind == ReasonKind::Definition);
        let mut found = None;
        if declares && definitions.len() < MAX_DEFINITIONS {
            let file = evidence::indexed_file(conn, project.id, &hit.rel_path)?;
            let v = evidence::verify(root, &hit.rel_path, &file.content_hash)?;
            found = v
                .lines
                .iter()
                .position(|l| declared_name(l) == Some(symbol))
                .map(|i| (i as i64 + 1, v, file));
        }
        match found {
            Some((line, v, file)) => {
                let (s, e, text) =
                    evidence::slice(&v.lines, (line - 2).max(1), line + DEFINITION_LINES - 1);
                definitions.push(SymbolDefinition {
                    rel_path: hit.rel_path.clone(),
                    language: hit.language.clone(),
                    line,
                    start_line: s,
                    end_line: e,
                    text,
                    freshness: v.freshness,
                    content_hash: file.content_hash,
                });
            }
            None if references.len() < MAX_REFERENCES => references.push(hit.rel_path.clone()),
            None => {}
        }
    }
    // Usos: arquivos que contêm o nome exato, além dos já vistos na busca.
    for p in files_mentioning(conn, project.id, symbol, "", MAX_REFERENCES)? {
        if references.len() >= MAX_REFERENCES {
            break;
        }
        if !references.contains(&p) && !definitions.iter().any(|d| d.rel_path == p) {
            references.push(p);
        }
    }
    let mut limitations = vec![
        "Declarações encontradas por expressão regular (fn, function, def, class, struct, const…); sem análise por linguagem nem resolução de imports (Fase 6).".to_owned(),
    ];
    if definitions.is_empty() {
        limitations.insert(
            0,
            format!("Nenhuma declaração de `{symbol}` encontrada nos arquivos indexados."),
        );
    }
    Ok(SymbolContext {
        symbol: symbol.to_owned(),
        definitions,
        references,
        limitations,
    })
}
