//! Memória do projeto: estados, revisões, quem pode revisar, importação de ADRs e busca.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::memory::{
    Actor, MemoryKind, MemoryOrigin, MemoryQuery, MemoryRefInput, MemoryStatus, MemoryUpdate,
    NewMemory, RefState, RevisionChange, Transition,
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
    _tmp: tempfile::TempDir,
    root: PathBuf,
    core: Core,
    pid: i64,
}

fn setup() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saas-app"),
        &root,
    );
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(&root).unwrap().id;
    reindex(&core, pid);
    Env {
        _tmp: tmp,
        root,
        core,
        pid,
    }
}

fn reindex(core: &Core, pid: i64) {
    core.index_project(
        pid,
        &IndexOptions::new(IndexMode::Incremental, IndexTrigger::Cli),
        &JobControl::new(),
        &NoopObserver,
    )
    .unwrap();
}

fn new_memory(pid: i64, title: &str, body: &str) -> NewMemory {
    NewMemory {
        project_id: pid,
        kind: MemoryKind::Decision,
        title: title.into(),
        body: body.into(),
        refs: Vec::new(),
        confirm: false,
        note: None,
    }
}

fn file_ref(path: &str) -> MemoryRefInput {
    MemoryRefInput {
        rel_path: path.into(),
        start_line: None,
        end_line: None,
    }
}

fn adr_memories(env: &Env) -> Vec<nexus_core::memory::Memory> {
    let mut all = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            ..Default::default()
        })
        .unwrap();
    all.sort_by(|a, b| a.origin_path.cmp(&b.origin_path));
    all
}

#[test]
fn adr_import_creates_proposals_and_is_idempotent() {
    let env = setup();
    let report = env.core.import_adrs(env.pid).unwrap();
    assert_eq!(report.scanned, 3);
    assert_eq!(report.created, 3);
    let all = adr_memories(&env);
    assert_eq!(all.len(), 3);
    for m in &all {
        assert_eq!(m.status, MemoryStatus::Proposed, "importação só propõe");
        assert_eq!(m.origin, MemoryOrigin::AdrImport);
        assert_eq!(m.kind, MemoryKind::Decision);
        assert_eq!(m.refs.len(), 1);
        assert_eq!(m.refs[0].rel_path, m.origin_path.clone().unwrap());
    }
    let money = &all[1];
    assert_eq!(money.title, "ADR 0002 — Dinheiro em centavos inteiros");
    assert!(
        money
            .body
            .starts_with("Todo valor monetário é um inteiro em centavos")
    );
    let expected = blake3::hash(
        &std::fs::read(env.root.join("docs/decisions/0002-money-in-cents.md")).unwrap(),
    )
    .to_hex()
    .to_string();
    assert_eq!(money.origin_hash.as_deref(), Some(expected.as_str()));

    let again = env.core.import_adrs(env.pid).unwrap();
    assert_eq!((again.created, again.updated, again.unchanged), (0, 0, 3));
    assert_eq!(adr_memories(&env).len(), 3);
}

#[test]
fn only_the_user_confirms() {
    let env = setup();
    let m = env
        .core
        .create_memory(
            &new_memory(env.pid, "Sessões no servidor", "Sem JWT."),
            Actor::User,
        )
        .unwrap();
    assert_eq!(m.status, MemoryStatus::Proposed);
    for actor in [Actor::Agent, Actor::AdrImport, Actor::System] {
        let err = env
            .core
            .transition_memory(&m.id, Transition::Confirm, None, None, actor)
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::Conflict, "{actor:?} não confirma");
    }
    let c = env
        .core
        .transition_memory(
            &m.id,
            Transition::Confirm,
            Some(1),
            Some("revisado"),
            Actor::User,
        )
        .unwrap();
    assert_eq!(c.status, MemoryStatus::Confirmed);
    assert_eq!(c.revision, 2);
    assert!(c.confirmed_at.is_some());
    let detail = env.core.memory(&m.id).unwrap();
    let changes: Vec<RevisionChange> = detail.revisions.iter().map(|r| r.change).collect();
    assert_eq!(
        changes,
        [RevisionChange::Confirmed, RevisionChange::Created]
    );
    assert_eq!(detail.revisions[0].note.as_deref(), Some("revisado"));
    assert_eq!(detail.revisions[0].actor, Actor::User);

    // Transições inválidas.
    let err = env
        .core
        .transition_memory(&m.id, Transition::Confirm, None, None, Actor::User)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    let err = env
        .core
        .transition_memory(&m.id, Transition::Restore, None, None, Actor::User)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
}

