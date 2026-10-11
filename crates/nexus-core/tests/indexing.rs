//! Testes de integração do indexador sobre fixtures reais + casos adversariais gerados no teste.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use nexus_core::index::{
    IndexJob, IndexMode, IndexObserver, IndexOptions, IndexTrigger, JobControl, JobStatus,
    NoopObserver,
};
use nexus_core::{Core, DataPaths, ErrorCode};
use rusqlite::params;

const MAX_FILE: u64 = 64 * 1024;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
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

fn count_files(dir: &Path) -> i64 {
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            n += count_files(&entry.path());
        } else {
            n += 1;
        }
    }
    n
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    core: Core,
    project_id: i64,
}

fn setup(name: &str) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    copy_dir(&fixture(name), &root);
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let project_id = core.add_project(&root).unwrap().id;
    Env {
        _tmp: tmp,
        root,
        core,
        project_id,
    }
}

fn options(mode: IndexMode) -> IndexOptions {
    IndexOptions {
        threads: 2,
        max_file_size: MAX_FILE,
        batch_size: 4,
        ..IndexOptions::new(mode, IndexTrigger::Cli)
    }
}

fn index(env: &Env, mode: IndexMode) -> IndexJob {
    env.core
        .index_project(
            env.project_id,
            &options(mode),
            &JobControl::new(),
            &NoopObserver,
        )
        .unwrap()
}

/// (status, hash) por caminho — o estado observável do índice.
fn snapshot(env: &Env) -> BTreeMap<String, (String, Option<String>)> {
    env.core
        .db()
        .read(|c| {
            let mut s = c.prepare(
                "SELECT rel_path, status, content_hash FROM files WHERE project_id = ?1",
            )?;
            let rows = s.query_map([env.project_id], |r| {
                Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?)))
            })?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

