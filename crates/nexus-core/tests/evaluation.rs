//! Avaliação da recuperação (Fase 3): compara busca simples (bm25 com os termos digitados),
//! busca híbrida e recuperação contextual (Context Pack) sobre `tests/fixtures/saas-app`.
//!
//! ```text
//! cargo test -p nexus-core --test evaluation -- --nocapture
//! ```
//!
//! Imprime as métricas e grava `evaluation-results.json` em `CARGO_TARGET_TMPDIR`.
//! As asserções finais são guardas de regressão definidas depois da primeira medição.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use nexus_core::context::ContextPackRequest;
use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::search::evidence::Freshness;
use nexus_core::search::{SearchMode, SearchRequest};
use nexus_core::{Core, DataPaths};
use serde::Deserialize;

#[derive(Deserialize)]
struct TaskFile {
    fixture: String,
    tasks: Vec<Task>,
}

#[derive(Deserialize)]
struct Task {
    id: String,
    query: String,
    relevant: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}

#[derive(Default, Clone, Copy, serde::Serialize)]
struct Metrics {
    recall5: f64,
    precision5: f64,
    recall10: f64,
    precision10: f64,
    mrr: f64,
}

fn metrics(ranked: &[String], relevant: &[String]) -> Metrics {
    let hits_at = |k: usize| {
        ranked
            .iter()
            .take(k)
            .filter(|p| relevant.contains(p))
            .count() as f64
    };
    let first = ranked.iter().take(10).position(|p| relevant.contains(p));
    Metrics {
        recall5: hits_at(5) / relevant.len() as f64,
        precision5: hits_at(5) / 5.0,
        recall10: hits_at(10) / relevant.len() as f64,
        precision10: hits_at(10) / 10.0,
        mrr: first.map_or(0.0, |i| 1.0 / (i + 1) as f64),
    }
}

fn mean(all: &[Metrics]) -> Metrics {
    let n = all.len() as f64;
    let sum = |f: fn(&Metrics) -> f64| all.iter().map(f).sum::<f64>() / n;
    Metrics {
        recall5: sum(|m| m.recall5),
        precision5: sum(|m| m.precision5),
        recall10: sum(|m| m.recall10),
        precision10: sum(|m| m.precision10),
        mrr: sum(|m| m.mrr),
    }
}

fn percentile(values: &mut [f64], p: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let idx = ((values.len() as f64 - 1.0) * p).round() as usize;
    values[idx]
}

/// Confere uma referência contra o disco: hash do arquivo e texto das linhas.
fn reference_is_valid(
    root: &Path,
    rel: &str,
    hash: &str,
    start: i64,
    end: i64,
    text: &str,
) -> bool {
    let Ok(bytes) = std::fs::read(root.join(rel)) else {
        return false;
    };
    if blake3::hash(&bytes).to_hex().as_str() != hash {
        return false;
    }
    let content = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = content.lines().collect();
    if start < 1 || end < start || end as usize > lines.len() {
        return false;
    }
    lines[(start - 1) as usize..end as usize].join("\n") == text
}

struct Report {
    plain: Metrics,
    hybrid: Metrics,
    contextual: Metrics,
    search_p50: f64,
    search_p95: f64,
    pack_p50: f64,
    pack_p95: f64,
    refs_valid: usize,
    refs_total: usize,
    packs_with_sources: usize,
    tasks: usize,
}

