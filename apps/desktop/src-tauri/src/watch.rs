//! Monitoramento das pastas dos projetos. Eventos são agrupados (debounce) e viram pedidos de
//! indexação incremental — que, pelo fast-path de tamanho/mtime, só reprocessam o que mudou.
//! A reconciliação periódica cobre eventos perdidos pelo sistema operacional.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexus_core::classify::DEFAULT_EXCLUDED_DIRS;
use nexus_core::index::{IndexMode, IndexTrigger};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::indexing::{JobManager, Request};

/// Espera após o último evento antes de reindexar (salvar em rajadas gera um único job).
const DEBOUNCE: Duration = Duration::from_millis(1500);

type Roots = Arc<Mutex<HashMap<i64, PathBuf>>>;

pub struct ProjectWatcher {
    manager: JobManager,
    roots: Roots,
    debouncer: Mutex<Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>>>,
}

/// Caminhos dentro de diretórios ignorados por padrão não disparam reindexação.
pub fn is_excluded(path: &Path) -> bool {
    path.components().any(|c| match c {
        Component::Normal(name) => name
            .to_str()
            .is_some_and(|n| DEFAULT_EXCLUDED_DIRS.contains(&n)),
        _ => false,
    })
}

/// Projetos afetados por um conjunto de caminhos.
pub fn affected_projects(roots: &HashMap<i64, PathBuf>, paths: &[PathBuf]) -> HashSet<i64> {
    let mut hit = HashSet::new();
    for path in paths {
        for (id, root) in roots {
            if let Ok(rel) = path.strip_prefix(root)
                && !is_excluded(rel)
            {
                hit.insert(*id);
            }
        }
    }
    hit
}

impl ProjectWatcher {
    pub fn new(manager: JobManager) -> Self {
        Self {
            manager,
            roots: Arc::new(Mutex::new(HashMap::new())),
            debouncer: Mutex::new(None),
        }
    }

    fn create_debouncer(&self) -> Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>> {
        let roots = Arc::clone(&self.roots);
        let manager = self.manager.clone();
        let handler = move |result: DebounceEventResult| {
            let events = match result {
                Ok(events) => events,
                Err(errors) => {
                    for e in errors {
                        tracing::warn!(error = %e, "erro do monitor de arquivos");
                    }
                    return;
                }
            };
            let paths: Vec<PathBuf> = events
                .iter()
                .filter(|e| !matches!(e.kind, EventKind::Access(_)))
                .flat_map(|e| e.paths.iter().cloned())
                .collect();
            let hit = match roots.lock() {
                Ok(roots) => affected_projects(&roots, &paths),
                Err(_) => return,
            };
            for project_id in hit {
                let _ = manager.enqueue(Request {
                    project_id,
                    mode: IndexMode::Incremental,
                    trigger: IndexTrigger::Watcher,
                });
            }
        };
        match new_debouncer(DEBOUNCE, None, handler) {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::error!(error = %e, "não foi possível iniciar o monitor de arquivos");
                None
            }
        }
    }

    /// Ajusta o monitoramento à lista atual de projetos (ou desliga tudo).
    pub fn sync(&self, projects: &[(i64, PathBuf)], enabled: bool) {
        let Ok(mut guard) = self.debouncer.lock() else {
            return;
        };
        let Ok(mut roots) = self.roots.lock() else {
            return;
        };
        if !enabled {
            *guard = None; // drop para de observar
            roots.clear();
            return;
        }
        if guard.is_none() {
            *guard = self.create_debouncer();
            roots.clear();
        }
        let Some(debouncer) = guard.as_mut() else {
            return;
        };
        let wanted: HashMap<i64, PathBuf> = projects.iter().cloned().collect();
        roots.retain(|id, root| {
            let keep = wanted.get(id) == Some(root);
            if !keep {
                let _ = debouncer.unwatch(root.as_path());
            }
            keep
        });
        for (id, root) in wanted {
            if roots.contains_key(&id) || !root.is_dir() {
                continue;
            }
            match debouncer.watch(&root, RecursiveMode::Recursive) {
                Ok(()) => {
                    roots.insert(id, root);
                }
                Err(e) => {
                    tracing::warn!(project_id = id, error = %e, "não foi possível monitorar o projeto")
                }
            }
        }
        tracing::info!(watched = roots.len(), "monitor de arquivos atualizado");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_dirs_do_not_trigger() {
        assert!(is_excluded(Path::new("node_modules/x/index.js")));
        assert!(is_excluded(Path::new(".git/index")));
        assert!(is_excluded(Path::new("target/debug/app.exe")));
        assert!(!is_excluded(Path::new("src/main.rs")));
    }

    #[test]
    fn maps_paths_to_projects() {
        let mut roots = HashMap::new();
        roots.insert(1, PathBuf::from("C:/repos/a"));
        roots.insert(2, PathBuf::from("C:/repos/b"));
        let paths = vec![
            PathBuf::from("C:/repos/a/src/lib.rs"),
            PathBuf::from("C:/repos/b/node_modules/x.js"),
            PathBuf::from("C:/outro/arquivo.txt"),
        ];
        assert_eq!(affected_projects(&roots, &paths), HashSet::from([1]));
    }
}
