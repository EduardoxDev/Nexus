//! Leitura mínima do HEAD de um repositório git, sem executar `git` nem depender de libgit2.
//! Suporta `.git` como diretório ou arquivo (worktrees/submódulos) e `packed-refs`.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Ex.: `refs/heads/main`; `None` com HEAD destacado.
    pub reference: Option<String>,
    pub commit: Option<String>,
}

const MAX_FILE: u64 = 64 * 1024;

fn read_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn is_hash(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    if meta.is_file() {
        let content = read_small(&dot_git)?;
        let target = content.trim().strip_prefix("gitdir:")?.trim();
        let path = PathBuf::from(target);
        return Some(if path.is_absolute() {
            path
        } else {
            root.join(path)
        });
    }
    None
}

/// Lê o HEAD. `None` quando `root` não é um repositório git.
pub fn read_head(root: &Path) -> Option<Head> {
    let dir = git_dir(root)?;
    let head = read_small(&dir.join("HEAD"))?;
    let head = head.trim();
    if is_hash(head) {
        return Some(Head {
            reference: None,
            commit: Some(head.to_ascii_lowercase()),
        });
    }
    let reference = head.strip_prefix("ref:")?.trim().to_owned();
    // Somente refs sob refs/ (evita caminhos arbitrários vindos do arquivo HEAD).
    if !reference.starts_with("refs/") || reference.contains("..") {
        return None;
    }
    let commit = resolve_ref(&dir, &reference).or_else(|| {
        // Worktrees guardam refs compartilhadas no diretório comum.
        let common = read_small(&dir.join("commondir"))?;
        let common = dir.join(common.trim());
        resolve_ref(&common, &reference)
    });
    Some(Head {
        reference: Some(reference),
        commit,
    })
}

fn resolve_ref(dir: &Path, reference: &str) -> Option<String> {
    if let Some(loose) = read_small(&dir.join(reference)) {
        let loose = loose.trim();
        if is_hash(loose) {
            return Some(loose.to_ascii_lowercase());
        }
    }
    let packed = read_small(&dir.join("packed-refs"))?;
    packed.lines().find_map(|line| {
        let (hash, name) = line.split_once(' ')?;
        (name.trim() == reference && is_hash(hash)).then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn reads_loose_ref() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(git.join("refs/heads/main"), format!("{HASH}\n")).unwrap();
        let head = read_head(dir.path()).unwrap();
        assert_eq!(head.reference.as_deref(), Some("refs/heads/main"));
        assert_eq!(head.commit.as_deref(), Some(HASH));
    }

    #[test]
    fn reads_packed_ref_and_detached_head() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/dev\n").unwrap();
        std::fs::write(
            git.join("packed-refs"),
            format!("# pack-refs with: peeled\n{HASH} refs/heads/dev\n"),
        )
        .unwrap();
        assert_eq!(read_head(dir.path()).unwrap().commit.as_deref(), Some(HASH));

        std::fs::write(git.join("HEAD"), HASH).unwrap();
        let head = read_head(dir.path()).unwrap();
        assert_eq!(head.reference, None);
        assert_eq!(head.commit.as_deref(), Some(HASH));
    }

    #[test]
    fn rejects_suspicious_refs_and_non_repos() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_head(dir.path()).is_none());
        let git = dir.path().join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: ../../etc/passwd\n").unwrap();
        assert!(read_head(dir.path()).is_none());
    }
}
