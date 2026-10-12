//! Fase 7 — endurecimento: leituras de arquivos do projeto depois da indexação.
//!
//! A indexação nunca segue links, mas trechos (busca, prévia, packs, MCP) são lidos de novo
//! do disco. Um arquivo trocado por link para fora do projeto, ou que passou a conter um
//! segredo depois de indexado, não pode vazar por esses caminhos.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::search::SearchRequest;
use nexus_core::{Core, DataPaths, ErrorCode};

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

fn setup() -> (tempfile::TempDir, PathBuf, Core, i64) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("acme");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saas-app"),
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
    (tmp, root, core, pid)
}

#[cfg(windows)]
fn make_dir_link(link: &Path, target: &Path) -> bool {
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

fn search_text(core: &Core, pid: i64, q: &str) -> String {
    core.search(&SearchRequest {
        project_id: pid,
        query: q.into(),
        ..Default::default()
    })
    .unwrap()
    .hits
    .iter()
    .map(|h| h.snippet.clone())
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn a_folder_swapped_for_a_link_outside_the_project_is_not_read() {
    let (tmp, root, core, pid) = setup();
    // Fora do projeto: um "session.ts" com conteúdo que não pode vazar.
    let outside = tmp.path().join("fora");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(
        outside.join("session.ts"),
        "export function validateSession() { return \"SEGREDO_DE_FORA\"; }\n",
    )
    .unwrap();
    // Depois da indexação, src/auth vira uma junction para fora do projeto.
    std::fs::remove_dir_all(root.join("src/auth")).unwrap();
    assert!(
        make_dir_link(&root.join("src").join("auth"), &outside),
        "link não criado"
    );

    let err = core
        .file_context(pid, "src/auth/session.ts", None, None)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput, "{err}");
    assert!(!search_text(&core, pid, "validateSession").contains("SEGREDO_DE_FORA"));
    let pack = core
        .compile_context_pack(&nexus_core::context::ContextPackRequest {
            project_id: pid,
            task: "validar sessão".into(),
            token_budget: None,
            max_files: None,
        })
        .unwrap();
    assert!(
        pack.excerpts
            .iter()
            .all(|e| !e.text.contains("SEGREDO_DE_FORA")),
        "packs não leem através do link"
    );
}

#[test]
fn a_file_that_now_contains_a_secret_is_not_returned() {
    let (_tmp, root, core, pid) = setup();
    let key = format!("-----BEGIN {} PRIVATE KEY-----\nMIIEsegredo\n", "RSA");
    let path = root.join("src/auth/session.ts");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&format!("// {key}"));
    std::fs::write(&path, text).unwrap();

    let err = core
        .file_context(pid, "src/auth/session.ts", None, None)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput, "{err}");
    assert!(err.to_string().contains("dados sensíveis"), "{err}");
    assert!(!search_text(&core, pid, "validateSession").contains("MIIEsegredo"));
}
