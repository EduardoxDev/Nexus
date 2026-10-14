//! Histórico do git de um arquivo do projeto, pelo `git` instalado (ADR-0013).
//!
//! Só leitura (`log`, `blame`), sem shell, com argumentos fixos. O caminho passa antes pela
//! conferência de escopo (`scan::check_project_path`): nada fora do projeto chega ao git.
//! Defesas contra configuração maliciosa do repositório: `core.fsmonitor` e hooks desligados,
//! `blame` sem `textconv`, sem prompt de senha, sem locks opcionais; prazo e limite de saída.
//! Sem `git` no computador, `available()` é falso e as funções devolvem `GitUnavailable`.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Serialize;
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::scan::{ProjectPath, check_project_path};

/// Prazo de cada chamada ao git.
const TIMEOUT: Duration = Duration::from_secs(5);
/// Saída máxima lida do git (o resto é descartado).
const MAX_OUTPUT: usize = 1024 * 1024;
/// Separadores de campo e de registro no `--format` (não aparecem em texto comum).
const FS: char = '\u{1f}';
const RS: char = '\u{1e}';

/// Um commit que tocou o arquivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    /// Nome do autor (o e-mail não é exposto).
    pub author: String,
    /// Data do autor (ms desde a época).
    pub date_ms: i64,
    pub subject: String,
}

/// Autoria de um trecho (de `git blame`), agrupada por commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BlameEntry {
    pub commit: Commit,
    /// Linhas do trecho que vieram deste commit.
    pub lines: Vec<i64>,
}

/// Resposta para a interface: commits do arquivo, ou `git_available` falso sem git instalado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileHistory {
    pub git_available: bool,
    pub commits: Vec<Commit>,
}

fn git() -> Command {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Sem janela de console piscando quando chamado pelo app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `git` existe e responde (verificado uma vez por processo).
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let mut cmd = git();
        cmd.arg("--version");
        run(cmd).is_ok_and(|out| out.starts_with("git version"))
    })
}

