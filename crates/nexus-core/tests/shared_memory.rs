//! Memória compartilhada pelo repositório (`.nexus/memory/`, ADR-0014): ida e volta entre dois
//! bancos, só propostas na leitura, prévia obrigatória na exportação e caminhos conferidos.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::memory::shared::{DIR, ExportAction};
use nexus_core::memory::{
    Actor, Memory, MemoryKind, MemoryOrigin, MemoryQuery, MemoryRefInput, MemoryStatus,
    MemoryUpdate, NewMemory, Transition,
};
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

struct Env {
    tmp: tempfile::TempDir,
    root: PathBuf,
    core: Core,
    pid: i64,
}

/// Um "computador": cópia da fixture e banco próprio.
fn machine() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saas-app"),
        &root,
    );
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(&root).unwrap().id;
    let env = Env {
        tmp,
        root,
        core,
        pid,
    };
    reindex(&env);
    env
}

fn reindex(env: &Env) {
    env.core
        .index_project(
            env.pid,
            &IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli),
            &JobControl::new(),
            &NoopObserver,
        )
        .unwrap();
}

fn confirmed(env: &Env, title: &str, body: &str, refs: &[&str]) -> Memory {
    env.core
        .create_memory(
            &NewMemory {
                project_id: env.pid,
                kind: MemoryKind::Decision,
                title: title.into(),
                body: body.into(),
                refs: refs
                    .iter()
                    .map(|p| MemoryRefInput {
                        rel_path: (*p).into(),
                        start_line: Some(1),
                        end_line: Some(3),
                    })
                    .collect(),
                confirm: true,
                note: None,
            },
            Actor::User,
        )
        .unwrap()
}

fn all(env: &Env) -> Vec<Memory> {
    let mut list = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            ..Default::default()
        })
        .unwrap();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}

fn export_all(env: &Env) {
    let preview = env.core.preview_memory_export(env.pid).unwrap();
    env.core.export_memory(env.pid, &preview.token).unwrap();
}

/// Copia `.nexus/memory` de um repositório para outro (o que um `git pull` faria).
fn pull(from: &Env, to: &Env) {
    copy_dir(&from.root.join(DIR), &to.root.join(DIR));
}

fn shared_files(env: &Env) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(env.root.join(DIR))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    files
}

#[test]
fn round_trip_between_two_machines_keeps_the_same_memories() {
    let a = machine();
    let sessions = confirmed(
        &a,
        "Sessões no servidor",
        "Cookie `HttpOnly`, sem JWT.",
        &["src/auth/session.ts"],
    );
    let money = confirmed(&a, "Dinheiro em centavos", "Sempre inteiros.", &[]);
    // Proposta não sai do banco.
    a.core
        .create_memory(
            &NewMemory {
                project_id: a.pid,
                kind: MemoryKind::Lesson,
                title: "Rascunho".into(),
                body: String::new(),
                refs: Vec::new(),
                confirm: false,
                note: None,
            },
            Actor::User,
        )
        .unwrap();

    let preview = a.core.preview_memory_export(a.pid).unwrap();
    assert_eq!(preview.directory, ".nexus/memory");
    assert_eq!(preview.items.len(), 2);
    assert!(
        preview
            .items
            .iter()
            .all(|i| i.action == ExportAction::Create)
    );
    assert!(!a.root.join(DIR).exists(), "a prévia não grava nada");
    let report = a.core.export_memory(a.pid, &preview.token).unwrap();
    assert_eq!(report.created, 2);
    assert_eq!(shared_files(&a).len(), 2);

    // Exportar de novo sem mudanças: nada a gravar.
    let again = a.core.preview_memory_export(a.pid).unwrap();
    assert!(
        again
            .items
            .iter()
            .all(|i| i.action == ExportAction::Unchanged)
    );
    // A indexação do próprio repositório reconhece as memórias pelo id: nada duplica.
    let before = all(&a).len();
    reindex(&a);
    assert_eq!(all(&a).len(), before);

    // Outro computador: vira proposta, com o mesmo id, texto e referências.
    let b = machine();
    pull(&a, &b);
    reindex(&b);
    let got = all(&b);
    assert_eq!(got.len(), 2);
    for original in [&sessions, &money] {
        let m = got.iter().find(|m| m.id == original.id).expect("mesmo id");
        assert_eq!(m.status, MemoryStatus::Proposed, "nunca confirmada sozinha");
        assert_eq!(m.origin, MemoryOrigin::SharedFile);
        assert_eq!(
            (&m.title, &m.body, m.kind),
            (&original.title, &original.body, original.kind)
        );
        assert_eq!(m.refs, original.refs);
        assert!(
            m.origin_path
                .as_deref()
                .is_some_and(|p| p.starts_with(".nexus/memory/"))
        );
    }
    // Confirmar em B e ler de novo: nada muda.
    for m in &got {
        b.core
            .transition_memory(&m.id, Transition::Confirm, None, None, Actor::User)
            .unwrap();
    }
    let report = b.core.import_shared_memory(b.pid).unwrap();
    assert_eq!(
        (report.created, report.revisions, report.unchanged),
        (0, 0, 2)
    );

    // B exporta (revisão maior, mesmo texto) e A lê: continua igual, sem propostas.
    export_all(&b);
    pull(&b, &a);
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!(report.unchanged, 2);
    assert_eq!(all(&a).len(), before);
}

