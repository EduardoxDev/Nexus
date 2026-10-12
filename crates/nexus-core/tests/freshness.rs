//! Atualidade: caminhos e símbolos citados em documentos, referências de memória, contradições,
//! arquivos alterados desde a indexação e ciclo de vida dos alertas.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use nexus_core::context::ContextPackRequest;
use nexus_core::context::export::{ExportFormat, ExportOptions};
use nexus_core::freshness::{FreshnessIssue, IssueKind, IssueQuery, IssueStatus, Severity};
use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::memory::{
    Actor, MemoryKind, MemoryQuery, MemoryRefInput, MemoryUpdate, NewMemory, Transition,
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

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    core: Core,
    pid: i64,
}

fn setup_from(src: &Path, prepare: impl FnOnce(&Path)) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    copy_dir(src, &root);
    prepare(&root);
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

fn setup(prepare: impl FnOnce(&Path)) -> Env {
    setup_from(&fixture("saas-app"), prepare)
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

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn replace_in(root: &Path, rel: &str, from: &str, to: &str) {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{from} em {rel}");
    std::fs::write(&path, text.replace(from, to)).unwrap();
}

fn open_issues(env: &Env) -> Vec<FreshnessIssue> {
    env.core
        .freshness_issues(&IssueQuery {
            project_id: env.pid,
            ..Default::default()
        })
        .unwrap()
}

fn of_kind(issues: &[FreshnessIssue], kind: IssueKind) -> Vec<&FreshnessIssue> {
    issues.iter().filter(|i| i.kind == kind).collect()
}

fn confirmed(env: &Env, title: &str, body: &str, refs: &[&str]) -> String {
    let m = env
        .core
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
                        start_line: None,
                        end_line: None,
                    })
                    .collect(),
                confirm: true,
                note: None,
            },
            Actor::User,
        )
        .unwrap();
    m.id
}

#[test]
fn fixtures_have_no_document_alerts() {
    // Os documentos das fixtures citam apenas caminhos e nomes que existem: qualquer alerta
    // aqui é falso positivo da heurística.
    for name in ["saas-app", "ts-webapp", "py-service", "rust-lib"] {
        let env = setup_from(&fixture(name), |_| {});
        let issues = open_issues(&env);
        let docs: Vec<String> = issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::MissingPath | IssueKind::MissingSymbol))
            .map(|i| i.title.clone())
            .collect();
        assert!(docs.is_empty(), "{name}: {docs:#?}");
        let summary = env.core.freshness_summary(env.pid).unwrap();
        let scan = summary
            .last_scan
            .expect("verificação automática após indexar");
        assert!(scan.docs_checked > 0, "{name}");
        assert!(!summary.scan_outdated);
    }
}

