//! Mapa do código: o grafo de imports internos do projeto, pronto para desenhar.
//!
//! Nós são arquivos (ou pastas, no modo agrupado); arestas são os imports já resolvidos em
//! `edges`. Cada nó traz graus de entrada/saída e uma centralidade (PageRank), e o mapa é cortado
//! nos nós mais centrais para continuar legível. Tudo é calculado na hora, a partir do grafo que
//! a indexação já mantém: nada novo é gravado.

use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};

/// Nós no mapa, por padrão.
pub const DEFAULT_MAX_NODES: u32 = 150;
/// Limite de nós aceito num pedido.
pub const MAX_NODES: u32 = 500;
const DAMPING: f64 = 0.85;
const ITERATIONS: usize = 30;
/// Nome do nó das pastas na raiz do projeto, no modo agrupado.
pub const ROOT_FOLDER: &str = ".";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapRequest {
    pub project_id: i64,
    /// Padrão 150, máximo 500.
    pub max_nodes: Option<u32>,
    /// Só arquivos dentro desta pasta (relativa, com `/`).
    pub folder: Option<String>,
    /// Um nó por pasta em vez de um por arquivo.
    #[serde(default)]
    pub group_by_folder: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapNode {
    /// Caminho do arquivo ou da pasta.
    pub id: String,
    /// Pasta do arquivo (no modo agrupado, a própria pasta).
    pub folder: String,
    /// Linguagem do arquivo; no modo agrupado, a mais comum da pasta.
    pub language: Option<String>,
    /// Arquivos representados (1 para arquivo).
    pub files: i64,
    /// Nós distintos que importam este (no grafo inteiro, antes do corte).
    pub in_degree: i64,
    /// Nós distintos que este importa (no grafo inteiro, antes do corte).
    pub out_degree: i64,
    /// PageRank normalizado: o nó mais central vale 1.
    pub centrality: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapEdge {
    pub from: String,
    pub to: String,
    /// Imports de `from` para `to` (no modo agrupado, soma dos arquivos).
    pub weight: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMap {
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    /// Nós antes do corte.
    pub total_nodes: i64,
    /// Arestas antes do corte.
    pub total_edges: i64,
    pub grouped: bool,
}

/// Pasta de um caminho relativo (`src/a/b.ts` → `src/a`; na raiz, `.`).
pub fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(ROOT_FOLDER, |(dir, _)| dir)
}

fn normalize_folder(folder: Option<&str>) -> Result<Option<String>> {
    let Some(f) = folder.map(|f| f.trim().trim_matches('/')) else {
        return Ok(None);
    };
    if f.is_empty() || f == ROOT_FOLDER {
        return Ok(None);
    }
    if f.contains('\\') || f.split('/').any(|p| p.is_empty() || p == "." || p == "..") {
        return Err(Error::InvalidInput(format!("pasta inválida: {f}")));
    }
    Ok(Some(f.to_owned()))
}

/// Monta o mapa a partir das arestas internas (`from` importa `to`) e da linguagem de cada
/// arquivo. Separado do banco para ser testado com grafos pequenos e medido com grandes.
pub fn build(
    file_edges: &[(String, String)],
    languages: &HashMap<String, Option<String>>,
    req: &MapRequest,
) -> Result<ProjectMap> {
    let max_nodes = req.max_nodes.unwrap_or(DEFAULT_MAX_NODES);
    if max_nodes == 0 || max_nodes > MAX_NODES {
        return Err(Error::InvalidInput(format!(
            "maxNodes deve estar entre 1 e {MAX_NODES}"
        )));
    }
    let folder = normalize_folder(req.folder.as_deref())?;
    let inside = |p: &str| {
        folder
            .as_deref()
            .is_none_or(|f| p.strip_prefix(f).is_some_and(|rest| rest.starts_with('/')))
    };
    let node_of = |p: &str| -> String {
        if req.group_by_folder {
            folder_of(p).to_owned()
        } else {
            p.to_owned()
        }
    };

    // Arestas entre nós, com peso; laços (arquivo → si mesmo, pasta → si mesma) ficam de fora.
    let mut weights: BTreeMap<(String, String), i64> = BTreeMap::new();
    let mut files_of: BTreeMap<String, HashSet<&str>> = BTreeMap::new();
    for (from, to) in file_edges {
        if !inside(from) || !inside(to) {
            continue;
        }
        files_of.entry(node_of(from)).or_default().insert(from);
        files_of.entry(node_of(to)).or_default().insert(to);
        let (a, b) = (node_of(from), node_of(to));
        if a != b {
            *weights.entry((a, b)).or_default() += 1;
        }
    }

    let ids: Vec<&String> = files_of.keys().collect();
    let index: HashMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let n = ids.len();
    let mut out_links: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree = vec![0i64; n];
    for (a, b) in weights.keys() {
        let (ia, ib) = (index[a.as_str()], index[b.as_str()]);
        out_links[ia].push(ib);
        in_degree[ib] += 1;
    }
    let rank = pagerank(&out_links);

    // Corte: os mais centrais; empate pelo grau de entrada e depois pelo nome (estável).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&x, &y| {
        rank[y]
            .total_cmp(&rank[x])
            .then(in_degree[y].cmp(&in_degree[x]))
            .then(ids[x].cmp(ids[y]))
    });
    order.truncate(max_nodes as usize);
    let kept: HashSet<usize> = order.iter().copied().collect();
    let top = order
        .first()
        .map_or(1.0, |&i| rank[i])
        .max(f64::MIN_POSITIVE);

    let nodes = order
        .iter()
        .map(|&i| {
            let id = ids[i].clone();
            let files = &files_of[&id];
            MapNode {
                folder: if req.group_by_folder {
                    id.clone()
                } else {
                    folder_of(&id).to_owned()
                },
                language: dominant_language(files, languages),
                files: files.len() as i64,
                in_degree: in_degree[i],
                out_degree: out_links[i].len() as i64,
                centrality: rank[i] / top,
                id,
            }
        })
        .collect();
    let edges = weights
        .iter()
        .filter(|((a, b), _)| {
            kept.contains(&index[a.as_str()]) && kept.contains(&index[b.as_str()])
        })
        .map(|((a, b), w)| MapEdge {
            from: a.clone(),
            to: b.clone(),
            weight: *w,
        })
        .collect();
    Ok(ProjectMap {
        nodes,
        edges,
        total_nodes: n as i64,
        total_edges: weights.len() as i64,
        grouped: req.group_by_folder,
    })
}

