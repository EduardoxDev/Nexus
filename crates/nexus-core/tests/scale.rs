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
