//! Registro de projetos: validação de pastas e remoção segura (nunca toca arquivos).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use nexus_core::index::{IndexMode, IndexOptions, IndexTrigger, JobControl, NoopObserver};
use nexus_core::{Core, DataPaths, ErrorCode};

fn core(tmp: &tempfile::TempDir) -> Core {
    Core::open(DataPaths::at(tmp.path().join("data"))).unwrap()
}

#[test]
fn rejects_invalid_or_dangerous_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core(&tmp);
    let file = tmp.path().join("arquivo.txt");
    std::fs::write(&file, "x").unwrap();

    let cases = [
        tmp.path().join("nao-existe"),
        file,
        tmp.path().join("data"), // diretório de dados do NEXUS
        std::path::PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }),
    ];
    for path in cases {
        let err = core.add_project(&path).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidInput, "{}", path.display());
    }
    if let Some(home) = dirs_home() {
        assert_eq!(
            core.add_project(&home).unwrap_err().code(),
            ErrorCode::InvalidInput
        );
    }
}

/// A pasta de dados chega por um caminho não canônico (`..`, nome curto 8.3 do Windows, como
/// `RUNNER~1` no CI): a proteção precisa comparar o caminho real, não o texto.
#[test]
fn rejects_data_dir_given_by_a_non_canonical_path() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("desvio").join("..").join("data");
    let core = Core::open(DataPaths::at(&data)).unwrap();
    let err = core.add_project(&tmp.path().join("data")).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    assert_eq!(
        core.add_project(tmp.path()).unwrap_err().code(),
        ErrorCode::InvalidInput
    );
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(Into::into)
}

#[test]
fn rejects_duplicates_and_overlaps() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core(&tmp);
    let work = tmp.path().join("work");
    let repo = work.join("repo");
    std::fs::create_dir_all(repo.join("sub")).unwrap();
    let project = core.add_project(&repo).unwrap();
    assert_eq!(project.name, "repo");

    assert_eq!(
        core.add_project(&repo).unwrap_err().code(),
        ErrorCode::Conflict
    );
    assert_eq!(
        core.add_project(&repo.join("sub")).unwrap_err().code(),
        ErrorCode::Conflict
    );
    assert_eq!(
        core.add_project(&work).unwrap_err().code(),
        ErrorCode::Conflict
    );
    // A pasta que contém os dados do NEXUS é recusada antes mesmo da sobreposição.
    assert_eq!(
        core.add_project(tmp.path()).unwrap_err().code(),
        ErrorCode::InvalidInput
    );
}

#[test]
fn find_by_id_name_or_path() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core(&tmp);
    let repo = tmp.path().join("meu-repo");
    std::fs::create_dir_all(&repo).unwrap();
    let id = core.add_project(&repo).unwrap().id;
    assert_eq!(core.find_project(&id.to_string()).unwrap().id, id);
    assert_eq!(core.find_project("MEU-REPO").unwrap().id, id);
    assert_eq!(core.find_project(repo.to_str().unwrap()).unwrap().id, id);
    assert_eq!(
        core.find_project("outro").unwrap_err().code(),
        ErrorCode::NotFound
    );
}

#[test]
fn removing_a_project_clears_the_index_but_never_the_files() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core(&tmp);
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(
        repo.join("src/main.rs"),
        "fn main() { println!(\"oi\"); }\n",
    )
    .unwrap();
    std::fs::write(repo.join("README.md"), "# Repo\n").unwrap();
    let id = core.add_project(&repo).unwrap().id;
    core.index_project(
        id,
        &IndexOptions::new(IndexMode::Incremental, IndexTrigger::User),
        &JobControl::new(),
        &NoopObserver,
    )
    .unwrap();

    let removed = core.remove_project(id).unwrap();
    assert_eq!(removed.id, id);
    assert!(
        repo.join("src/main.rs").is_file(),
        "arquivos originais permanecem"
    );
    assert!(repo.join("README.md").is_file());
    let counts: (i64, i64, i64, i64) = core
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT (SELECT COUNT(*) FROM files), (SELECT COUNT(*) FROM contents),
                        (SELECT COUNT(*) FROM chunks), (SELECT COUNT(*) FROM chunks_fts)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .unwrap();
    assert_eq!(counts, (0, 0, 0, 0));
    assert_eq!(core.project(id).unwrap_err().code(), ErrorCode::NotFound);
    // Pode ser adicionado de novo.
    core.add_project(&repo).unwrap();
}

#[test]
fn cannot_remove_while_indexing() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core(&tmp);
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let id = core.add_project(&repo).unwrap().id;
    let now = nexus_core::time_util::now_ms();
    core.db()
        .write(|c| {
            c.execute(
                "INSERT INTO indexing_jobs (id, project_id, mode, trigger, status, phase, pid, started_at, heartbeat_at)
                 VALUES ('ativo', ?1, 'incremental', 'user', 'running', 'indexing', 1, ?2, ?2)",
                rusqlite::params![id, now],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        core.remove_project(id).unwrap_err().code(),
        ErrorCode::Conflict
    );
}