/// Roda o comando com prazo e limite de saída. Erro = git ausente, falhou ou passou do prazo.
fn run(mut cmd: Command) -> std::io::Result<String> {
    let mut child = cmd.spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("sem stdout"))?;
    // Lê em outra thread: com o pipe cheio, o git travaria esperando.
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = (&mut stdout).take(MAX_OUTPUT as u64).read_to_end(&mut buf);
        // Descarta o excedente para o git conseguir terminar.
        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
        buf
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "o git passou do prazo",
            ));
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    let buf = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(std::io::Error::other(format!("o git saiu com {status}")));
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Comando git dentro do repositório, com as defesas contra configuração do próprio repositório.
fn repo_git(root: &Path) -> Command {
    let mut cmd = git();
    cmd.arg("-C")
        .arg(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(["-c", "core.hooksPath=/dev/null"])
        .arg("--no-pager");
    cmd
}

/// O arquivo está versionado num repositório git (fora de repositório ou não versionado: falso).
fn tracked(root: &Path, rel: &str) -> bool {
    let mut cmd = repo_git(root);
    cmd.args(["ls-files", "--error-unmatch", "--"]).arg(rel);
    run(cmd).is_ok()
}

fn unavailable() -> Error {
    Error::NotFound("o git não está instalado neste computador".into())
}

/// Caminho relativo conferido: existe, é arquivo comum e está dentro do projeto.
fn checked(root: &Path, rel: &str) -> Result<()> {
    match check_project_path(root, rel)? {
        ProjectPath::File(_) => Ok(()),
        ProjectPath::Missing => Err(Error::NotFound(format!("{rel} não existe no projeto"))),
        ProjectPath::Outside => Err(Error::InvalidInput(format!(
            "{rel} está fora do projeto ou passa por um link"
        ))),
    }
}

fn parse_commits(out: &str) -> Vec<Commit> {
    out.split(RS)
        .filter_map(|rec| {
            let mut f = rec.trim_matches(['\n', '\r']).split(FS);
            let hash = f.next()?.trim().to_owned();
            if hash.len() < 7 {
                return None;
            }
            let author = f.next()?.to_owned();
            let secs: i64 = f.next()?.trim().parse().ok()?;
            let subject = f.next().unwrap_or("").trim().to_owned();
            Some(Commit {
                short_hash: hash[..7].to_owned(),
                hash,
                author,
                date_ms: secs * 1000,
                subject,
            })
        })
        .collect()
}

/// Commits que tocaram `rel` (seguindo renomeações), do mais novo ao mais antigo.
/// Fora de um repositório, ou com o arquivo não versionado, a lista é vazia.
pub fn file_history(root: &Path, rel: &str, limit: u32) -> Result<Vec<Commit>> {
    if !available() {
        return Err(unavailable());
    }
    checked(root, rel)?;
    if !tracked(root, rel) {
        return Ok(Vec::new());
    }
    let mut cmd = repo_git(root);
    cmd.args(["log", "--follow", "--no-color", "--no-ext-diff"])
        .arg(format!("-n{}", limit.clamp(1, 200)))
        .arg(format!("--format={RS}%H{FS}%an{FS}%at{FS}%s"))
        .arg("--")
        .arg(rel);
    let out = run(cmd).map_err(|e| Error::Internal(format!("histórico do git de {rel}: {e}")))?;
    Ok(parse_commits(&out))
}

/// Quem escreveu as linhas `start..=end` de `rel`, agrupado por commit (mais recente primeiro).
/// Fora de um repositório, ou com o arquivo não versionado, a lista é vazia.
pub fn blame(root: &Path, rel: &str, start: i64, end: i64) -> Result<Vec<BlameEntry>> {
    if !available() {
        return Err(unavailable());
    }
    checked(root, rel)?;
    if start < 1 || end < start || end - start > 2_000 {
        return Err(Error::InvalidInput(format!(
            "intervalo de linhas inválido: {start}–{end}"
        )));
    }
    if !tracked(root, rel) {
        return Ok(Vec::new());
    }
    let mut cmd = repo_git(root);
    cmd.args(["blame", "--porcelain", "--no-textconv"])
        .arg(format!("-L{start},{end}"))
        .arg("--")
        .arg(rel);
    let out = run(cmd).map_err(|e| Error::Internal(format!("autoria do git de {rel}: {e}")))?;
    Ok(parse_blame(&out))
}

fn parse_blame(out: &str) -> Vec<BlameEntry> {
    use std::collections::HashMap;
    // Cabeçalho: "<hash> <linha original> <linha final> [<n>]"; metadados só na 1ª ocorrência.
    let mut meta: HashMap<String, (String, i64, String)> = HashMap::new();
    let mut lines: Vec<(String, i64)> = Vec::new();
    let mut current: Option<String> = None;
    for line in out.lines() {
        if line.starts_with('\t') {
            continue;
        }
        let mut parts = line.split(' ');
        let first = parts.next().unwrap_or("");
        if first.len() >= 40 && first.bytes().all(|b| b.is_ascii_hexdigit()) {
            let final_line: i64 = parts.nth(1).and_then(|n| n.parse().ok()).unwrap_or(0);
            lines.push((first.to_owned(), final_line));
            meta.entry(first.to_owned()).or_default();
            current = Some(first.to_owned());
            continue;
        }
        let Some(hash) = &current else { continue };
        let entry = meta.entry(hash.clone()).or_default();
        if let Some(v) = line.strip_prefix("author ") {
            entry.0 = v.to_owned();
        } else if let Some(v) = line.strip_prefix("author-time ") {
            entry.1 = v.trim().parse::<i64>().unwrap_or(0) * 1000;
        } else if let Some(v) = line.strip_prefix("summary ") {
            entry.2 = v.to_owned();
        }
    }
    let mut out: Vec<BlameEntry> = Vec::new();
    for (hash, line) in lines {
        if let Some(e) = out.iter_mut().find(|e| e.commit.hash == hash) {
            e.lines.push(line);
            continue;
        }
        let (author, date_ms, subject) = meta.get(&hash).cloned().unwrap_or_default();
        out.push(BlameEntry {
            commit: Commit {
                short_hash: hash.get(..7).unwrap_or(&hash).to_owned(),
                hash,
                author,
                date_ms,
                subject,
            },
            lines: vec![line],
        });
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.commit.date_ms));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_log_records() {
        let out = format!(
            "{RS}0123456789abcdef0123456789abcdef01234567{FS}Ana Maria{FS}1700000000{FS}feat: cupom{FS}x\n{RS}fedcba9876543210fedcba9876543210fedcba98{FS}Bruno{FS}1690000000{FS}\n"
        );
        let c = parse_commits(&out);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].short_hash, "0123456");
        assert_eq!(c[0].author, "Ana Maria");
        assert_eq!(c[0].date_ms, 1_700_000_000_000);
        assert_eq!(c[0].subject, "feat: cupom");
        assert_eq!(c[1].subject, "");
    }

    #[test]
    fn parses_porcelain_blame_grouped_by_commit() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let out = format!(
            "{a} 1 1 2\nauthor Ana\nauthor-time 1700000000\nsummary primeiro\nfilename x.ts\n\tlinha 1\n{a} 2 2\n\tlinha 2\n{b} 5 3 1\nauthor Bruno\nauthor-time 1710000000\nsummary segundo\nfilename x.ts\n\tlinha 3\n"
        );
        let e = parse_blame(&out);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].commit.author, "Bruno", "mais recente primeiro");
        assert_eq!(e[0].lines, [3]);
        assert_eq!(e[1].commit.subject, "primeiro");
        assert_eq!(e[1].lines, [1, 2]);
    }
}