/// Caminhos de arquivos cujo conteúdo indexado casa com a consulta FTS.
fn fts_paths(env: &Env, query: &str) -> Vec<String> {
    env.core
        .db()
        .read(|c| {
            let mut s = c.prepare(
                "SELECT DISTINCT f.rel_path FROM chunks_fts
                 JOIN chunks ch ON ch.id = chunks_fts.rowid
                 JOIN files f ON f.project_id = ch.project_id AND f.content_hash = ch.content_hash
                 WHERE chunks_fts MATCH ?1 AND f.project_id = ?2 ORDER BY f.rel_path",
            )?;
            let rows = s.query_map(params![query, env.project_id], |r| r.get(0))?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

fn scalar(env: &Env, sql: &str) -> i64 {
    env.core
        .db()
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn write(root: &Path, rel: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Arquivos adversariais. Devolve (indexáveis extras, ignorados com registro).
fn add_adversarial(root: &Path) -> (i64, i64) {
    // Excluídos por regras (não aparecem no índice).
    write(root, "generated/out.ts", "export const gerado = 1;\n"); // .gitignore
    write(root, "debug.log", "log\n"); // .gitignore
    write(root, "node_modules/lib/index.js", "module.exports = 1;\n"); // padrão
    write(root, "fixtures-private/segredo.ts", "export const x = 1;\n"); // .nexusignore
    write(root, ".git/config", "[core]\n"); // padrão

    // Registrados como ignorados, sem conteúdo.
    let fake_aws = format!("{}{}", "AKIA", "QWERTYUIOPASDFGH");
    write(root, ".env", format!("AWS_ACCESS_KEY_ID={fake_aws}\n"));
    write(root, "deploy/server.pem", "certificado\n");
    write(
        root,
        "src/config/aws.ts",
        format!("export const key = \"{fake_aws}\";\n"),
    );
    write(root, "assets/logo.png", [0x89, b'P', b'N', b'G', 0, 1, 2]);
    write(root, "notes/raw.txt", b"texto\0binario");
    write(root, "big/huge.txt", "x".repeat(100 * 1024));
    write(
        root,
        "legacy/latin1.txt",
        [b'a', 0xE7, 0xE3, b'o', 0xE9, 0xE9, 0xE9],
    );
    let skipped = 7;

    // Indexáveis extras: duplicata e arquivo profundamente aninhado.
    let cart = std::fs::read(root.join("src/checkout/cart.ts")).unwrap();
    write(root, "src/checkout/cart_copy.ts", &cart);
    let deep: String = (0..20).map(|i| format!("d{i}/")).collect();
    write(root, &format!("{deep}profundo.md"), "# Arquivo profundo\n");
    (2, skipped)
}

#[test]
fn initial_index_has_exact_counts() {
    let env = setup("ts-webapp");
    let base = count_files(&env.root);
    let (extra, skipped) = add_adversarial(&env.root);

    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.status, JobStatus::Succeeded, "{job:?}");
    assert_eq!(job.discovered, base + extra + skipped);
    assert_eq!(job.processed, base + extra + skipped);
    assert_eq!(job.skipped, skipped);
    assert_eq!(job.failed, 0);
    assert_eq!(job.unchanged, 0);
    assert_eq!(job.removed, 0);
    // A cópia idêntica reutiliza o conteúdo do original.
    assert_eq!(job.reused_contents, 1);
    assert_eq!(job.new_contents, base + extra - 1);
    assert_eq!(job.generation, Some(1));

    let summary = env.core.project_summary(env.project_id).unwrap();
    assert_eq!(summary.counts.indexed, base + extra);
    assert_eq!(summary.counts.skipped, skipped);
    assert!(summary.counts.indexed_lines > 0);
}

#[test]
fn ignore_rules_and_default_exclusions_are_respected() {
    let env = setup("ts-webapp");
    add_adversarial(&env.root);
    index(&env, IndexMode::Incremental);
    let snap = snapshot(&env);
    for excluded in [
        "generated/out.ts",
        "debug.log",
        "node_modules/lib/index.js",
        "fixtures-private/segredo.ts",
        ".git/config",
    ] {
        assert!(
            !snap.contains_key(excluded),
            "{excluded} não deveria ser indexado"
        );
    }
    assert!(snap.contains_key(".gitignore"));
    assert!(snap.contains_key(".github/workflows/ci.yml"));
}

#[test]
fn sensitive_content_never_reaches_the_index() {
    let env = setup("ts-webapp");
    add_adversarial(&env.root);
    index(&env, IndexMode::Incremental);
    let snap = snapshot(&env);
    for path in [".env", "deploy/server.pem", "src/config/aws.ts"] {
        let (status, hash) = &snap[path];
        assert_eq!(status, "skipped_sensitive", "{path}");
        assert!(
            hash.is_none(),
            "arquivos sensíveis não guardam hash: {path}"
        );
    }
    // Montado em tempo de execução para que este próprio arquivo não pareça conter uma chave.
    let fake_key = format!("{}{}", "AKIA", "QWERTYUIOPASDFGH");
    assert!(fts_paths(&env, &fake_key).is_empty());
    assert!(fts_paths(&env, "certificado").is_empty());
    let skipped = env.core.skipped_files(env.project_id, 100).unwrap();
    for s in &skipped {
        assert!(!s.reason.as_deref().unwrap_or("").contains("AKIA"));
    }
    let aws = skipped
        .iter()
        .find(|s| s.rel_path == "src/config/aws.ts")
        .unwrap();
    assert_eq!(
        aws.reason.as_deref(),
        Some("conteúdo parece conter chave de acesso AWS (linha 1)")
    );
    assert_eq!(snap["assets/logo.png"].0, "skipped_binary");
    assert_eq!(snap["notes/raw.txt"].0, "skipped_binary");
    assert_eq!(snap["big/huge.txt"].0, "skipped_large");
    assert_eq!(snap["legacy/latin1.txt"].0, "skipped_encoding");
}

#[test]
fn text_and_identifier_subwords_are_searchable() {
    let env = setup("ts-webapp");
    index(&env, IndexMode::Incremental);
    assert_eq!(
        fts_paths(&env, "validateSession"),
        [
            "docs/decisions/0001-auth-sessions.md",
            "src/auth/session.ts"
        ]
    );
    // "validate session" só existe como partes do identificador.
    assert!(fts_paths(&env, "validate AND session").contains(&"src/auth/session.ts".to_owned()));
    // Acentos são ignorados pelo tokenizador.
    assert!(!fts_paths(&env, "autenticacao").is_empty());
}

#[test]
fn second_run_reprocesses_nothing() {
    let env = setup("py-service");
    let first = index(&env, IndexMode::Incremental);
    let before = snapshot(&env);
    let second = index(&env, IndexMode::Incremental);
    assert_eq!(second.processed, 0);
    assert_eq!(second.unchanged, first.discovered);
    assert_eq!(
        second.generation, first.generation,
        "sem mudanças, sem nova geração"
    );
    assert_eq!(snapshot(&env), before);
}

#[test]
fn full_mode_rereads_everything_but_keeps_state() {
    let env = setup("rust-lib");
    let first = index(&env, IndexMode::Incremental);
    let before = snapshot(&env);
    let full = index(&env, IndexMode::Full);
    assert_eq!(full.processed, first.discovered);
    assert_eq!(full.new_contents, 0);
    assert_eq!(
        full.reused_contents,
        first.new_contents + first.reused_contents
    );
    assert_eq!(full.generation, first.generation);
    assert_eq!(snapshot(&env), before);
}

#[test]
fn modifying_one_file_reprocesses_exactly_one() {
    let env = setup("ts-webapp");
    let first = index(&env, IndexMode::Incremental);
    write(
        &env.root,
        "src/checkout/cart.ts",
        "export const descontoEspecialBlackFriday = 0.2;\n",
    );
    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.processed, 1);
    assert_eq!(job.unchanged, first.discovered - 1);
    assert_eq!(job.new_contents, 1);
    assert_eq!(job.generation, Some(first.generation.unwrap() + 1));
    assert_eq!(
        fts_paths(&env, "descontoEspecialBlackFriday"),
        ["src/checkout/cart.ts"]
    );
    // O conteúdo antigo não é mais encontrado.
    assert!(
        fts_paths(&env, "cartTotalCents")
            .iter()
            .all(|p| p != "src/checkout/cart.ts")
    );
}

#[test]
fn deleted_files_leave_index_and_fts() {
    let env = setup("ts-webapp");
    index(&env, IndexMode::Incremental);
    assert!(!fts_paths(&env, "loginWithPassword").is_empty());
    let chunks_before = scalar(&env, "SELECT COUNT(*) FROM chunks");
    std::fs::remove_file(env.root.join("src/auth/login.ts")).unwrap();

    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.removed, 1);
    assert_eq!(job.processed, 0);
    assert!(fts_paths(&env, "loginWithPassword").is_empty());
    assert!(!snapshot(&env).contains_key("src/auth/login.ts"));
    // Conteúdo órfão coletado: chunks e linhas do FTS foram embora.
    let chunks_after = scalar(&env, "SELECT COUNT(*) FROM chunks");
    assert!(chunks_after < chunks_before);
    assert_eq!(
        scalar(&env, "SELECT COUNT(*) FROM chunks_fts"),
        chunks_after
    );
}

#[test]
fn renames_are_detected_and_reuse_content() {
    let env = setup("ts-webapp");
    index(&env, IndexMode::Incremental);
    let chunks_before = scalar(&env, "SELECT COUNT(*) FROM chunks");
    std::fs::rename(
        env.root.join("src/checkout/payment.ts"),
        env.root.join("src/checkout/payments.ts"),
    )
    .unwrap();
    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.renamed, 1);
    assert_eq!(job.removed, 1);
    assert_eq!(job.new_contents, 0);
    assert_eq!(job.reused_contents, 1);
    assert_eq!(scalar(&env, "SELECT COUNT(*) FROM chunks"), chunks_before);
    assert_eq!(
        fts_paths(&env, "checkoutCart"),
        ["src/checkout/payment.test.ts", "src/checkout/payments.ts"]
    );
}

#[test]
fn identical_files_share_one_content() {
    let env = setup("ts-webapp");
    let cart = std::fs::read(env.root.join("src/checkout/cart.ts")).unwrap();
    write(&env.root, "legacy/cart.ts", &cart);
    write(&env.root, "vendor/cart.ts", &cart);
    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.reused_contents, 2);
    let files = scalar(&env, "SELECT COUNT(*) FROM files WHERE status = 'indexed'");
    let contents = scalar(&env, "SELECT COUNT(*) FROM contents");
    assert_eq!(contents, files - 2);
    assert_eq!(fts_paths(&env, "cartTotalCents").len(), 4); // 3 cópias + payment.ts
}