/// Linguagem mais comum entre os arquivos (empate: ordem alfabética).
fn dominant_language(
    files: &HashSet<&str>,
    languages: &HashMap<String, Option<String>>,
) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for f in files {
        if let Some(Some(l)) = languages.get(*f) {
            *counts.entry(l.as_str()).or_default() += 1;
        }
    }
    let best = counts.values().copied().max()?;
    counts
        .into_iter()
        .find(|(_, c)| *c == best)
        .map(|(l, _)| l.to_owned())
}

/// PageRank clássico: quem é importado por arquivos importantes é importante. Nós sem saída
/// distribuem a nota igualmente (evita que a soma vaze).
fn pagerank(out_links: &[Vec<usize>]) -> Vec<f64> {
    let n = out_links.len();
    if n == 0 {
        return Vec::new();
    }
    let nf = n as f64;
    let mut rank = vec![1.0 / nf; n];
    let mut next = vec![0.0; n];
    for _ in 0..ITERATIONS {
        let dangling: f64 = out_links
            .iter()
            .zip(&rank)
            .filter(|(links, _)| links.is_empty())
            .map(|(_, r)| r)
            .sum();
        let base = (1.0 - DAMPING) / nf + DAMPING * dangling / nf;
        next.iter_mut().for_each(|v| *v = base);
        for (i, links) in out_links.iter().enumerate() {
            if links.is_empty() {
                continue;
            }
            let share = DAMPING * rank[i] / links.len() as f64;
            for &j in links {
                next[j] += share;
            }
        }
        std::mem::swap(&mut rank, &mut next);
    }
    rank
}