#[test]
fn agents_only_propose_and_never_edit_confirmed() {
    let env = setup();
    let mut req = new_memory(env.pid, "Cache local de sessões", "Guardar por 5 minutos.");
    req.confirm = true;
    let err = env.core.create_memory(&req, Actor::Agent).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);

    req.confirm = false;
    let p = env.core.create_memory(&req, Actor::Agent).unwrap();
    assert_eq!(p.origin, MemoryOrigin::Agent);
    assert_eq!(p.status, MemoryStatus::Proposed);
    // O agente pode ajustar a própria proposta…
    let p = env
        .core
        .update_memory(
            &p.id,
            &MemoryUpdate {
                expected_revision: 1,
                body: Some("Guardar por 10 minutos.".into()),
                kind: None,
                title: None,
                refs: None,
                note: None,
            },
            Actor::Agent,
        )
        .unwrap();
    assert_eq!(p.revision, 2);
    let c = env
        .core
        .transition_memory(&p.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();

    // …mas não a memória confirmada.
    let edit = MemoryUpdate {
        expected_revision: c.revision,
        body: Some("Sem cache.".into()),
        kind: None,
        title: None,
        refs: None,
        note: None,
    };
    let err = env
        .core
        .update_memory(&c.id, &edit, Actor::Agent)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);
    let err = env
        .core
        .transition_memory(&c.id, Transition::Archive, None, None, Actor::Agent)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);

    // O usuário edita: continua confirmada e o texto anterior fica no histórico.
    let e = env.core.update_memory(&c.id, &edit, Actor::User).unwrap();
    assert_eq!(e.status, MemoryStatus::Confirmed);
    assert_eq!(e.body, "Sem cache.");
    let detail = env.core.memory(&c.id).unwrap();
    assert_eq!(detail.revisions.len(), 4);
    assert_eq!(detail.revisions[1].body, "Guardar por 10 minutos.");
    assert_eq!(detail.revisions[3].body, "Guardar por 5 minutos.");
}

#[test]
fn stale_revision_is_a_conflict() {
    let env = setup();
    let m = env
        .core
        .create_memory(&new_memory(env.pid, "Logs estruturados", ""), Actor::User)
        .unwrap();
    let update = |rev: i64, title: &str| MemoryUpdate {
        expected_revision: rev,
        title: Some(title.into()),
        kind: None,
        body: None,
        refs: None,
        note: None,
    };
    env.core
        .update_memory(&m.id, &update(1, "Logs em JSON"), Actor::User)
        .unwrap();
    let err = env
        .core
        .update_memory(&m.id, &update(1, "Logs em texto"), Actor::User)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);
    assert_eq!(env.core.memory(&m.id).unwrap().memory.title, "Logs em JSON");
    // Sem mudança real: não cria revisão.
    let same = env
        .core
        .update_memory(&m.id, &update(2, "Logs em JSON"), Actor::User)
        .unwrap();
    assert_eq!(same.revision, 2);
}

