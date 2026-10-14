//! Histórico do git contra um repositório de verdade (criado no teste com autor e data fixos).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::process::Command;

use nexus_core::ErrorCode;
use nexus_core::git_history::{available, blame, file_history};

fn git(root: &Path, args: &[&str], date: &str, author: &str) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", author)
        .env("GIT_AUTHOR_EMAIL", "dev@example.com")
        .env("GIT_COMMITTER_NAME", author)
        .env("GIT_COMMITTER_EMAIL", "dev@example.com")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

/// Repositório com três commits: cria, edita (outro autor) e renomeia o arquivo.
fn repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    git(
        root,
        &["init", "-q", "-b", "main"],
        "2026-01-01T10:00:00Z",
        "Ana",
    );
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/frete.ts"),
        "export const a = 1;\nexport const b = 2;\n",
    )
    .unwrap();
    git(root, &["add", "."], "2026-01-01T10:00:00Z", "Ana");
    git(
        root,
        &["commit", "-q", "-m", "feat: frete"],
        "2026-01-01T10:00:00Z",
        "Ana",
    );
    std::fs::write(
        root.join("src/frete.ts"),
        "export const a = 1;\nexport const b = 3;\n",
    )
    .unwrap();
    git(
        root,
        &["commit", "-q", "-am", "fix: valor do frete"],
        "2026-02-01T10:00:00Z",
        "Bruno",
    );
    git(
        root,
        &["mv", "src/frete.ts", "src/shipping.ts"],
        "2026-03-01T10:00:00Z",
        "Ana",
    );
    git(
        root,
        &["commit", "-q", "-m", "refactor: renomeia"],
        "2026-03-01T10:00:00Z",
        "Ana",
    );
    tmp
}

#[test]
fn history_follows_renames_newest_first() {
    if !available() {
        eprintln!("git ausente: teste pulado");
        return;
    }
    let tmp = repo();
    let h = file_history(tmp.path(), "src/shipping.ts", 10).unwrap();
    let subjects: Vec<_> = h.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(
        subjects,
        ["refactor: renomeia", "fix: valor do frete", "feat: frete"]
    );
    assert_eq!(h[1].author, "Bruno");
    assert_eq!(h[2].date_ms, 1_767_261_600_000); // 2026-01-01T10:00:00Z
    assert_eq!(
        file_history(tmp.path(), "src/shipping.ts", 1)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn blame_groups_lines_by_commit() {
    if !available() {
        return;
    }
    let tmp = repo();
    let b = blame(tmp.path(), "src/shipping.ts", 1, 2).unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].commit.author, "Bruno", "linha 2 mudou depois");
    assert_eq!(b[0].lines, [2]);
    assert_eq!(b[1].commit.subject, "feat: frete");
    assert_eq!(b[1].lines, [1]);
}

#[test]
fn paths_outside_the_project_or_missing_are_refused() {
    if !available() {
        return;
    }
    let tmp = repo();
    let err = file_history(tmp.path(), "../fora.ts", 5).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
    let err = file_history(tmp.path(), "src/nao-existe.ts", 5).unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotFound);
    let err = blame(tmp.path(), "src/shipping.ts", 5, 2).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidInput);
}

/// Um repositório pode configurar comandos que o git executa (`core.fsmonitor`). A NEXUS lê o
/// histórico de repositórios que não escreveu: nada configurado no repositório pode rodar.
#[test]
fn repository_config_cannot_run_commands() {
    if !available() {
        return;
    }
    let tmp = repo();
    let marker = tmp.path().join("EXECUTADO");
    let cmd = if cfg!(windows) {
        format!("cmd /c echo x > \"{}\"", marker.display())
    } else {
        format!("touch '{}'", marker.display())
    };
    git(
        tmp.path(),
        &["config", "core.fsmonitor", &cmd],
        "2026-03-01T10:00:00Z",
        "Ana",
    );
    file_history(tmp.path(), "src/shipping.ts", 5).unwrap();
    blame(tmp.path(), "src/shipping.ts", 1, 2).unwrap();
    assert!(!marker.exists(), "um comando do repositório foi executado");
}

#[test]
fn untracked_files_and_plain_folders_have_no_history() {
    if !available() {
        return;
    }
    let tmp = repo();
    std::fs::write(
        tmp.path().join("novo.ts"),
        "x
",
    )
    .unwrap();
    assert!(file_history(tmp.path(), "novo.ts", 5).unwrap().is_empty());
    assert!(blame(tmp.path(), "novo.ts", 1, 1).unwrap().is_empty());

    let plain = tempfile::tempdir().unwrap();
    std::fs::write(
        plain.path().join("a.ts"),
        "x
",
    )
    .unwrap();
    assert!(file_history(plain.path(), "a.ts", 5).unwrap().is_empty());
}