fn evaluate(core: &Core, pid: i64, root: &Path, label: &str, tasks: &[Task]) -> Report {
    let (mut plain_m, mut hybrid_m, mut ctx_m) = (Vec::new(), Vec::new(), Vec::new());
    let (mut search_ms, mut pack_ms) = (Vec::new(), Vec::new());
    let (mut refs_total, mut refs_valid) = (0usize, 0usize);
    let mut packs_with_sources = 0usize;
    let mut rows = Vec::new();

    for task in tasks {
        let search = |mode| {
            core.search(&SearchRequest {
                project_id: pid,
                query: task.query.clone(),
                limit: Some(10),
                mode: Some(mode),
                ..Default::default()
            })
            .unwrap()
        };
        let plain = search(SearchMode::Plain);
        let t = Instant::now();
        let hybrid = search(SearchMode::Hybrid);
        search_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        let t = Instant::now();
        let pack = core
            .compile_context_pack(&ContextPackRequest {
                project_id: pid,
                task: task.query.clone(),
                token_budget: Some(8_000),
                max_files: Some(12),
            })
            .unwrap();
        pack_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        for hit in plain.hits.iter().chain(&hybrid.hits) {
            refs_total += 1;
            assert_eq!(hit.freshness, Freshness::Fresh);
            if reference_is_valid(
                root,
                &hit.rel_path,
                &hit.content_hash,
                hit.start_line,
                hit.end_line,
                &hit.snippet,
            ) {
                refs_valid += 1;
            }
        }
        if !pack.plan.sources.is_empty() {
            packs_with_sources += 1;
        }
        for (s, e) in pack.plan.sources.iter().zip(&pack.excerpts) {
            refs_total += 1;
            if reference_is_valid(
                root,
                &s.rel_path,
                &s.content_hash,
                e.start_line,
                e.end_line,
                &e.text,
            ) {
                refs_valid += 1;
            }
        }

        let paths = |hits: &[nexus_core::search::SearchHit]| -> Vec<String> {
            hits.iter()
                .flat_map(|h| std::iter::once(h.rel_path.clone()).chain(h.also_in.iter().cloned()))
                .collect()
        };
        let p = metrics(&paths(&plain.hits), &task.relevant);
        let h = metrics(&paths(&hybrid.hits), &task.relevant);
        let ctx_paths: Vec<String> = pack
            .plan
            .sources
            .iter()
            .map(|s| s.rel_path.clone())
            .collect();
        let c = metrics(&ctx_paths, &task.relevant);
        rows.push((
            task.id.clone(),
            p,
            h,
            c,
            hybrid
                .hits
                .first()
                .map(|x| x.rel_path.clone())
                .unwrap_or_default(),
        ));
        plain_m.push(p);
        hybrid_m.push(h);
        ctx_m.push(c);
    }

    let report = Report {
        plain: mean(&plain_m),
        hybrid: mean(&hybrid_m),
        contextual: mean(&ctx_m),
        search_p50: percentile(&mut search_ms.clone(), 0.5),
        search_p95: percentile(&mut search_ms.clone(), 0.95),
        pack_p50: percentile(&mut pack_ms.clone(), 0.5),
        pack_p95: percentile(&mut pack_ms.clone(), 0.95),
        refs_valid,
        refs_total,
        packs_with_sources,
        tasks: tasks.len(),
    };
    eprintln!("\n### {label} ({} tarefas)\n", tasks.len());
    eprintln!("| Estratégia | Recall@5 | Precision@5 | Recall@10 | Precision@10 | MRR |");
    eprintln!("|---|---|---|---|---|---|");
    for (name, m) in [
        ("Busca simples (bm25, termos digitados)", report.plain),
        ("Busca híbrida", report.hybrid),
        ("Recuperação contextual (Context Pack)", report.contextual),
    ] {
        eprintln!(
            "| {name} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |",
            m.recall5, m.precision5, m.recall10, m.precision10, m.mrr
        );
    }
    eprintln!(
        "\nPor tarefa (MRR simples / híbrida / contextual · Recall@5 híbrida · 1º resultado híbrido):"
    );
    for (id, p, h, c, first) in &rows {
        eprintln!(
            "- {id}: {:.2} / {:.2} / {:.2} · R@5 {:.2} · {first}",
            p.mrr, h.mrr, c.mrr, h.recall5
        );
    }
    eprintln!(
        "\nLatência: busca híbrida p50 {:.1} ms, p95 {:.1} ms; Context Pack p50 {:.1} ms, p95 {:.1} ms",
        report.search_p50, report.search_p95, report.pack_p50, report.pack_p95
    );
    eprintln!(
        "Referências válidas: {}/{}; packs com fontes: {}/{}",
        report.refs_valid, report.refs_total, report.packs_with_sources, report.tasks
    );
    report
}

fn report_json(r: &Report) -> serde_json::Value {
    serde_json::json!({
        "tasks": r.tasks,
        "plain": r.plain, "hybrid": r.hybrid, "contextual": r.contextual,
        "latencyMs": { "searchP50": r.search_p50, "searchP95": r.search_p95, "packP50": r.pack_p50, "packP95": r.pack_p95 },
        "references": { "valid": r.refs_valid, "total": r.refs_total },
        "packsWithSources": r.packs_with_sources,
    })
}

fn load_tasks(name: &str) -> TaskFile {
    serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("tests/evaluation").join(name)).unwrap(),
    )
    .unwrap()
}

#[test]
fn retrieval_quality() {
    let dev = load_tasks("tasks.json");
    let holdout = load_tasks("holdout.json");
    assert!(
        dev.tasks.len() >= 25,
        "o conjunto principal precisa de pelo menos 25 tarefas"
    );
    assert_eq!(dev.fixture, holdout.fixture);

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    copy_dir(
        &repo_root().join("tests/fixtures").join(&dev.fixture),
        &root,
    );
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(&root).unwrap().id;
    core.index_project(
        pid,
        &IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli),
        &JobControl::new(),
        &NoopObserver,
    )
    .unwrap();
    let root = PathBuf::from(core.project(pid).unwrap().root_path);

    let dev_report = evaluate(&core, pid, &root, "Conjunto principal", &dev.tasks);
    let holdout_report = evaluate(&core, pid, &root, "Conjunto reservado", &holdout.tasks);

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("evaluation-results.json");
    let json = serde_json::json!({ "main": report_json(&dev_report), "holdout": report_json(&holdout_report) });
    std::fs::write(&out, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    eprintln!("\nResultados gravados em {}", out.display());

    for r in [&dev_report, &holdout_report] {
        // Invariante: toda referência (caminho + linhas + hash) confere com o disco.
        assert_eq!(r.refs_valid, r.refs_total, "referências inválidas");
        assert_eq!(
            r.packs_with_sources, r.tasks,
            "todo pack deve ter fontes nesta fixture"
        );
    }
    // Guardas de regressão (definidas após a medição registrada em docs/evaluation.md).
    assert!(dev_report.hybrid.mrr > dev_report.plain.mrr);
    assert!(dev_report.hybrid.recall5 > dev_report.plain.recall5);
    assert!(
        dev_report.hybrid.mrr >= 0.9,
        "MRR híbrido caiu: {:.3}",
        dev_report.hybrid.mrr
    );
    assert!(
        dev_report.contextual.recall10 >= 0.9,
        "Recall@10 contextual caiu"
    );
}
