//! Grafo de dependências entre arquivos, com evidência por aresta (linha e especificador).
//!
//! Incremental em duas camadas: a **análise** (símbolos e imports) é feita uma vez por conteúdo
//! durante a indexação; a **resolução** (import → arquivo) depende do conjunto de caminhos e
//! é refeita para o projeto inteiro quando o índice muda de geração — uma passada em memória,
//! medida em `docs/benchmarks.md`.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use ts_rs::TS;

use super::resolve::{Resolver, Target, TsConfig};
use super::{FileAnalysis, Import, ImportKind, normalize_name};
use crate::error::Result;
use crate::storage::Db;
use crate::time_util::now_ms;

/// Grava símbolos e imports de um conteúdo (substitui os anteriores).
pub(crate) fn store_analysis(
    tx: &Transaction<'_>,
    project_id: i64,
    hash: &str,
    analysis: &FileAnalysis,
) -> Result<()> {
    tx.execute(
        "DELETE FROM symbols WHERE project_id = ?1 AND content_hash = ?2",
        params![project_id, hash],
    )?;
    tx.execute(
        "DELETE FROM imports WHERE project_id = ?1 AND content_hash = ?2",
        params![project_id, hash],
    )?;
    let mut sym = tx.prepare_cached(
        "INSERT INTO symbols (project_id, content_hash, name, name_norm, kind, line, end_line, exported, container)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for s in &analysis.symbols {
        sym.execute(params![
            project_id,
            hash,
            s.name,
            normalize_name(&s.name),
            s.kind.as_str(),
            s.line,
            s.end_line,
            s.exported,
            s.container
        ])?;
    }
    let mut imp = tx.prepare_cached(
        "INSERT INTO imports (project_id, content_hash, ordinal, line, specifier, kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (i, im) in analysis.imports.iter().enumerate() {
        imp.execute(params![
            project_id,
            hash,
            i as i64,
            im.line,
            im.specifier,
            im.kind.as_str()
        ])?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub index_generation: i64,
    pub built_at: i64,
    pub duration_ms: i64,
    /// Arquivos com imports analisados.
    pub files: i64,
    pub edges: i64,
    /// Arestas para arquivos do projeto.
    pub resolved: i64,
    /// Arestas para pacotes/crates externos.
    pub external: i64,
    /// Imports relativos/internos que não correspondem a nenhum arquivo.
    pub unresolved: i64,
}

pub fn state(conn: &Connection, project_id: i64) -> Result<Option<GraphStats>> {
    Ok(conn
        .query_row(
            "SELECT index_generation, built_at, duration_ms, files, edges, resolved, external, unresolved
             FROM graph_state WHERE project_id = ?1",
            [project_id],
            |r| {
                Ok(GraphStats {
                    index_generation: r.get(0)?,
                    built_at: r.get(1)?,
                    duration_ms: r.get(2)?,
                    files: r.get(3)?,
                    edges: r.get(4)?,
                    resolved: r.get(5)?,
                    external: r.get(6)?,
                    unresolved: r.get(7)?,
                })
            },
        )
        .optional()?)
}

pub fn needs_rebuild(conn: &Connection, project_id: i64, generation: i64) -> Result<bool> {
    Ok(state(conn, project_id)?.is_none_or(|s| s.index_generation != generation))
}

fn crate_name(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package
            && let Some(v) = t.strip_prefix("name")
            && let Some(v) = v.trim_start().strip_prefix('=')
        {
            return Some(v.trim().trim_matches('"').replace('-', "_"));
        }
    }
    None
}

/// (origem, linha, especificador, tipo, arquivo de destino, pacote externo)
type EdgeRow = (
    String,
    i64,
    String,
    &'static str,
    Option<String>,
    Option<String>,
);