#[test]
fn archived_memories_are_read_only_until_restored() {
    let env = setup();
    let m = env
        .core
        .create_memory(
            &new_memory(env.pid, "Usar Redis para filas", ""),
            Actor::User,
        )
        .unwrap();
    let a = env
        .core
        .transition_memory(&m.id, Transition::Archive, None, None, Actor::User)
        .unwrap();
    assert_eq!(a.status, MemoryStatus::Archived);
    let err = env
        .core
        .update_memory(
            &m.id,
            &MemoryUpdate {
                expected_revision: a.revision,
                title: Some("Outro".into()),
                kind: None,
                body: None,
                refs: None,
                note: None,
            },
            Actor::User,
        )
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    let r = env
        .core
        .transition_memory(&m.id, Transition::Restore, None, None, Actor::User)
        .unwrap();
    assert_eq!(r.status, MemoryStatus::Proposed);
}

#[test]
fn references_are_validated_against_the_index() {
    let env = setup();
    let mut req = new_memory(env.pid, "Sessão expira em 30 dias", "");
    req.refs = vec![MemoryRefInput {
        rel_path: ".\\src\\auth\\session.ts".into(),
        start_line: Some(10),
        end_line: Some(20),
    }];
    let m = env.core.create_memory(&req, Actor::User).unwrap();
    assert_eq!(m.refs[0].rel_path, "src/auth/session.ts");
    let hash = blake3::hash(&std::fs::read(env.root.join("src/auth/session.ts")).unwrap())
        .to_hex()
        .to_string();
    assert_eq!(m.refs[0].content_hash, hash);
    let detail = env.core.memory(&m.id).unwrap();
    assert_eq!(detail.ref_checks[0].state, RefState::Unchanged);

    // Arquivo sensível: existe no índice como ignorado, sem hash — não pode ser referenciado.
    std::fs::write(env.root.join(".env.production"), "API_KEY=x\n").unwrap();
    reindex(&env.core, env.pid);
    for bad in ["../fora.ts", "src/nao-existe.ts", ".env.production"] {
        req.refs = vec![file_ref(bad)];
        let err = env.core.create_memory(&req, Actor::User).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidInput, "{bad}");
    }
    req.refs = vec![MemoryRefInput {
        rel_path: "src/auth/session.ts".into(),
        start_line: Some(9),
        end_line: Some(3),
    }];
    assert_eq!(
        env.core
            .create_memory(&req, Actor::User)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidInput
    );
}