#[test]
fn missing_paths_are_reported_with_line_reason_and_suggestion() {
    let env = setup(|root| {
        write(
            root,
            "docs/guide.md",
            "# Guia\n\n\
             Veja [a sessão](../src/auth/old-session.ts) e [o ADR](decisions/0001-sessions-not-jwt.md).\n\
             Cobrança em `src/billing/stripeClient.ts`; cupons em `src/checkout/coupons.ts`.\n\
             Frete: [shipping](../src/shipping.ts).\n\
             ```\nexemplo: `src/inventado/exemplo.ts` e [x](nao/existe.md)\n```\n\
             Gerado em `target/release/nexus.exe`. Site: [docs](https://example.com/a.md).\n\
             Relativo ao subprojeto: `checkout/cart.ts`; pasta de execução: `logs/`.\n",
        );
    });
    let issues = open_issues(&env);
    let missing = of_kind(&issues, IssueKind::MissingPath);
    let cited: Vec<&str> = missing
        .iter()
        .map(|i| i.sources[0].cited.as_deref().unwrap())
        .collect();
    assert_eq!(missing.len(), 3, "{cited:?}");
    let old = missing
        .iter()
        .find(|i| i.sources[0].cited.as_deref() == Some("../src/auth/old-session.ts"))
        .unwrap();
    assert_eq!(old.sources[0].rel_path, "docs/guide.md");
    assert_eq!(old.sources[0].line, Some(3));
    assert_eq!(old.severity, Severity::Warning);
    assert!(!old.needs_review, "link quebrado é inequívoco");
    assert!(old.reason.contains("linha 3"));
    let coupons = missing
        .iter()
        .find(|i| i.sources[0].cited.as_deref() == Some("src/checkout/coupons.ts"))
        .unwrap();
    assert!(coupons.needs_review, "código inline pode ser exemplo");
    assert_eq!(coupons.severity, Severity::Info);
    // Mesmo nome de arquivo em outro lugar ⇒ sugestão de caminho.
    let shipping = missing
        .iter()
        .find(|i| i.sources[0].cited.as_deref() == Some("../src/shipping.ts"))
        .unwrap();
    assert_eq!(
        shipping.suggestion.as_deref(),
        Some("src/checkout/shipping.ts")
    );
    assert!(shipping.recommendation.contains("src/checkout/shipping.ts"));

    // Corrigir o documento resolve o alerta na próxima indexação (verificação automática).
    replace_in(
        &env.root,
        "docs/guide.md",
        "../src/shipping.ts",
        "../src/checkout/shipping.ts",
    );
    reindex(&env.core, env.pid);
    let after = open_issues(&env);
    assert_eq!(of_kind(&after, IssueKind::MissingPath).len(), 2);
    let all = env
        .core
        .freshness_issues(&IssueQuery {
            project_id: env.pid,
            statuses: Some(vec![IssueStatus::Resolved]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].resolved_at.is_some());
}

#[test]
fn removed_symbols_differ_from_never_seen_ones() {
    let env = setup(|root| {
        write(
            root,
            "src/checkout/quote.ts",
            "export function computeShippingQuote(cents: number) {\n  return cents;\n}\n",
        );
        write(
            root,
            "docs/quote.md",
            "# Frete\n\nUse `computeShippingQuote` (não `legacyQuoteV1`). Sessões: `createSession`.\n",
        );
    });
    // `legacyQuoteV1` nunca existiu no código (poderia ser uma API externa): sem alerta.
    assert!(of_kind(&open_issues(&env), IssueKind::MissingSymbol).is_empty());

    std::fs::remove_file(env.root.join("src/checkout/quote.ts")).unwrap();
    reindex(&env.core, env.pid);
    let issues = open_issues(&env);
    let symbols = of_kind(&issues, IssueKind::MissingSymbol);
    assert_eq!(symbols.len(), 1, "{symbols:#?}");
    let removed = symbols[0];
    assert!(removed.title.contains("computeShippingQuote"));
    assert!(removed.title.contains("removido"));
    assert_eq!(removed.severity, Severity::Warning);
    assert!(!removed.needs_review);
    assert_eq!(removed.sources[0].rel_path, "docs/quote.md");
    assert_eq!(removed.sources[0].line, Some(3));

    // Continua removido nas verificações seguintes, até a documentação mudar.
    env.core.scan_freshness(env.pid).unwrap();
    assert_eq!(
        of_kind(&open_issues(&env), IssueKind::MissingSymbol).len(),
        1
    );
    replace_in(
        &env.root,
        "docs/quote.md",
        "Use `computeShippingQuote`",
        "Use `quoteShipping`",
    );
    reindex(&env.core, env.pid);
    assert!(of_kind(&open_issues(&env), IssueKind::MissingSymbol).is_empty());
}

#[test]
fn memory_refs_track_changes_moves_and_deletions() {
    let env = setup(|_| {});
    let id = confirmed(
        &env,
        "Regras de dinheiro e sessão",
        "Centavos inteiros; sessão no servidor.",
        &[
            "src/auth/session.ts",
            "src/utils/money.ts",
            "src/checkout/discounts.ts",
        ],
    );
    assert!(of_kind(&open_issues(&env), IssueKind::MemoryRefChanged).is_empty());

    replace_in(
        &env.root,
        "src/auth/session.ts",
        "export async function createSession",
        "// sessão revisada\nexport async function createSession",
    );
    std::fs::create_dir_all(env.root.join("src/lib")).unwrap();
    std::fs::rename(
        env.root.join("src/utils/money.ts"),
        env.root.join("src/lib/money.ts"),
    )
    .unwrap();
    std::fs::remove_file(env.root.join("src/checkout/discounts.ts")).unwrap();
    reindex(&env.core, env.pid);

    let issues = open_issues(&env);
    let changed = of_kind(&issues, IssueKind::MemoryRefChanged);
    assert_eq!(changed.len(), 1);
    assert_eq!(
        changed[0].subject_path.as_deref(),
        Some("src/auth/session.ts")
    );
    assert_eq!(changed[0].memory_id.as_deref(), Some(id.as_str()));
    assert!(changed[0].needs_review);
    let missing = of_kind(&issues, IssueKind::MemoryRefMissing);
    assert_eq!(missing.len(), 2);
    let moved = missing
        .iter()
        .find(|i| i.subject_path.as_deref() == Some("src/utils/money.ts"))
        .unwrap();
    assert_eq!(moved.suggestion.as_deref(), Some("src/lib/money.ts"));
    assert!(!moved.needs_review, "o destino é conhecido");
    let gone = missing
        .iter()
        .find(|i| i.subject_path.as_deref() == Some("src/checkout/discounts.ts"))
        .unwrap();
    assert!(gone.needs_review);

    // O usuário revisa: hash atualizado e caminho movido corrigido; o arquivo apagado continua.
    let detail = env.core.memory(&id).unwrap();
    let reviewed = env
        .core
        .review_memory_refs(&id, detail.memory.revision, Actor::User)
        .unwrap();
    let paths: Vec<&str> = reviewed.refs.iter().map(|r| r.rel_path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "src/auth/session.ts",
            "src/lib/money.ts",
            "src/checkout/discounts.ts"
        ]
    );
    let issues = open_issues(&env);
    assert!(of_kind(&issues, IssueKind::MemoryRefChanged).is_empty());
    let missing = of_kind(&issues, IssueKind::MemoryRefMissing);
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing[0].subject_path.as_deref(),
        Some("src/checkout/discounts.ts")
    );

    // Agentes não revisam memórias confirmadas.
    let err = env
        .core
        .review_memory_refs(&id, reviewed.revision, Actor::Agent)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Conflict);
}