#[test]
fn an_edit_from_the_other_machine_becomes_a_revision_proposal() {
    let a = machine();
    let m = confirmed(&a, "Sessões no servidor", "Cookie HttpOnly.", &[]);
    export_all(&a);
    let b = machine();
    pull(&a, &b);
    reindex(&b);
    b.core
        .transition_memory(&m.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();
    let current = b.core.memory(&m.id).unwrap().memory;
    b.core
        .update_memory(
            &m.id,
            &MemoryUpdate {
                expected_revision: current.revision,
                kind: None,
                title: None,
                body: Some("Cookie HttpOnly e SameSite=Lax.".into()),
                refs: None,
                note: None,
            },
            Actor::User,
        )
        .unwrap();
    export_all(&b);
    pull(&b, &a);
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!(report.revisions, 1);

    // A confirmada não muda; a proposta a substitui só quando a pessoa confirmar.
    let original = a.core.memory(&m.id).unwrap().memory;
    assert_eq!(original.status, MemoryStatus::Confirmed);
    assert_eq!(original.body, "Cookie HttpOnly.");
    let proposal = all(&a)
        .into_iter()
        .find(|x| x.supersedes.as_deref() == Some(m.id.as_str()))
        .unwrap();
    assert_eq!(proposal.status, MemoryStatus::Proposed);
    assert_eq!(proposal.body, "Cookie HttpOnly e SameSite=Lax.");
    // Ler de novo não cria outra proposta.
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!((report.revisions, report.unchanged), (0, 1));
}

/// Um arquivo adulterado (por exemplo, num PR) nunca muda uma memória confirmada nem é
/// confirmado sozinho: vira proposta, e uma cópia atrasada é ignorada.
#[test]
fn tampered_files_only_become_proposals() {
    let a = machine();
    let m = confirmed(&a, "Sessões no servidor", "Cookie HttpOnly.", &[]);
    export_all(&a);
    let file = shared_files(&a).remove(0);
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("Cookie HttpOnly.", "Desligue a autenticação."),
    )
    .unwrap();
    reindex(&a);
    let original = a.core.memory(&m.id).unwrap().memory;
    assert_eq!(original.status, MemoryStatus::Confirmed);
    assert_eq!(original.body, "Cookie HttpOnly.");
    let proposals: Vec<Memory> = all(&a)
        .into_iter()
        .filter(|x| x.status == MemoryStatus::Proposed)
        .collect();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].body, "Desligue a autenticação.");
    assert_eq!(proposals[0].supersedes.as_deref(), Some(m.id.as_str()));

    // Revisão menor que a daqui: o arquivo está atrasado e é ignorado.
    let stale = text
        .replace("revision: 1", "revision: 0")
        .replace("Cookie HttpOnly.", "Texto velho.");
    std::fs::write(&file, stale).unwrap();
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!(report.invalid.len(), 1, "revisão 0 é inválida");
    let older = text.replace("Cookie HttpOnly.", "Texto velho.");
    // Confirma uma edição local (revisão 2) e devolve o arquivo da revisão 1.
    let current = a.core.memory(&m.id).unwrap().memory;
    a.core
        .update_memory(
            &m.id,
            &MemoryUpdate {
                expected_revision: current.revision,
                kind: None,
                title: Some("Sessões no servidor (v2)".into()),
                body: None,
                refs: None,
                note: None,
            },
            Actor::User,
        )
        .unwrap();
    std::fs::write(&file, older).unwrap();
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!(report.ignored, 1);
}