/// Lê do banco as arestas internas e a linguagem dos arquivos, e monta o mapa.
pub fn project_map(conn: &Connection, req: &MapRequest) -> Result<ProjectMap> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT from_path, to_path FROM edges
         WHERE project_id = ?1 AND to_path IS NOT NULL AND from_path <> to_path",
    )?;
    let edges: Vec<(String, String)> = stmt
        .query_map([req.project_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT rel_path, language FROM files WHERE project_id = ?1 AND status = 'indexed'",
    )?;
    let languages: HashMap<String, Option<String>> = stmt
        .query_map([req.project_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    build(&edges, &languages, req)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    }

    fn langs(files: &[(&str, &str)]) -> HashMap<String, Option<String>> {
        files
            .iter()
            .map(|(f, l)| ((*f).to_owned(), Some((*l).to_owned())))
            .collect()
    }

    fn req() -> MapRequest {
        MapRequest {
            project_id: 1,
            ..Default::default()
        }
    }

    /// Três arquivos importam `db.ts`; `db.ts` importa `log.ts`.
    fn star() -> Vec<(String, String)> {
        e(&[
            ("src/a.ts", "src/lib/db.ts"),
            ("src/b.ts", "src/lib/db.ts"),
            ("src/c.ts", "src/lib/db.ts"),
            ("src/lib/db.ts", "src/lib/log.ts"),
        ])
    }

    #[test]
    fn most_imported_files_come_first_with_degrees() {
        let m = build(&star(), &HashMap::new(), &req()).unwrap();
        let ids: Vec<_> = m.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(&ids[..2], ["src/lib/log.ts", "src/lib/db.ts"]);
        let db = &m.nodes[1];
        assert_eq!((db.in_degree, db.out_degree), (3, 1));
        assert!((m.nodes[0].centrality - 1.0).abs() < 1e-9);
        assert!(m.nodes.iter().all(|n| n.centrality <= 1.0 + 1e-9));
        assert_eq!((m.total_nodes, m.total_edges), (5, 4));
        assert_eq!(m.edges.len(), 4);
    }

    #[test]
    fn cut_keeps_the_central_nodes_and_only_their_edges() {
        let m = build(
            &star(),
            &HashMap::new(),
            &MapRequest {
                max_nodes: Some(2),
                ..req()
            },
        )
        .unwrap();
        assert_eq!(m.nodes.len(), 2);
        assert_eq!(m.total_nodes, 5);
        assert_eq!(
            m.edges,
            [MapEdge {
                from: "src/lib/db.ts".into(),
                to: "src/lib/log.ts".into(),
                weight: 1
            }]
        );
    }

    #[test]
    fn grouping_by_folder_sums_imports_and_drops_self_loops() {
        let languages = langs(&[
            ("src/a.ts", "typescript"),
            ("src/b.ts", "typescript"),
            ("src/c.ts", "javascript"),
            ("src/lib/db.ts", "typescript"),
            ("src/lib/log.ts", "typescript"),
        ]);
        let m = build(
            &star(),
            &languages,
            &MapRequest {
                group_by_folder: true,
                ..req()
            },
        )
        .unwrap();
        assert!(m.grouped);
        assert_eq!(m.nodes.len(), 2);
        let src = m.nodes.iter().find(|n| n.id == "src").unwrap();
        assert_eq!(src.files, 3);
        assert_eq!(src.language.as_deref(), Some("typescript"));
        assert_eq!(
            m.edges,
            [MapEdge {
                from: "src".into(),
                to: "src/lib".into(),
                weight: 3
            }]
        );
    }

    #[test]
    fn folder_filter_keeps_edges_inside_it() {
        let m = build(
            &star(),
            &HashMap::new(),
            &MapRequest {
                folder: Some("src/lib/".into()),
                ..req()
            },
        )
        .unwrap();
        let ids: Vec<_> = m.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["src/lib/log.ts", "src/lib/db.ts"]);
        // `src/lib` não pega `src/library/x.ts`.
        let m = build(
            &e(&[("src/library/x.ts", "src/library/y.ts")]),
            &HashMap::new(),
            &MapRequest {
                folder: Some("src/lib".into()),
                ..req()
            },
        )
        .unwrap();
        assert!(m.nodes.is_empty());
    }

    #[test]
    fn invalid_requests_are_refused() {
        for bad in ["../fora", "a//b", "a\\b", "./a"] {
            let r = MapRequest {
                folder: Some(bad.into()),
                ..req()
            };
            assert!(build(&star(), &HashMap::new(), &r).is_err(), "{bad}");
        }
        for max in [0, MAX_NODES + 1] {
            let r = MapRequest {
                max_nodes: Some(max),
                ..req()
            };
            assert!(build(&star(), &HashMap::new(), &r).is_err(), "{max}");
        }
    }

    #[test]
    fn root_files_belong_to_the_root_folder() {
        assert_eq!(folder_of("main.rs"), ROOT_FOLDER);
        assert_eq!(folder_of("src/a/b.ts"), "src/a");
    }

    #[test]
    fn pagerank_sums_to_one_with_dangling_nodes() {
        let r = pagerank(&[vec![1], vec![], vec![1]]);
        assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(r[1] > r[0] && r[1] > r[2]);
    }
}