#[test]
fn duplicates_and_contradictions_between_memories() {
    let env = setup(|_| {});
    let a = confirmed(
        &env,
        "Tempo de expiração da sessão",
        "Sessões expiram em 30 dias.",
        &[],
    );
    let b = confirmed(
        &env,
        "Tempo de expiração da sessão",
        "Sessões expiram após 2 horas de inatividade, renovadas a cada requisição.",
        &[],
    );
    let issues = open_issues(&env);
    let dup = of_kind(&issues, IssueKind::DuplicateMemory);
    assert_eq!(dup.len(), 1);
    assert!(dup[0].contradiction, "mesmo assunto, textos diferentes");
    assert_eq!(dup[0].severity, Severity::Warning);
    let pair = [
        dup[0].memory_id.clone().unwrap(),
        dup[0].related_memory_id.clone().unwrap(),
    ];
    assert!(pair.contains(&a) && pair.contains(&b));

    // Duplicata simples (proposta parecida): informação, não contradição.
    env.core
        .create_memory(
            &NewMemory {
                project_id: env.pid,
                kind: MemoryKind::Convention,
                title: "Exportações nomeadas sempre".into(),
                body: "Use exportações nomeadas; nada de export default.".into(),
                refs: Vec::new(),
                confirm: false,
                note: None,
            },
            Actor::Agent,
        )
        .unwrap();
    env.core
        .create_memory(
            &NewMemory {
                project_id: env.pid,
                kind: MemoryKind::Convention,
                title: "Sempre exportações nomeadas".into(),
                body: "Nada de export default; use exportações nomeadas.".into(),
                refs: Vec::new(),
                confirm: false,
                note: None,
            },
            Actor::Agent,
        )
        .unwrap();
    let issues = open_issues(&env);
    let dups = of_kind(&issues, IssueKind::DuplicateMemory);
    assert_eq!(dups.len(), 2);
    let plain = dups.iter().find(|i| !i.contradiction).unwrap();
    assert_eq!(plain.severity, Severity::Info);

    // Arquivar uma das contraditórias resolve o alerta (reavaliação automática da memória).
    env.core
        .transition_memory(&b, Transition::Archive, None, None, Actor::User)
        .unwrap();
    let issues = open_issues(&env);
    assert!(
        of_kind(&issues, IssueKind::DuplicateMemory)
            .iter()
            .all(|i| !i.contradiction)
    );
}