#[cfg(windows)]
fn make_dir_link(link: &Path, target: &Path) -> bool {
    // Junction: não exige privilégios (ao contrário de symlinks de diretório).
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(unix)]
fn make_dir_link(link: &Path, target: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[test]
fn links_outside_the_project_are_not_followed() {
    let env = setup("rust-lib");
    let outside = env.root.parent().unwrap().join("fora-do-projeto");
    write(
        &outside,
        "segredo.rs",
        "pub const FORA: &str = \"nao indexar\";\n",
    );
    assert!(
        make_dir_link(&env.root.join("atalho"), &outside),
        "não foi possível criar o link"
    );
    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.failed, 0);
    let snap = snapshot(&env);
    assert!(snap.keys().all(|k| !k.starts_with("atalho")), "{snap:?}");
    assert!(fts_paths(&env, "FORA").is_empty());
}

#[test]
fn missing_root_fails_clearly() {
    let env = setup("rust-lib");
    std::fs::rename(&env.root, env.root.with_file_name("movido")).unwrap();
    let err = env
        .core
        .index_project(
            env.project_id,
            &options(IndexMode::Incremental),
            &JobControl::new(),
            &NoopObserver,
        )
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotFound);
    let last = env.core.list_jobs(env.project_id, 1).unwrap().remove(0);
    assert_eq!(last.status, JobStatus::Failed);
    assert!(last.error_message.unwrap().contains("não existe mais"));
}

