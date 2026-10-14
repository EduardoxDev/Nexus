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
/// Linhas máximas por `blame`.
const MAX_BLAME_LINES: i64 = 2_000;
/// Campos por commit na saída de `git log -z` (hash, autor, data, assunto, separados por NUL).
const LOG_FIELDS: usize = 4;

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
        run(cmd, TIMEOUT).is_ok_and(|out| out.text.starts_with("git version"))
    })
}

/// Saída do git: texto e se passou do limite (o resto foi descartado).
struct Output {
    text: String,
    truncated: bool,
}

/// Roda o comando com prazo e limite de saída. Erro = git ausente, falhou ou passou do prazo.
fn run(mut cmd: Command, timeout: Duration) -> std::io::Result<Output> {
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
        let extra = std::io::copy(&mut stdout, &mut std::io::sink()).unwrap_or(0);
        (buf, extra > 0)
    });
    let deadline = Instant::now() + timeout;
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
    let (buf, truncated) = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(std::io::Error::other(format!("o git saiu com {status}")));
    }
    Ok(Output {
        text: String::from_utf8_lossy(&buf).into_owned(),
        truncated,
    })
}

/// Comando git dentro do repositório, com as defesas contra configuração do próprio repositório.
fn repo_git(root: &Path) -> Command {
    let mut cmd = git();
    cmd.arg("-C")
        .arg(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(["-c", "core.hooksPath=/dev/null"])
        .arg("--no-pager")
        // O caminho é o nome exato do arquivo, não um padrão (`a*` não pega outros arquivos).
        .arg("--literal-pathspecs");
    cmd
}

/// Tempo que resta até o prazo (zero se já passou).
fn remaining(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

/// Há um `.git` (pasta, ou arquivo de worktree) na raiz ou acima: é assim que o git acha o
/// repositório. Sem nenhum, a pasta não é repositório e não há histórico a mostrar.
fn in_repository(root: &Path) -> bool {
    root.ancestors().any(|dir| dir.join(".git").exists())
}

/// Se o arquivo está versionado; `None` quando o próprio git falha (repositório quebrado, prazo).
/// Só consultado quando o comando principal falha, para não gastar um processo a mais sempre.
fn tracked(root: &Path, rel: &str, deadline: Instant) -> Option<bool> {
    let mut cmd = repo_git(root);
    cmd.args(["ls-files", "-z", "--"]).arg(rel);
    run(cmd, remaining(deadline))
        .ok()
        .map(|out| !out.text.is_empty())
}

/// Falha do git: fora de repositório ou arquivo não versionado viram "sem histórico" (`None`);
/// o resto (prazo, repositório quebrado, erro do git) é erro, para não parecer "sem commits".
fn failure(
    root: &Path,
    rel: &str,
    e: &std::io::Error,
    what: &str,
    deadline: Instant,
) -> Option<Error> {
    let error = Some(Error::Internal(format!("{what} de {rel}: {e}")));
    if e.kind() == std::io::ErrorKind::TimedOut {
        return error;
    }
    if !in_repository(root) {
        return None;
    }
    match tracked(root, rel, deadline) {
        Some(false) => None,
        _ => error,
    }
}

/// Roda dentro do prazo; sem tempo restante, nem começa.
fn run_until(cmd: Command, deadline: Instant) -> std::io::Result<Output> {
    let left = remaining(deadline).min(TIMEOUT);
    if left.is_zero() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "o git passou do prazo",
        ));
    }
    run(cmd, left)
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

/// `git log -z` com campos separados por NUL: NUL não existe em nome nem em mensagem de commit,
/// então nenhum texto do repositório desalinha os registros. Um registro incompleto (saída cortada
/// no limite) é descartado.
fn parse_commits(out: &str) -> Vec<Commit> {
    let fields: Vec<&str> = out.split('\0').collect();
    fields
        .as_chunks::<LOG_FIELDS>()
        .0
        .iter()
        .filter_map(|f| {
            let hash = f[0].trim_matches(['\n', '\r']).to_owned();
            if hash.len() < 7 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let author = f[1].to_owned();
            let secs: i64 = f[2].trim().parse().ok()?;
            let subject = f[3].trim().to_owned();
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
    file_history_until(root, rel, limit, Instant::now() + TIMEOUT)
}

/// Como [`file_history`], com um prazo absoluto que vale para tudo, inclusive a conferência
/// depois de uma falha (o pack divide um prazo total entre os arquivos).
pub fn file_history_until(
    root: &Path,
    rel: &str,
    limit: u32,
    deadline: Instant,
) -> Result<Vec<Commit>> {
    if !available() {
        return Err(unavailable());
    }
    checked(root, rel)?;
    let mut cmd = repo_git(root);
    cmd.args(["log", "-z", "--follow", "--no-color", "--no-ext-diff"])
        .arg(format!("-n{}", limit.clamp(1, 200)))
        .arg("--format=%H%x00%an%x00%at%x00%s")
        .arg("--")
        .arg(rel);
    match run_until(cmd, deadline) {
        Ok(out) => {
            let mut commits = parse_commits(&out.text);
            // Saída cortada no limite: o último registro pode ter o assunto pela metade.
            if out.truncated {
                commits.pop();
            }
            Ok(commits)
        }
        Err(e) => failure(root, rel, &e, "histórico do git", deadline).map_or(Ok(Vec::new()), Err),
    }
}

/// Quem escreveu as linhas `start..=end` de `rel`, agrupado por commit (mais recente primeiro).
/// Fora de um repositório, ou com o arquivo não versionado, a lista é vazia.
pub fn blame(root: &Path, rel: &str, start: i64, end: i64) -> Result<Vec<BlameEntry>> {
    if !available() {
        return Err(unavailable());
    }
    checked(root, rel)?;
    if start < 1 || end < start || end - start >= MAX_BLAME_LINES {
        return Err(Error::InvalidInput(format!(
            "intervalo de linhas inválido: {start}–{end} (até {MAX_BLAME_LINES} linhas)"
        )));
    }
    let mut cmd = repo_git(root);
    cmd.args(["blame", "--porcelain", "--no-textconv"])
        .arg(format!("-L{start},{end}"))
        .arg("--")
        .arg(rel);
    let deadline = Instant::now() + TIMEOUT;
    match run_until(cmd, deadline) {
        // Autoria parcial seria enganosa: com a saída cortada, é erro.
        Ok(out) if out.truncated => Err(Error::Internal(format!(
            "autoria do git de {rel}: saída grande demais"
        ))),
        Ok(out) => Ok(parse_blame(&out.text)),
        Err(e) => failure(root, rel, &e, "autoria do git", deadline).map_or(Ok(Vec::new()), Err),
    }
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
        // Caracteres de controle no assunto não desalinham; registro cortado no fim é descartado.
        let out = [
            "0123456789abcdef0123456789abcdef01234567",
            "Ana Maria",
            "1700000000",
            "feat: cupom \u{1e}\u{1f}",
            "\nfedcba9876543210fedcba9876543210fedcba98",
            "Bruno",
            "1690000000",
            "",
            "\nabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "Cortado",
        ]
        .join("\u{0}");
        let c = parse_commits(&out);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].short_hash, "0123456");
        assert_eq!(c[0].author, "Ana Maria");
        assert_eq!(c[0].date_ms, 1_700_000_000_000);
        assert_eq!(c[0].subject, "feat: cupom \u{1e}\u{1f}");
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