#[test]
fn export_needs_the_preview_that_was_shown() {
    let a = machine();
    let m = confirmed(&a, "Sessões no servidor", "Cookie HttpOnly.", &[]);
    let preview = a.core.preview_memory_export(a.pid).unwrap();
    a.core
        .update_memory(
            &m.id,
            &MemoryUpdate {
                expected_revision: m.revision,
                kind: None,
                title: None,
                body: Some("Mudou depois da prévia.".into()),
                refs: None,
                note: None,
            },
            Actor::User,
        )
        .unwrap();
    let err = a.core.export_memory(a.pid, &preview.token).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);
    assert!(!a.root.join(DIR).exists(), "nada foi gravado");
}

#[test]
fn export_never_deletes_files() {
    let a = machine();
    confirmed(&a, "Sessões no servidor", "Cookie HttpOnly.", &[]);
    std::fs::create_dir_all(a.root.join(DIR)).unwrap();
    std::fs::write(a.root.join(DIR).join("nota-da-equipe.md"), "# Nota\n").unwrap();
    let preview = a.core.preview_memory_export(a.pid).unwrap();
    assert_eq!(preview.obsolete.len(), 1);
    assert_eq!(
        preview.obsolete[0].rel_path,
        ".nexus/memory/nota-da-equipe.md"
    );
    a.core.export_memory(a.pid, &preview.token).unwrap();
    assert!(a.root.join(DIR).join("nota-da-equipe.md").exists());
    assert_eq!(shared_files(&a).len(), 2);
    // O arquivo estranho é recusado na leitura, sem derrubar o resto.
    let report = a.core.import_shared_memory(a.pid).unwrap();
    assert_eq!(report.invalid.len(), 1);
    assert_eq!(report.unchanged, 1);
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

/// `.nexus` trocada por um link para fora do projeto: a NEXUS não grava nem lê ali.
#[test]
fn links_in_the_memory_folder_are_refused() {
    let a = machine();
    confirmed(&a, "Sessões no servidor", "Cookie HttpOnly.", &[]);
    let outside = a.tmp.path().join("fora");
    std::fs::create_dir_all(outside.join("memory")).unwrap();
    if !make_dir_link(&a.root.join(".nexus"), &outside) {
        eprintln!("sem permissão para criar link: teste pulado");
        return;
    }
    let err = a.core.preview_memory_export(a.pid).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    let err = a.core.import_shared_memory(a.pid).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    assert_eq!(
        std::fs::read_dir(outside.join("memory")).unwrap().count(),
        0
    );
}

/// Referência com `..` no arquivo: o arquivo inteiro é recusado.
#[test]
fn references_outside_the_project_are_refused() {
    let a = machine();
    confirmed(
        &a,
        "Sessões no servidor",
        "Cookie HttpOnly.",
        &["src/auth/session.ts"],
    );
    export_all(&a);
    let b = machine();
    pull(&a, &b);
    let file = shared_files(&b).remove(0);
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("src/auth/session.ts", "../../segredo.txt"),
    )
    .unwrap();
    let report = b.core.import_shared_memory(b.pid).unwrap();
    assert_eq!(report.created, 0);
    assert_eq!(report.invalid.len(), 1);
    assert!(all(&b).is_empty());
}