#[test]
fn imported_decisions_are_checked_against_their_source() {
    let env = setup(|_| {});
    env.core.import_adrs(env.pid).unwrap();
    let all = env
        .core
        .list_memories(&MemoryQuery {
            project_id: env.pid,
            ..Default::default()
        })
        .unwrap();
    let oauth = all
        .iter()
        .find(|m| m.origin_path.as_deref() == Some("docs/decisions/0003-oauth-providers.md"))
        .unwrap();
    let money = all
        .iter()
        .find(|m| m.origin_path.as_deref() == Some("docs/decisions/0002-money-in-cents.md"))
        .unwrap();
    for m in [oauth, money] {
        env.core
            .transition_memory(&m.id, Transition::Confirm, None, None, Actor::User)
            .unwrap();
    }

    replace_in(
        &env.root,
        "docs/decisions/0003-oauth-providers.md",
        "Estado: aceita",
        "Estado: substituída pela ADR 0004",
    );
    replace_in(
        &env.root,
        "docs/decisions/0002-money-in-cents.md",
        "Todo valor monetário é um inteiro em centavos.",
        "Todo valor monetário usa Decimal com 4 casas.",
    );
    reindex(&env.core, env.pid);
    let issues = open_issues(&env);
    let superseded = of_kind(&issues, IssueKind::SupersededSource);
    assert_eq!(superseded.len(), 1);
    assert!(superseded[0].contradiction);
    assert_eq!(superseded[0].memory_id.as_deref(), Some(oauth.id.as_str()));
    assert!(superseded[0].title.contains("substituída pela ADR 0004"));
    // Só o estado mudou na 0003: não é "texto diferente".
    let changed = of_kind(&issues, IssueKind::MemorySourceChanged);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].memory_id.as_deref(), Some(money.id.as_str()));
    assert!(changed[0].contradiction);
    assert!(changed[0].related_memory_id.is_none());
    // Mudanças no documento de origem não aparecem também como "arquivo alterado".
    assert!(of_kind(&issues, IssueKind::MemoryRefChanged).is_empty());

    // Reimportar cria a proposta de atualização; o alerta passa a apontar para ela.
    env.core.import_adrs(env.pid).unwrap();
    let issues = open_issues(&env);
    let changed = of_kind(&issues, IssueKind::MemorySourceChanged);
    assert!(changed[0].related_memory_id.is_some());
    assert!(changed[0].recommendation.contains("proposta"));
}