/// Recalcula todas as arestas do projeto a partir dos imports já analisados.
pub fn rebuild(db: &Db, project_id: i64, root: &Path, generation: i64) -> Result<GraphStats> {
    let started = Instant::now();
    // (caminho, linguagem, hash)
    let files: Vec<(String, Option<String>, String)> = db.read(|c| {
        let mut stmt = c.prepare(
            "SELECT rel_path, language, content_hash FROM files
             WHERE project_id = ?1 AND status = 'indexed' AND content_hash IS NOT NULL",
        )?;
        let rows = stmt.query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    })?;
    let mut imports: HashMap<String, Vec<Import>> = HashMap::new();
    db.read(|c| {
        let mut stmt = c.prepare(
            "SELECT content_hash, line, specifier, kind FROM imports WHERE project_id = ?1 ORDER BY content_hash, ordinal",
        )?;
        let rows = stmt.query_map([project_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Import {
                    line: r.get(1)?,
                    specifier: r.get(2)?,
                    kind: ImportKind::parse(&r.get::<_, String>(3)?),
                },
            ))
        })?;
        for row in rows {
            let (hash, imp) = row?;
            imports.entry(hash).or_default().push(imp);
        }
        Ok(())
    })?;
    // Todos os caminhos conhecidos (também ignorados: um import de `./data.json` grande existe).
    let all_paths: HashSet<String> = db.read(|c| {
        let mut stmt = c.prepare("SELECT rel_path FROM files WHERE project_id = ?1")?;
        let rows = stmt.query_map([project_id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    })?;

    // Configurações lidas do disco (só as indexadas; nunca arquivos ignorados).
    let mut tsconfigs = Vec::new();
    let mut crates = HashMap::new();
    let mut go_modules = Vec::new();
    for (path, _, _) in &files {
        let name = path.rsplit('/').next().unwrap_or(path);
        let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        let read = || match crate::scan::check_project_path(root, path) {
            Ok(crate::scan::ProjectPath::File(p)) => std::fs::read_to_string(p).ok(),
            _ => None,
        };
        if name.starts_with("tsconfig") && name.ends_with(".json") {
            if let Some(cfg) = read().and_then(|t| TsConfig::parse(dir, &t)) {
                tsconfigs.push(cfg);
            }
        } else if name == "Cargo.toml"
            && let Some(n) = read().and_then(|t| crate_name(&t))
        {
            crates.insert(n, dir.to_owned());
        } else if name == "go.mod"
            && let Some(m) = read().and_then(|t| super::go::module_path(&t))
        {
            go_modules.push((m, dir.to_owned()));
        }
    }
    // `package` (Java) e `namespace` (C#) já analisados: resolvem imports por nome.
    let namespaces: Vec<(String, String, String)> = db.read(|c| {
        let mut stmt = c.prepare(
            "SELECT f.language, s.name, f.rel_path FROM files f
             JOIN symbols s ON s.project_id = f.project_id AND s.content_hash = f.content_hash
             WHERE f.project_id = ?1 AND f.status = 'indexed' AND s.kind = 'module'
               AND f.language IN ('java', 'csharp')",
        )?;
        let rows = stmt.query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    })?;
    let resolver = Resolver::new(&all_paths, tsconfigs, crates)
        .with_go_modules(go_modules)
        .with_namespaces(namespaces);

    let mut stats = GraphStats {
        index_generation: generation,
        built_at: now_ms(),
        ..Default::default()
    };
    // (from, line, specifier, kind, to, external)
    let mut edges: Vec<EdgeRow> = Vec::new();
    for (path, language, hash) in &files {
        let (Some(lang), Some(list)) = (language.as_deref(), imports.get(hash)) else {
            continue;
        };
        stats.files += 1;
        for imp in list {
            // Um import pode apontar para vários arquivos (pacote Go, namespace C#).
            let targets: Vec<(Option<String>, Option<String>)> =
                match resolver.resolve(path, lang, imp) {
                    Target::File(f) if f != *path => {
                        stats.resolved += 1;
                        vec![(Some(f), None)]
                    }
                    Target::File(_) => continue, // auto-import (ex.: `use self::X` no próprio arquivo)
                    Target::Files(files) => {
                        stats.resolved += 1;
                        files.into_iter().map(|f| (Some(f), None)).collect()
                    }
                    Target::External(e) => {
                        stats.external += 1;
                        vec![(None, Some(e))]
                    }
                    Target::Unresolved => {
                        stats.unresolved += 1;
                        vec![(None, None)]
                    }
                };
            for (to, ext) in targets {
                edges.push((
                    path.clone(),
                    imp.line,
                    imp.specifier.clone(),
                    imp.kind.as_str(),
                    to,
                    ext,
                ));
            }
        }
    }
    stats.edges = edges.len() as i64;
    db.write(|c| {
        let tx = c.transaction()?;
        tx.execute("DELETE FROM edges WHERE project_id = ?1", [project_id])?;
        {
            let mut ins = tx.prepare_cached(
                "INSERT OR IGNORE INTO edges (project_id, from_path, line, specifier, kind, to_path, external, target)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for (from, line, spec, kind, to, ext) in &edges {
                let target = to.as_deref().or(ext.as_deref()).unwrap_or("");
                ins.execute(params![project_id, from, line, spec, kind, to, ext, target])?;
            }
        }
        stats.duration_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
        tx.execute(
            "INSERT INTO graph_state (project_id, index_generation, built_at, duration_ms, files, edges, resolved, external, unresolved)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT (project_id) DO UPDATE SET
               index_generation = excluded.index_generation, built_at = excluded.built_at,
               duration_ms = excluded.duration_ms, files = excluded.files, edges = excluded.edges,
               resolved = excluded.resolved, external = excluded.external, unresolved = excluded.unresolved",
            params![
                project_id,
                stats.index_generation,
                stats.built_at,
                stats.duration_ms,
                stats.files,
                stats.edges,
                stats.resolved,
                stats.external,
                stats.unresolved
            ],
        )?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(stats)
}

/// Aresta vista de um arquivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from_path: String,
    pub line: i64,
    pub specifier: String,
    pub kind: String,
    pub to_path: Option<String>,
    pub external: Option<String>,
}

fn edge_rows(conn: &Connection, sql: &str, project_id: i64, path: &str) -> Result<Vec<Edge>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params![project_id, path], |r| {
        Ok(Edge {
            from_path: r.get(0)?,
            line: r.get(1)?,
            specifier: r.get(2)?,
            kind: r.get(3)?,
            to_path: r.get(4)?,
            external: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// O que o arquivo importa.
pub fn imports_of(conn: &Connection, project_id: i64, path: &str) -> Result<Vec<Edge>> {
    edge_rows(
        conn,
        "SELECT from_path, line, specifier, kind, to_path, external FROM edges
         WHERE project_id = ?1 AND from_path = ?2 ORDER BY line, specifier",
        project_id,
        path,
    )
}

/// Quem importa o arquivo.
pub fn importers_of(conn: &Connection, project_id: i64, path: &str) -> Result<Vec<Edge>> {
    edge_rows(
        conn,
        "SELECT from_path, line, specifier, kind, to_path, external FROM edges
         WHERE project_id = ?1 AND to_path = ?2 ORDER BY from_path, line",
        project_id,
        path,
    )
}

/// Grau de entrada (quantos arquivos distintos importam cada arquivo).
pub fn in_degrees(conn: &Connection, project_id: i64) -> Result<HashMap<String, i64>> {
    let mut stmt = conn.prepare_cached(
        "SELECT to_path, COUNT(DISTINCT from_path) FROM edges
         WHERE project_id = ?1 AND to_path IS NOT NULL GROUP BY to_path",
    )?;
    let rows = stmt.query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_crate_name_from_package_section() {
        let toml =
            "[workspace]\nname = \"no\"\n[package]\nname = \"rust-lib\"\nversion = \"0.1.0\"\n";
        assert_eq!(crate_name(toml).as_deref(), Some("rust_lib"));
        assert_eq!(crate_name("[dependencies]\nname = \"x\"\n"), None);
    }
}