#[test]
fn concurrent_job_for_same_project_is_rejected() {
    let env = setup("rust-lib");
    let now = nexus_core::time_util::now_ms();
    env.core
        .db()
        .write(|c| {
            c.execute(
                "INSERT INTO indexing_jobs (id, project_id, mode, trigger, status, phase, pid, started_at, heartbeat_at)
                 VALUES ('outro-processo', ?1, 'incremental', 'cli', 'running', 'indexing', 1, ?2, ?2)",
                params![env.project_id, now],
            )?;
            Ok(())
        })
        .unwrap();
    let err = env
        .core
        .index_project(
            env.project_id,
            &options(IndexMode::Incremental),
            &JobControl::new(),
            &NoopObserver,
        )
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);
}

#[test]
fn stale_job_from_dead_process_is_interrupted_and_replaced() {
    let env = setup("rust-lib");
    env.core
        .db()
        .write(|c| {
            c.execute(
                "INSERT INTO indexing_jobs (id, project_id, mode, trigger, status, phase, pid, started_at, heartbeat_at)
                 VALUES ('morto', ?1, 'incremental', 'cli', 'running', 'indexing', 1, 0, 0)",
                [env.project_id],
            )?;
            Ok(())
        })
        .unwrap();
    let job = index(&env, IndexMode::Incremental);
    assert_eq!(job.status, JobStatus::Succeeded);
    let jobs = env.core.list_jobs(env.project_id, 10).unwrap();
    let dead = jobs.iter().find(|j| j.id == "morto").unwrap();
    assert_eq!(dead.status, JobStatus::Interrupted);
}

/// Cancela depois do primeiro lote gravado.
struct CancelAfterFirstBatch<'a> {
    control: &'a JobControl,
    cancelled_at: Mutex<Option<std::time::Instant>>,
}

impl IndexObserver for CancelAfterFirstBatch<'_> {
    fn on_update(&self, job: &IndexJob) {
        if job.processed > 0 && !self.control.is_cancelled() {
            *self.cancelled_at.lock().unwrap() = Some(std::time::Instant::now());
            self.control.cancel();
        }
    }
}

#[test]
fn cancel_keeps_committed_batches_and_resume_reaches_clean_state() {
    let clean = setup("ts-webapp");
    index(&clean, IndexMode::Incremental);

    let env = setup("ts-webapp");
    let control = JobControl::new();
    let observer = CancelAfterFirstBatch {
        control: &control,
        cancelled_at: Mutex::new(None),
    };
    let job = env
        .core
        .index_project(
            env.project_id,
            &options(IndexMode::Incremental),
            &control,
            &observer,
        )
        .unwrap();
    let elapsed = observer.cancelled_at.lock().unwrap().unwrap().elapsed();
    assert_eq!(job.status, JobStatus::Cancelled);
    assert!(
        job.processed > 0 && job.processed < job.to_process,
        "{job:?}"
    );
    eprintln!("cancelamento: job encerrado {elapsed:?} após o pedido");

    let resumed = index(&env, IndexMode::Incremental);
    assert_eq!(resumed.status, JobStatus::Succeeded);
    assert_eq!(
        resumed.unchanged, job.processed,
        "lotes gravados não são refeitos"
    );
    assert_eq!(snapshot(&env), snapshot(&clean));
}

/// Simula a morte do processo no meio da indexação.
struct PanicAfterFirstBatch;

impl IndexObserver for PanicAfterFirstBatch {
    fn on_update(&self, job: &IndexJob) {
        if job.processed > 0 {
            panic!("queda simulada");
        }
    }
}