#[test]
fn dismissed_alerts_stay_dismissed_until_fixed() {
    let env = setup(|root| {
        write(
            root,
            "docs/notes.md",
            "# Notas\n\nVer `src/legacy/cart.ts`.\n",
        );
    });
    let issue = of_kind(&open_issues(&env), IssueKind::MissingPath)[0].clone();
    let d = env
        .core
        .set_issue_dismissed(issue.id, true, Some("é só um exemplo"))
        .unwrap();
    assert_eq!(d.status, IssueStatus::Dismissed);
    assert_eq!(d.dismiss_note.as_deref(), Some("é só um exemplo"));
    env.core.scan_freshness(env.pid).unwrap();
    assert!(of_kind(&open_issues(&env), IssueKind::MissingPath).is_empty());
    let dismissed = env
        .core
        .freshness_issues(&IssueQuery {
            project_id: env.pid,
            statuses: Some(vec![IssueStatus::Dismissed]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(dismissed.len(), 1);
    assert_eq!(dismissed[0].id, issue.id, "mesmo alerta, não um novo");
    let summary = env.core.freshness_summary(env.pid).unwrap();
    assert_eq!(summary.dismissed, 1);

    // Reabrir só vale para dispensados; dispensar duas vezes é inválido.
    let reopened = env.core.set_issue_dismissed(issue.id, false, None).unwrap();
    assert_eq!(reopened.status, IssueStatus::Open);
    assert_eq!(
        env.core
            .set_issue_dismissed(issue.id, false, None)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidInput
    );
    env.core.set_issue_dismissed(issue.id, true, None).unwrap();

    replace_in(
        &env.root,
        "docs/notes.md",
        "src/legacy/cart.ts",
        "src/checkout/cart.ts",
    );
    reindex(&env.core, env.pid);
    let resolved = env
        .core
        .freshness_issues(&IssueQuery {
            project_id: env.pid,
            statuses: Some(vec![IssueStatus::Resolved]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(resolved.len(), 1);
}

#[test]
fn files_changed_since_index_are_reported_until_reindexed() {
    let env = setup(|_| {});
    replace_in(&env.root, "src/utils/money.ts", "BRL", "USD");
    std::fs::remove_file(env.root.join("src/utils/logger.ts")).unwrap();
    let report = env.core.scan_freshness(env.pid).unwrap();
    assert!(report.new_issues >= 1);
    let issues = open_issues(&env);
    let changed = of_kind(&issues, IssueKind::FilesChanged);
    assert_eq!(changed.len(), 1);
    let cited: Vec<(&str, &str)> = changed[0]
        .sources
        .iter()
        .map(|s| (s.rel_path.as_str(), s.cited.as_deref().unwrap()))
        .collect();
    assert_eq!(
        cited,
        [
            ("src/utils/logger.ts", "apagado"),
            ("src/utils/money.ts", "alterado")
        ]
    );
    assert!(!changed[0].needs_review);

    reindex(&env.core, env.pid);
    assert!(of_kind(&open_issues(&env), IssueKind::FilesChanged).is_empty());
}

#[test]
fn packs_include_confirmed_memories_and_related_alerts() {
    let env = setup(|_| {});
    let session = confirmed(
        &env,
        "Expiração de sessão em 30 dias",
        "Sessões expiram em 30 dias; renovar no login.",
        &["src/auth/session.ts"],
    );
    // Proposta relevante: não deve entrar.
    env.core
        .create_memory(
            &NewMemory {
                project_id: env.pid,
                kind: MemoryKind::Decision,
                title: "Expiração de sessão em 1 hora".into(),
                body: "Proposta de um agente.".into(),
                refs: Vec::new(),
                confirm: false,
                note: None,
            },
            Actor::Agent,
        )
        .unwrap();
    replace_in(
        &env.root,
        "src/auth/session.ts",
        "export async function createSession",
        "// mudou\nexport async function createSession",
    );
    reindex(&env.core, env.pid);

    let pack = env
        .core
        .compile_context_pack(&ContextPackRequest {
            project_id: env.pid,
            task: "alterar a expiração da sessão".into(),
            token_budget: None,
            max_files: None,
        })
        .unwrap();
    let ids: Vec<&str> = pack
        .plan
        .memories
        .iter()
        .map(|m| m.memory_id.as_str())
        .collect();
    assert_eq!(ids, [session.as_str()], "só a confirmada");
    assert!(pack.memories[0].current.is_some());
    assert!(
        pack.issues
            .iter()
            .any(|i| i.kind == IssueKind::MemoryRefChanged),
        "{:#?}",
        pack.issues
    );
    assert!(pack.plan.summary.contains("memória(s) confirmada(s)"));

    let md = env
        .core
        .export_context_pack(
            &pack.plan.id,
            ExportFormat::Markdown,
            &ExportOptions::default(),
        )
        .unwrap();
    assert!(md.contains("## Memória do projeto (confirmada pelo usuário)"));
    assert!(md.contains("Expiração de sessão em 30 dias"));
    assert!(!md.contains("Expiração de sessão em 1 hora"));
    assert!(md.contains("## Contradições e alertas pendentes"));
    assert!(md.contains("requer revisão humana"));
    let without = env
        .core
        .export_context_pack(
            &pack.plan.id,
            ExportFormat::Json,
            &ExportOptions {
                excluded_memories: Some(vec![session.clone()]),
                ..Default::default()
            },
        )
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&without).unwrap();
    assert_eq!(v["memories"].as_array().unwrap().len(), 0);
    assert_eq!(v["plan"]["memories"].as_array().unwrap().len(), 0);

    // Editar a memória depois: o pack reaberto mostra o texto atual e avisa.
    let rev = env.core.memory(&session).unwrap().memory.revision;
    env.core
        .update_memory(
            &session,
            &MemoryUpdate {
                expected_revision: rev,
                body: Some("Sessões expiram em 7 dias.".into()),
                kind: None,
                title: None,
                refs: None,
                note: None,
            },
            Actor::User,
        )
        .unwrap();
    let reopened = env.core.context_pack(&pack.plan.id).unwrap();
    assert_eq!(
        reopened.memories[0].state,
        nexus_core::context::PackMemoryState::Revised
    );
    let md = env
        .core
        .export_context_pack(
            &pack.plan.id,
            ExportFormat::Markdown,
            &ExportOptions::default(),
        )
        .unwrap();
    assert!(md.contains("Sessões expiram em 7 dias."));
    assert!(md.contains("EDITADA depois da criação do pack"));
}

#[test]
fn v1_pack_plans_still_load() {
    let env = setup(|_| {});
    let pack = env
        .core
        .compile_context_pack(&ContextPackRequest {
            project_id: env.pid,
            task: "checkout com cupom".into(),
            token_budget: None,
            max_files: None,
        })
        .unwrap();
    let mut v = serde_json::to_value(&pack.plan).unwrap();
    v.as_object_mut().unwrap().remove("memories");
    v["schemaVersion"] = 1.into();
    let plan: nexus_core::context::ContextPackPlan = serde_json::from_value(v).unwrap();
    assert!(plan.memories.is_empty());
}

/// Relatório de alertas para um repositório real: mede falsos positivos da heurística.
/// `NEXUS_FRESHNESS_REPO=C:\caminho cargo test -p nexus-core --test freshness -- --ignored --nocapture`
#[test]
#[ignore = "manual: requer NEXUS_FRESHNESS_REPO"]
fn report_for_real_repository() {
    let Some(repo) = std::env::var_os("NEXUS_FRESHNESS_REPO") else {
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let core = Core::open(DataPaths::at(tmp.path().join("data"))).unwrap();
    let pid = core.add_project(Path::new(&repo)).unwrap().id;
    reindex(&core, pid);
    let summary = core.freshness_summary(pid).unwrap();
    println!("{summary:#?}");
    for i in core
        .freshness_issues(&IssueQuery {
            project_id: pid,
            limit: Some(1000),
            ..Default::default()
        })
        .unwrap()
    {
        let src = i
            .sources
            .first()
            .map(|s| format!("{}:{}", s.rel_path, s.line.unwrap_or(0)))
            .unwrap_or_default();
        println!("[{:?}] {} — {src}", i.kind, i.title);
    }
}