#[test]
fn search_filters_and_counts() {
    let env = setup();
    env.core.import_adrs(env.pid).unwrap();
    let mut conv = new_memory(env.pid, "Exportações nomeadas", "Nada de export default.");
    conv.kind = MemoryKind::Convention;
    conv.confirm = true;
    env.core.create_memory(&conv, Actor::User).unwrap();

    let hits = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            text: Some("centavos".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(hits[0].title, "ADR 0002 — Dinheiro em centavos inteiros");
    let confirmed = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            statuses: Some(vec![MemoryStatus::Confirmed]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(confirmed.len(), 1);
    let conventions = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            kinds: Some(vec![MemoryKind::Convention]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(conventions.len(), 1);
    let counts = env.core.memory_counts(env.pid).unwrap();
    assert_eq!((counts.proposed, counts.confirmed), (3, 1));
    // Consulta só com palavras genéricas: nenhum termo útil, nenhum resultado (sem erro).
    assert!(
        env.core
            .list_memories(&MemoryQuery {
                project_id: env.pid,
                text: Some("de o a".into()),
                ..Default::default()
            })
            .unwrap()
            .is_empty()
    );
}

fn replace_in(root: &Path, rel: &str, from: &str, to: &str) {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{from} em {rel}");
    std::fs::write(&path, text.replace(from, to)).unwrap();
}

#[test]
fn changed_adr_never_overwrites_a_confirmed_decision() {
    let env = setup();
    env.core.import_adrs(env.pid).unwrap();
    let money = adr_memories(&env).remove(1);
    let confirmed = env
        .core
        .transition_memory(&money.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();

    replace_in(
        &env.root,
        "docs/decisions/0002-money-in-cents.md",
        "Todo valor monetário é um inteiro em centavos.",
        "Todo valor monetário usa Decimal com 4 casas.",
    );
    reindex(&env.core, env.pid);
    let report = env.core.import_adrs(env.pid).unwrap();
    assert_eq!(report.superseding, 1);
    assert_eq!(report.created, 0);

    let still = env.core.memory(&money.id).unwrap().memory;
    assert_eq!(still.status, MemoryStatus::Confirmed);
    assert_eq!(still.body, confirmed.body, "a confirmada não muda");
    let proposal = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            statuses: Some(vec![MemoryStatus::Proposed]),
            text: Some("Decimal".into()),
            ..Default::default()
        })
        .unwrap()
        .remove(0);
    assert_eq!(proposal.supersedes.as_deref(), Some(money.id.as_str()));

    // Reimportar sem nova mudança não cria outra proposta.
    assert_eq!(env.core.import_adrs(env.pid).unwrap().unchanged, 3);

    let done = env
        .core
        .transition_memory(&proposal.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();
    assert_eq!(done.status, MemoryStatus::Confirmed);
    let old = env.core.memory(&money.id).unwrap();
    assert_eq!(old.memory.status, MemoryStatus::Superseded);
    assert_eq!(
        old.memory.superseded_by.as_deref(),
        Some(proposal.id.as_str())
    );
    assert_eq!(old.revisions[0].change, RevisionChange::Superseded);
    assert!(old.superseded_by_title.unwrap().contains("Dinheiro"));
}

#[test]
fn proposal_follows_its_document_until_reviewed() {
    let env = setup();
    env.core.import_adrs(env.pid).unwrap();
    replace_in(
        &env.root,
        "docs/decisions/0002-money-in-cents.md",
        "apenas em `formatCents`",
        "apenas em `formatMoney`",
    );
    reindex(&env.core, env.pid);
    let r = env.core.import_adrs(env.pid).unwrap();
    assert_eq!((r.updated, r.created, r.superseding), (1, 0, 0));
    let m = adr_memories(&env).remove(1);
    assert!(m.body.contains("formatMoney"));
    assert_eq!(m.revision, 2);
    assert_eq!(m.status, MemoryStatus::Proposed);
}

#[test]
fn metadata_only_changes_do_not_create_proposals() {
    let env = setup();
    env.core.import_adrs(env.pid).unwrap();
    let money = adr_memories(&env).remove(1);
    env.core
        .transition_memory(&money.id, Transition::Confirm, None, None, Actor::User)
        .unwrap();
    replace_in(
        &env.root,
        "docs/decisions/0002-money-in-cents.md",
        "## Contexto",
        "## Contexto e histórico",
    );
    reindex(&env.core, env.pid);
    let r = env.core.import_adrs(env.pid).unwrap();
    assert_eq!((r.created, r.superseding, r.unchanged), (0, 0, 3));
    let d = env.core.memory(&money.id).unwrap();
    assert_eq!(d.memory.status, MemoryStatus::Confirmed);
    assert_eq!(d.revisions[0].change, RevisionChange::RefsReviewed);
    assert_eq!(d.revisions[0].actor, Actor::System);
    assert_eq!(d.ref_checks[0].state, RefState::Unchanged);
}

#[test]
fn inactive_adrs_are_not_proposed() {
    let env = setup();
    replace_in(
        &env.root,
        "docs/decisions/0003-oauth-providers.md",
        "Estado: aceita",
        "Estado: substituída pela ADR 0004",
    );
    reindex(&env.core, env.pid);
    let r = env.core.import_adrs(env.pid).unwrap();
    assert_eq!((r.created, r.skipped_inactive), (2, 1));
}

#[test]
fn removing_the_project_removes_its_memory() {
    let env = setup();
    env.core.import_adrs(env.pid).unwrap();
    env.core.remove_project(env.pid).unwrap();
    let again = env.core.add_project(&env.root).unwrap();
    assert_eq!(env.core.memory_counts(again.id).unwrap().total(), 0);
}
