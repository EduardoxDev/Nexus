//! Teste de escala (critério de aceite da Fase 2) e medição de tempos.
//! Ignorado por padrão por ser lento. Rode em release:
//!
//! ```text
//! cargo test -p nexus-core --release --test scale -- --ignored --nocapture
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::{Core, DataPaths};

const FILES: usize = 10_000;

fn synthetic_file(i: usize) -> String {
    format!(
        "// módulo {i}\nimport {{ helper{prev} }} from \"../m{prev}\";\n\n\
         export interface Record{i} {{\n  id: string;\n  total{i}: number;\n}}\n\n\
         export function computeTotal{i}(items: Record{i}[]): number {{\n\
         \x20 return items.reduce((acc, item) => acc + item.total{i}, 0);\n}}\n\n\
         export const helper{i} = (value: number) => value * {i};\n",
        prev = i.saturating_sub(1)
    )
}

#[test]
#[ignore = "lento: rode explicitamente em release"]
fn ten_thousand_files_incremental_touches_exactly_one() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let started = Instant::now();
    for i in 0..FILES {
        let dir = root.join(format!("pkg{:03}", i / 100));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("m{i}.ts")), synthetic_file(i)).unwrap();
    }
    eprintln!("gerar {FILES} arquivos: {:?}", started.elapsed());

    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let id = core.add_project(&root).unwrap().id;
    let opts = IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli);
    let run = |label: &str| {
        let t = Instant::now();
        let job = core
            .index_project(id, &opts, &JobControl::new(), &NoopObserver)
            .unwrap();
        let elapsed = t.elapsed();
        eprintln!(
            "{label}: {elapsed:?} (descobertos {}, processados {}, inalterados {}, threads {})",
            job.discovered,
            job.processed,
            job.unchanged,
            std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1)),
        );
        job
    };

    let first = run("indexação inicial");
    assert_eq!(first.discovered, FILES as i64);
    assert_eq!(first.processed, FILES as i64);

    let noop = run("incremental sem mudanças");
    assert_eq!(noop.processed, 0);
    assert_eq!(noop.unchanged, FILES as i64);

    std::fs::write(
        root.join("pkg042/m4242.ts"),
        "export const alterado = true;\n",
    )
    .unwrap();
    let one = run("incremental com 1 arquivo alterado");
    assert_eq!(one.processed, 1, "apenas o arquivo alterado é reprocessado");
    assert_eq!(one.unchanged, FILES as i64 - 1);

    // Latência de busca e de Context Pack com 10 000 arquivos indexados.
    let queries = [
        "computeTotal4242",
        "helper",
        "record total",
        "módulo 777",
        "reduce items",
        "export interface",
        "valor total dos registros",
        "m9999",
    ];
    let mut search_ms = Vec::new();
    for round in 0..3 {
        for q in queries {
            let t = Instant::now();
            let r = core
                .search(&nexus_core::search::SearchRequest {
                    project_id: id,
                    query: q.into(),
                    limit: Some(10),
                    ..Default::default()
                })
                .unwrap();
            if round == 0 {
                assert!(r.took_ms >= 0);
            }
            search_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let mut pack_ms = Vec::new();
    for q in [
        "alterar computeTotal42 para aceitar descontos",
        "helper de registros",
        "valor total",
    ] {
        let t = Instant::now();
        core.compile_context_pack(&nexus_core::context::ContextPackRequest {
            project_id: id,
            task: q.into(),
            token_budget: None,
            max_files: None,
        })
        .unwrap();
        pack_ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    search_ms.sort_by(f64::total_cmp);
    pack_ms.sort_by(f64::total_cmp);
    let p = |v: &[f64], q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    eprintln!(
        "busca ({} consultas): p50 {:.1} ms, p95 {:.1} ms; Context Pack ({}): mín {:.1} ms, máx {:.1} ms",
        search_ms.len(),
        p(&search_ms, 0.5),
        p(&search_ms, 0.95),
        pack_ms.len(),
        pack_ms[0],
        pack_ms[pack_ms.len() - 1]
    );

    // Verificação de atualidade completa (documentos, memória e stat de todos os arquivos).
    let mut scan_ms = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let report = core.scan_freshness(id).unwrap();
        scan_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(report.info.memories_checked, 0);
    }
    scan_ms.sort_by(f64::total_cmp);
    eprintln!(
        "verificação de atualidade (3x): mín {:.1} ms, máx {:.1} ms",
        scan_ms[0], scan_ms[2]
    );

    let db_size = std::fs::metadata(core.paths().db()).unwrap().len();
    let wal_size =
        std::fs::metadata(core.paths().db().with_extension("db-wal")).map_or(0, |m| m.len());
    let source_size: u64 = (0..FILES).map(|i| synthetic_file(i).len() as u64).sum();
    let mib = |b: u64| b as f64 / 1_048_576.0;
    eprintln!(
        "fonte: {:.1} MiB; banco: {:.1} MiB; WAL: {:.1} MiB",
        mib(source_size),
        mib(db_size),
        mib(wal_size)
    );
}