#[test]
fn crash_mid_run_is_recovered() {
    let clean = setup("py-service");
    index(&clean, IndexMode::Incremental);

    let env = setup("py-service");
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        env.core.index_project(
            env.project_id,
            &options(IndexMode::Incremental),
            &JobControl::new(),
            &PanicAfterFirstBatch,
        )
    }));
    assert!(crashed.is_err());
    let orphan = env
        .core
        .active_job(env.project_id)
        .unwrap()
        .expect("job órfão");
    // O processo "morreu": heartbeat envelhece.
    env.core
        .db()
        .write(|c| {
            c.execute(
                "UPDATE indexing_jobs SET heartbeat_at = 0 WHERE id = ?1",
                [&orphan.id],
            )?;
            Ok(())
        })
        .unwrap();

    let resumed = index(&env, IndexMode::Incremental);
    assert_eq!(resumed.status, JobStatus::Succeeded);
    assert!(
        resumed.unchanged > 0,
        "o que foi gravado antes da queda é reaproveitado"
    );
    assert_eq!(snapshot(&env), snapshot(&clean));
    let orphan = env
        .core
        .list_jobs(env.project_id, 10)
        .unwrap()
        .into_iter()
        .find(|j| j.id == orphan.id)
        .unwrap();
    assert_eq!(orphan.status, JobStatus::Interrupted);
}

struct Recorder {
    statuses: Mutex<Vec<JobStatus>>,
    paused_seen: AtomicBool,
}

impl IndexObserver for Recorder {
    fn on_update(&self, job: &IndexJob) {
        self.statuses.lock().unwrap().push(job.status);
        if job.status == JobStatus::Paused {
            self.paused_seen.store(true, Ordering::SeqCst);
        }
    }
}

#[test]
fn pause_and_resume() {
    let env = setup("ts-webapp");
    let control = JobControl::new();
    control.pause();
    let recorder = Recorder {
        statuses: Mutex::new(Vec::new()),
        paused_seen: AtomicBool::new(false),
    };
    let job = std::thread::scope(|s| {
        let handle = s.spawn(|| {
            env.core
                .index_project(
                    env.project_id,
                    &options(IndexMode::Incremental),
                    &control,
                    &recorder,
                )
                .unwrap()
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !recorder.paused_seen.load(Ordering::SeqCst) {
            assert!(std::time::Instant::now() < deadline, "job não pausou");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let paused = env.core.active_job(env.project_id).unwrap().unwrap();
        assert_eq!(paused.status, JobStatus::Paused);
        assert_eq!(paused.processed, 0);
        control.resume();
        handle.join().unwrap()
    });
    assert_eq!(job.status, JobStatus::Succeeded);
    let statuses = recorder.statuses.lock().unwrap();
    assert!(statuses.contains(&JobStatus::Paused));
    assert_eq!(statuses.last(), Some(&JobStatus::Succeeded));
}

#[test]
fn git_head_is_recorded() {
    let env = setup("rust-lib");
    let commit = "89abcdef0123456789abcdef0123456789abcdef";
    write(&env.root, ".git/HEAD", "ref: refs/heads/main\n");
    write(&env.root, ".git/refs/heads/main", format!("{commit}\n"));
    index(&env, IndexMode::Incremental);
    let project = env.core.project(env.project_id).unwrap();
    assert_eq!(project.head_ref.as_deref(), Some("refs/heads/main"));
    assert_eq!(project.head_commit.as_deref(), Some(commit));
    assert!(!snapshot(&env).keys().any(|k| k.starts_with(".git/")));
}

#[test]
fn overview_reports_languages_kinds_and_structure() {
    let env = setup("ts-webapp");
    index(&env, IndexMode::Incremental);
    let o = env.core.project_overview(env.project_id).unwrap();
    let langs: Vec<&str> = o.languages.iter().map(|l| l.language.as_str()).collect();
    assert!(langs.contains(&"typescript"));
    assert!(langs.contains(&"markdown"));
    assert!(o.manifests.contains(&"package.json".to_owned()));
    assert_eq!(o.ci_files, [".github/workflows/ci.yml"]);
    assert!(
        o.top_directories
            .iter()
            .any(|d| d.name == "src" && d.files == 6)
    );
    let tests = o
        .kinds
        .iter()
        .find(|k| k.kind == nexus_core::classify::FileKind::Test)
        .unwrap();
    assert_eq!(tests.files, 1);
}
