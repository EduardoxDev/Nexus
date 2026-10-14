//! Memória compartilhada pelo repositório: `.nexus/memory/<slug>.md` (ADR-0014).
//!
//! - **Exportar** (só com o clique do usuário): as memórias **confirmadas** viram arquivos
//!   Markdown com front-matter. A prévia mostra o que será criado ou alterado; a gravação confere
//!   que nada mudou desde a prévia (`token`), escreve só dentro de `.nexus/memory/` (sem links,
//!   pelo caminho real) e nunca apaga arquivos.
//! - **Ler** (na indexação ou pelo botão): cada arquivo vira, no máximo, uma **proposta**.
//!   Nada é confirmado sozinho, venha o arquivo de quem vier (um PR pode trazer um arquivo
//!   malicioso). A memória é reconhecida pelo `id`, então a ida e volta não duplica nada.
//!
//! Regras da leitura, por arquivo:
//! - `id` desconhecido → proposta nova com o mesmo `id` (substituindo `supersedes`, se a
//!   memória citada estiver confirmada aqui);
//! - `id` de memória confirmada: texto igual → nada; `revision` menor que a daqui → ignorado
//!   (o arquivo está atrasado); senão → proposta de revisão (`supersedes` = a confirmada);
//! - `id` de proposta lida de arquivo → acompanha o arquivo; arquivada ou substituída → ignorado;
//! - arquivo de uma memória que outro arquivo da pasta substitui → ignorado (ficou obsoleto).

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;

use super::{
    Actor, Draft, MAX_REFS, Memory, MemoryKind, MemoryOrigin, MemoryRef, MemoryStatus,
    RevisionChange, check_body, check_title, normalize_rel_path,
};
use crate::error::{Error, Result};
use crate::time_util::now_ms;

/// Pasta, relativa à raiz do projeto.
pub const DIR: &str = ".nexus/memory";
/// Versão do formato dos arquivos.
pub const FORMAT: &str = "nexus-memory/1";
/// Cabe o maior texto aceito (20 000 caracteres de até 4 bytes) mais o front-matter.
const MAX_FILE_BYTES: u64 = 128 * 1024;
const MAX_FILES: usize = 500;
/// Revisão máxima aceita num arquivo (folga para as edições seguintes sem estourar).
const MAX_REVISION: i64 = 1_000_000_000;

/// Uma memória como fica no arquivo (sem estado: quem recebe sempre decide).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedMemory {
    pub id: String,
    pub kind: MemoryKind,
    pub title: String,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub supersedes: Option<String>,
    pub refs: Vec<MemoryRef>,
    pub body: String,
}

impl SharedMemory {
    fn from_memory(m: &Memory) -> Self {
        Self {
            id: m.id.clone(),
            kind: m.kind,
            title: m.title.clone(),
            revision: m.revision,
            created_at: m.created_at,
            updated_at: m.updated_at,
            supersedes: m.supersedes.clone(),
            refs: m.refs.clone(),
            body: m.body.clone(),
        }
    }
}

// ---- Formato -----------------------------------------------------------------------------

fn quoted(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_owned())
}

fn iso(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .ok()
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
}

fn parse_iso(s: &str) -> Option<i64> {
    let t = OffsetDateTime::parse(s, &Rfc3339).ok()?;
    i64::try_from(t.unix_timestamp_nanos() / 1_000_000).ok()
}

/// Markdown com front-matter (YAML simples: textos entre aspas, como JSON).
pub fn render(m: &SharedMemory) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("format: {FORMAT}\n"));
    out.push_str(&format!("id: {}\n", m.id));
    out.push_str(&format!("kind: {}\n", m.kind.as_str()));
    out.push_str(&format!("title: {}\n", quoted(&m.title)));
    out.push_str(&format!("revision: {}\n", m.revision));
    out.push_str(&format!("createdAt: {}\n", iso(m.created_at)));
    out.push_str(&format!("updatedAt: {}\n", iso(m.updated_at)));
    if let Some(s) = &m.supersedes {
        out.push_str(&format!("supersedes: {s}\n"));
    }
    if m.refs.is_empty() {
        out.push_str("refs: []\n");
    } else {
        out.push_str("refs:\n");
        for r in &m.refs {
            out.push_str(&format!("  - relPath: {}\n", quoted(&r.rel_path)));
            if let (Some(s), Some(e)) = (r.start_line, r.end_line) {
                out.push_str(&format!("    startLine: {s}\n    endLine: {e}\n"));
            }
            out.push_str(&format!("    contentHash: {}\n", quoted(&r.content_hash)));
        }
    }
    out.push_str("---\n\n");
    out.push_str(m.body.trim());
    out.push('\n');
    out
}

fn value(raw: &str) -> Result<String> {
    let v = raw.trim();
    if v.starts_with('"') {
        serde_json::from_str::<String>(v)
            .map_err(|_| Error::InvalidInput(format!("texto entre aspas inválido: {v}")))
    } else {
        Ok(v.to_owned())
    }
}

fn is_id(s: &str) -> bool {
    (8..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

fn line_number(field: &str, v: &str) -> Result<i64> {
    v.parse::<i64>()
        .ok()
        .filter(|n| *n >= 1)
        .ok_or_else(|| Error::InvalidInput(format!("{field} inválido: {v}")))
}

/// Lê um arquivo de memória. Recusa versões de formato desconhecidas e campos inválidos;
/// campos desconhecidos são ignorados (versões futuras podem acrescentar).
pub fn parse(text: &str) -> Result<SharedMemory> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n").ok_or_else(|| {
        Error::InvalidInput("sem front-matter (o arquivo deve começar com ---)".into())
    })?;
    let (header, body) = match rest.find("\n---\n") {
        Some(i) => (&rest[..i], &rest[i + 5..]),
        None => match rest.strip_suffix("\n---") {
            Some(h) => (h, ""),
            None => return Err(Error::InvalidInput("front-matter sem fim (---)".into())),
        },
    };

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut refs: Vec<HashMap<String, String>> = Vec::new();
    let mut in_refs = false;
    for line in header.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if in_refs && let Some(item) = line.strip_prefix("  - ") {
            refs.push(HashMap::new());
            let (k, v) = item
                .split_once(':')
                .ok_or_else(|| Error::InvalidInput(format!("linha inválida: {line}")))?;
            if let Some(r) = refs.last_mut() {
                r.insert(k.trim().to_owned(), value(v)?);
            }
            continue;
        }
        if in_refs && let Some(item) = line.strip_prefix("    ") {
            let (k, v) = item
                .split_once(':')
                .ok_or_else(|| Error::InvalidInput(format!("linha inválida: {line}")))?;
            let r = refs.last_mut().ok_or_else(|| {
                Error::InvalidInput(format!("linha fora de uma referência: {line}"))
            })?;
            r.insert(k.trim().to_owned(), value(v)?);
            continue;
        }
        let (k, v) = line
            .split_once(':')
            .ok_or_else(|| Error::InvalidInput(format!("linha inválida: {line}")))?;
        let k = k.trim();
        in_refs = k == "refs";
        if in_refs {
            if !matches!(v.trim(), "" | "[]") {
                return Err(Error::InvalidInput("refs deve ser uma lista".into()));
            }
            continue;
        }
        fields.insert(k.to_owned(), value(v)?);
    }

    let field = |k: &str| {
        fields
            .get(k)
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("falta o campo {k}")))
    };
    let format = field("format")?;
    if format != FORMAT {
        return Err(Error::InvalidInput(format!(
            "formato {format} não suportado (esta versão lê {FORMAT})"
        )));
    }
    let id = field("id")?;
    if !is_id(&id) {
        return Err(Error::InvalidInput(format!("id inválido: {id}")));
    }
    let kind_text = field("kind")?;
    let kind = MemoryKind::parse(&kind_text)
        .ok_or_else(|| Error::InvalidInput(format!("tipo desconhecido: {kind_text}")))?;
    let revision = field("revision")?
        .parse::<i64>()
        .ok()
        .filter(|r| (1..=MAX_REVISION).contains(r))
        .ok_or_else(|| {
            Error::InvalidInput(format!("revision deve ser um número de 1 a {MAX_REVISION}"))
        })?;
    let created_at = parse_iso(&field("createdAt")?)
        .ok_or_else(|| Error::InvalidInput("createdAt não é uma data RFC 3339".into()))?;
    let updated_at = parse_iso(&field("updatedAt")?)
        .ok_or_else(|| Error::InvalidInput("updatedAt não é uma data RFC 3339".into()))?;
    let supersedes = match fields.get("supersedes") {
        Some(s) if !is_id(s) => {
            return Err(Error::InvalidInput(format!("supersedes inválido: {s}")));
        }
        Some(s) => Some(s.clone()),
        None => None,
    };
    if refs.len() > MAX_REFS {
        return Err(Error::InvalidInput(format!(
            "no máximo {MAX_REFS} referências por memória"
        )));
    }
    let refs = refs
        .into_iter()
        .map(|r| {
            let rel_path = normalize_rel_path(r.get("relPath").map_or("", String::as_str))?;
            let start_line = r
                .get("startLine")
                .map(|v| line_number("startLine", v))
                .transpose()?;
            let end_line = r
                .get("endLine")
                .map(|v| line_number("endLine", v))
                .transpose()?;
            if start_line.is_some() != end_line.is_some()
                || matches!((start_line, end_line), (Some(s), Some(e)) if e < s)
            {
                return Err(Error::InvalidInput(format!(
                    "intervalo de linhas inválido em {rel_path}"
                )));
            }
            let content_hash = r.get("contentHash").cloned().unwrap_or_default();
            if content_hash.is_empty()
                || content_hash.len() > 128
                || !content_hash.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(Error::InvalidInput(format!(
                    "contentHash inválido em {rel_path}"
                )));
            }
            Ok(MemoryRef {
                rel_path,
                start_line,
                end_line,
                content_hash,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SharedMemory {
        id,
        kind,
        title: check_title(&field("title")?)?,
        revision,
        created_at,
        updated_at,
        supersedes,
        refs,
        body: check_body(body)?,
    })
}

/// Nome do arquivo de uma memória nova: título sem acentos + fim do id (único e estável).
fn file_name(m: &SharedMemory, taken: &HashSet<String>) -> String {
    let mut slug = String::new();
    for c in crate::search::query::normalize(&m.title).chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "memoria" } else { slug };
    let tail: String = m.id.chars().filter(char::is_ascii_hexdigit).collect();
    let tail = &tail[tail.len().saturating_sub(8)..];
    let base = format!("{slug}-{tail}");
    let mut name = format!("{base}.md");
    let mut n = 2;
    while taken.contains(&name.to_ascii_lowercase()) {
        name = format!("{base}-{n}.md");
        n += 1;
    }
    name
}

// ---- Pasta -------------------------------------------------------------------------------

fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink() || crate::scan::is_reparse_point(meta)
}

/// `.nexus/memory` conferida: nenhum trecho é link (ou junction) e o caminho real fica dentro
/// do projeto. Sem a pasta: `None`, ou criada quando `create` (só na exportação).
fn memory_dir(root: &Path, create: bool) -> Result<Option<PathBuf>> {
    let real_root = dunce::canonicalize(root)
        .map_err(|e| Error::io(format!("abrir a pasta do projeto {}", root.display()), e))?;
    let mut dir = root.to_path_buf();
    for part in DIR.split('/') {
        dir.push(part);
        match std::fs::symlink_metadata(&dir) {
            Ok(meta) if is_link(&meta) || !meta.is_dir() => {
                return Err(Error::InvalidInput(format!(
                    "{} é um link ou não é uma pasta; a NEXUS não lê nem grava ali",
                    dir.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !create {
                    return Ok(None);
                }
                std::fs::create_dir(&dir)
                    .map_err(|e| Error::io(format!("criar {}", dir.display()), e))?;
            }
            Err(e) => return Err(Error::io(format!("inspecionar {}", dir.display()), e)),
        }
    }
    let real =
        dunce::canonicalize(&dir).map_err(|e| Error::io(format!("abrir {}", dir.display()), e))?;
    if !real.starts_with(&real_root) {
        return Err(Error::InvalidInput(format!(
            "{} aponta para fora do projeto",
            dir.display()
        )));
    }
    Ok(Some(real))
}

struct SharedFile {
    name: String,
    text: String,
    hash: String,
}

/// O que há na pasta: arquivos lidos, **todos** os nomes `.md` (para nunca escolher um nome
/// já ocupado, nem por um link ou arquivo grande) e os recusados, com o motivo.
struct Listing {
    files: Vec<SharedFile>,
    names: HashSet<String>,
    invalid: Vec<String>,
}

/// Lê um arquivo sem seguir links: confere antes de abrir e de novo no arquivo aberto (no
/// Windows, um reparse point é aberto como ele mesmo; no Unix, o arquivo aberto precisa ser o
/// mesmo que foi conferido). `Ok(Err(motivo))` = recusado.
fn read_plain_file(path: &Path) -> Result<std::result::Result<Vec<u8>, String>> {
    use std::io::Read as _;
    let Ok(before) = std::fs::symlink_metadata(path) else {
        return Ok(Err("sumiu durante a leitura".into()));
    };
    if is_link(&before) || !before.is_file() {
        return Ok(Err("é um link ou não é um arquivo comum".into()));
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        opts.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    // Unix: não segue link no último trecho e não trava num FIFO ou dispositivo trocado no
    // lugar do arquivo (o handle é conferido logo abaixo e recusado se não for arquivo comum).
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
        const FLAGS: i32 = 0x0100 | 0x0004; // O_NOFOLLOW | O_NONBLOCK
        #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd")))]
        const FLAGS: i32 = 0o400_000 | 0o4_000; // O_NOFOLLOW | O_NONBLOCK (Linux)
        opts.custom_flags(FLAGS);
    }
    let file = opts
        .open(path)
        .map_err(|e| Error::io(format!("abrir {}", path.display()), e))?;
    let meta = file
        .metadata()
        .map_err(|e| Error::io(format!("inspecionar {}", path.display()), e))?;
    if is_link(&meta) || !meta.is_file() {
        return Ok(Err("é um link ou não é um arquivo comum".into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (meta.dev(), meta.ino()) != (before.dev(), before.ino()) {
            return Ok(Err("foi trocado durante a leitura".into()));
        }
    }
    if meta.len() > MAX_FILE_BYTES {
        return Ok(Err(format!("maior que {} KiB", MAX_FILE_BYTES / 1024)));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(format!("ler {}", path.display()), e))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Ok(Err(format!("maior que {} KiB", MAX_FILE_BYTES / 1024)));
    }
    Ok(Ok(bytes))
}

/// Arquivos `.md` comuns da pasta. Com mais de `MAX_FILES`, recusa a pasta inteira: ler só uma
/// parte esconderia nomes ocupados da exportação.
fn read_dir(dir: &Path) -> Result<Listing> {
    let mut names: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(dir).map_err(|e| Error::io(format!("listar {}", dir.display()), e))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.to_ascii_lowercase().ends_with(".md") {
            names.push(name);
        }
    }
    if names.len() > MAX_FILES {
        return Err(Error::InvalidInput(format!(
            "{DIR} tem mais de {MAX_FILES} arquivos .md; a NEXUS não lê nem grava nessa pasta"
        )));
    }
    names.sort();
    let mut files = Vec::new();
    let mut invalid = Vec::new();
    for name in &names {
        match read_plain_file(&dir.join(name))? {
            Err(why) => invalid.push(format!("{name}: {why}")),
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    let hash = blake3::hash(text.as_bytes()).to_hex().to_string();
                    files.push(SharedFile {
                        name: name.clone(),
                        text,
                        hash,
                    });
                }
                Err(_) => invalid.push(format!("{name}: não é UTF-8")),
            },
        }
    }
    Ok(Listing {
        files,
        names: names.iter().map(|n| n.to_ascii_lowercase()).collect(),
        invalid,
    })
}

fn rel(name: &str) -> String {
    format!("{DIR}/{name}")
}

// ---- Exportar ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ExportAction {
    Create,
    Update,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    pub rel_path: String,
    pub memory_id: String,
    pub title: String,
    pub action: ExportAction,
}

/// Arquivo da pasta que não corresponde a nenhuma memória confirmada daqui (nunca é apagado).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ObsoleteFile {
    pub rel_path: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    /// Pasta de destino, relativa ao projeto.
    pub directory: String,
    pub items: Vec<ExportItem>,
    pub obsolete: Vec<ObsoleteFile>,
    /// Identifica esta prévia: a exportação só grava se nada mudou desde então.
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub directory: String,
    pub created: u32,
    pub updated: u32,
    pub unchanged: u32,
}

struct Planned {
    preview: ExportPreview,
    /// (nome do arquivo, conteúdo) a gravar.
    writes: Vec<(String, String)>,
}

fn confirmed(conn: &Connection, project_id: i64) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM memories WHERE project_id = ?1 AND status = 'confirmed'
         ORDER BY created_at, id",
    )?;
    let ids: Vec<String> = stmt
        .query_map([project_id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ids.iter().map(|id| super::get(conn, id)).collect()
}

fn plan(conn: &Connection, root: &Path, project_id: i64) -> Result<Planned> {
    let listing = match memory_dir(root, false)? {
        Some(dir) => read_dir(&dir)?,
        None => Listing {
            files: Vec::new(),
            names: HashSet::new(),
            invalid: Vec::new(),
        },
    };
    let existing = &listing.files;
    let mut by_id: HashMap<String, &SharedFile> = HashMap::new();
    let mut parsed_title: HashMap<&str, Option<String>> = HashMap::new();
    for f in existing {
        match parse(&f.text) {
            Ok(m) => {
                parsed_title.insert(f.name.as_str(), Some(m.title.clone()));
                by_id.entry(m.id).or_insert(f);
            }
            Err(_) => {
                parsed_title.insert(f.name.as_str(), None);
            }
        }
    }
    // Todos os nomes da pasta, inclusive links e arquivos recusados: nenhum é sobrescrito.
    let mut taken = listing.names.clone();
    let mut items = Vec::new();
    let mut writes = Vec::new();
    let mut used: HashSet<&str> = HashSet::new();
    for m in confirmed(conn, project_id)? {
        let shared = SharedMemory::from_memory(&m);
        let content = render(&shared);
        if content.len() as u64 > MAX_FILE_BYTES {
            return Err(Error::InvalidInput(format!(
                "a memória \"{}\" passa de {} KiB e não pode ser exportada",
                m.title,
                MAX_FILE_BYTES / 1024
            )));
        }
        let (name, action) = match by_id.get(&m.id) {
            Some(f) => {
                used.insert(f.name.as_str());
                let action = if f.text.replace("\r\n", "\n") == content {
                    ExportAction::Unchanged
                } else {
                    ExportAction::Update
                };
                (f.name.clone(), action)
            }
            None => {
                let name = file_name(&shared, &taken);
                taken.insert(name.to_ascii_lowercase());
                (name, ExportAction::Create)
            }
        };
        if action != ExportAction::Unchanged {
            writes.push((name.clone(), content));
        }
        items.push(ExportItem {
            rel_path: rel(&name),
            memory_id: m.id.clone(),
            title: m.title.clone(),
            action,
        });
    }
    let obsolete = existing
        .iter()
        .filter(|f| !used.contains(f.name.as_str()))
        .map(|f| ObsoleteFile {
            rel_path: rel(&f.name),
            title: parsed_title.get(f.name.as_str()).cloned().flatten(),
        })
        .collect();
    // A prévia vale para o que estava no disco: qualquer arquivo da pasta que mude depois
    // (inclusive um que já seria atualizado) invalida o token.
    let mut hasher = blake3::Hasher::new();
    hasher.update(root.to_string_lossy().as_bytes());
    let mut names: Vec<&String> = listing.names.iter().collect();
    names.sort();
    for name in names {
        hasher.update(name.as_bytes());
        hasher.update(&[0]);
    }
    for f in existing {
        hasher.update(f.name.as_bytes());
        hasher.update(f.hash.as_bytes());
        hasher.update(&[0]);
    }
    for (name, content) in &writes {
        hasher.update(name.as_bytes());
        hasher.update(&[0]);
        hasher.update(content.as_bytes());
        hasher.update(&[0]);
    }
    Ok(Planned {
        preview: ExportPreview {
            directory: DIR.to_owned(),
            items,
            obsolete,
            token: hasher.finalize().to_hex()[..32].to_owned(),
        },
        writes,
    })
}

/// O que a exportação faria agora (não grava nada).
pub fn preview(conn: &Connection, root: &Path, project_id: i64) -> Result<ExportPreview> {
    Ok(plan(conn, root, project_id)?.preview)
}

/// Grava os arquivos da prévia identificada por `token`. Se algo mudou desde a prévia
/// (memória editada, arquivo alterado), recusa: a pessoa precisa ver a prévia nova.
pub fn export(
    conn: &Connection,
    root: &Path,
    project_id: i64,
    token: &str,
) -> Result<ExportReport> {
    let planned = plan(conn, root, project_id)?;
    if planned.preview.token != token {
        return Err(Error::Conflict(
            "a memória ou a pasta mudou desde a prévia; abra a prévia de novo antes de exportar"
                .into(),
        ));
    }
    let count = |a: ExportAction| {
        u32::try_from(
            planned
                .preview
                .items
                .iter()
                .filter(|i| i.action == a)
                .count(),
        )
        .unwrap_or(u32::MAX)
    };
    let report = ExportReport {
        directory: DIR.to_owned(),
        created: count(ExportAction::Create),
        updated: count(ExportAction::Update),
        unchanged: count(ExportAction::Unchanged),
    };
    if planned.writes.is_empty() {
        return Ok(report);
    }
    let dir = memory_dir(root, true)?
        .ok_or_else(|| Error::Internal("a pasta da memória não pôde ser criada".into()))?;
    for (name, content) in &planned.writes {
        write_file(&dir, name, content)?;
    }
    Ok(report)
}

/// Grava num arquivo temporário novo da mesma pasta e troca pelo definitivo (nunca meio
/// escrito). O temporário é criado com exclusividade: se o nome já existir (inclusive como
/// link), falha em vez de escrever através dele. Um destino que virou link é recusado.
fn write_file(dir: &Path, name: &str, content: &str) -> Result<()> {
    let target = dir.join(name);
    if let Ok(meta) = std::fs::symlink_metadata(&target)
        && (is_link(&meta) || !meta.is_file())
    {
        return Err(Error::InvalidInput(format!(
            "{} é um link ou não é um arquivo comum; a NEXUS não grava ali",
            target.display()
        )));
    }
    let tmp = dir.join(format!(".{name}.{}.tmp", crate::ops::new_op_id()));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| Error::io(format!("criar {}", tmp.display()), e))?;
    let written = f.write_all(content.as_bytes()).and_then(|()| f.sync_all());
    drop(f);
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, &target)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::io(format!("gravar {}", target.display()), e));
    }
    Ok(())
}

// ---- Ler ---------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SharedImportReport {
    /// Arquivos lidos.
    pub scanned: u32,
    /// Propostas novas (memórias que este banco não conhecia).
    pub created: u32,
    /// Propostas de revisão de uma memória confirmada.
    pub revisions: u32,
    /// Propostas existentes atualizadas para acompanhar o arquivo.
    pub updated: u32,
    /// Propostas de revisão arquivadas porque o arquivo voltou ao texto confirmado.
    pub withdrawn: u32,
    /// Iguais ao que já existe.
    pub unchanged: u32,
    /// Atrasados, obsoletos ou de memórias arquivadas aqui.
    pub ignored: u32,
    /// Arquivos recusados, com o motivo.
    pub invalid: Vec<String>,
    pub memory_ids: Vec<String>,
}

fn find(conn: &Connection, id: &str) -> Result<Option<Memory>> {
    let exists: Option<String> = conn
        .query_row("SELECT id FROM memories WHERE id = ?1", [id], |r| r.get(0))
        .optional()?;
    exists.map(|id| super::get(conn, &id)).transpose()
}

/// Mesmo tipo, título, texto e referências (caminho e linhas; o hash registrado pode diferir
/// entre bancos sem que a memória tenha mudado).
fn same_content(m: &Memory, s: &SharedMemory) -> bool {
    let place = |refs: &[MemoryRef]| -> Vec<(String, Option<i64>, Option<i64>)> {
        refs.iter()
            .map(|r| (r.rel_path.clone(), r.start_line, r.end_line))
            .collect()
    };
    m.kind == s.kind && m.title == s.title && m.body == s.body && place(&m.refs) == place(&s.refs)
}

/// O arquivo voltou ao texto confirmado: a proposta de revisão que ele tinha gerado não
/// corresponde a mais nada no repositório e é arquivada (pode ser restaurada).
fn withdraw(tx: &rusqlite::Transaction<'_>, mut p: Memory) -> Result<()> {
    p.status = MemoryStatus::Archived;
    p.revision += 1;
    p.updated_at = now_ms();
    super::store(tx, &p)?;
    super::snapshot(
        tx,
        &p,
        Actor::System,
        RevisionChange::Archived,
        Some("O arquivo compartilhado voltou ao texto confirmado"),
        p.updated_at,
    )
}

/// Proposta de revisão pendente vinda deste arquivo para a memória `target`.
fn pending_revision(
    conn: &Connection,
    project_id: i64,
    target: &str,
    origin_path: &str,
) -> Result<Option<Memory>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM memories
             WHERE project_id = ?1 AND supersedes = ?2 AND origin = 'shared_file'
               AND origin_path = ?3 AND status = 'proposed'
             ORDER BY created_at DESC LIMIT 1",
            params![project_id, target, origin_path],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| super::get(conn, &id)).transpose()
}

/// `supersedes` do arquivo, se a memória citada está confirmada neste projeto (senão, nenhum).
fn resolve_supersedes(
    conn: &Connection,
    project_id: i64,
    s: &SharedMemory,
) -> Result<Option<String>> {
    Ok(match &s.supersedes {
        Some(old) => find(conn, old)?
            .filter(|m| m.project_id == project_id && m.status == MemoryStatus::Confirmed)
            .map(|m| m.id),
        None => None,
    })
}

/// Atualiza uma proposta lida de arquivo para o conteúdo novo do arquivo. `supersedes`:
/// `Some` troca a memória que a proposta substituiria (proposta nova vinda do arquivo);
/// `None` mantém (proposta de revisão, que sempre substitui a confirmada de mesmo id).
fn follow_file(
    tx: &rusqlite::Transaction<'_>,
    mut p: Memory,
    s: &SharedMemory,
    supersedes: Option<Option<String>>,
    hash: &str,
) -> Result<String> {
    p.kind = s.kind;
    p.title.clone_from(&s.title);
    p.body.clone_from(&s.body);
    p.refs.clone_from(&s.refs);
    p.origin_hash = Some(hash.to_owned());
    if let Some(target) = supersedes {
        p.supersedes = target;
    }
    p.revision += 1;
    p.updated_at = now_ms();
    super::store(tx, &p)?;
    super::snapshot(
        tx,
        &p,
        Actor::SharedFile,
        RevisionChange::Edited,
        Some("O arquivo compartilhado mudou"),
        p.updated_at,
    )?;
    Ok(p.id)
}

/// Arquivos de `.nexus/memory/` já lidos e interpretados (sem tocar no banco).
pub struct Loaded {
    parsed: Vec<(SharedFile, SharedMemory)>,
    scanned: u32,
    invalid: Vec<String>,
}

/// Lê e interpreta a pasta. Fica fora da trava de escrita do banco: só [`apply`] escreve.
pub fn load(root: &Path) -> Result<Option<Loaded>> {
    let Some(dir) = memory_dir(root, false)? else {
        return Ok(None);
    };
    let listing = read_dir(&dir)?;
    let mut loaded = Loaded {
        parsed: Vec::new(),
        scanned: 0,
        invalid: listing.invalid,
    };
    for f in listing.files {
        loaded.scanned += 1;
        match parse(&f.text) {
            Ok(m) => loaded.parsed.push((f, m)),
            Err(e) => loaded
                .invalid
                .push(format!("{}: {}", f.name, e.user_message())),
        }
    }
    Ok(Some(loaded))
}

/// Lê `.nexus/memory/` e cria ou ajusta **propostas** (regras no topo do módulo).
pub fn import(conn: &mut Connection, root: &Path, project_id: i64) -> Result<SharedImportReport> {
    match load(root)? {
        Some(loaded) => apply(conn, project_id, loaded),
        None => Ok(SharedImportReport::default()),
    }
}

/// Aplica ao banco os arquivos lidos por [`load`].
pub fn apply(conn: &mut Connection, project_id: i64, loaded: Loaded) -> Result<SharedImportReport> {
    let mut report = SharedImportReport {
        scanned: loaded.scanned,
        invalid: loaded.invalid,
        ..SharedImportReport::default()
    };
    let parsed = loaded.parsed;
    // Memórias que outro arquivo da pasta substitui: o arquivo delas ficou para trás.
    let replaced: HashSet<String> = parsed
        .iter()
        .filter_map(|(_, m)| m.supersedes.clone())
        .collect();

    for (file, s) in parsed {
        let origin_path = rel(&file.name);
        let note = format!("Lida de {origin_path}");
        if replaced.contains(&s.id) {
            report.ignored += 1;
            continue;
        }
        let existing = find(conn, &s.id)?;
        if existing
            .as_ref()
            .is_some_and(|m| m.project_id != project_id)
        {
            report
                .invalid
                .push(format!("{}: o id pertence a outro projeto", file.name));
            continue;
        }
        let tx = conn.transaction()?;
        match existing {
            None => {
                // Só substitui o que está confirmado aqui; senão, entra como proposta solta.
                let supersedes = resolve_supersedes(&tx, project_id, &s)?;
                let m = super::insert(
                    &tx,
                    Draft {
                        id: Some(s.id.clone()),
                        // Começa na revisão do arquivo: uma edição feita lá (revisão maior)
                        // é reconhecida como nova; uma cópia atrasada (menor), como velha.
                        revision: s.revision,
                        project_id,
                        kind: s.kind,
                        title: s.title.clone(),
                        body: s.body.clone(),
                        refs: s.refs.clone(),
                        origin_path: Some(origin_path.clone()),
                        origin_hash: Some(file.hash.clone()),
                        supersedes,
                    },
                    Actor::SharedFile,
                    false,
                    Some(&note),
                )?;
                report.created += 1;
                report.memory_ids.push(m.id);
            }
            Some(m) => match m.status {
                MemoryStatus::Confirmed if same_content(&m, &s) => {
                    match pending_revision(&tx, project_id, &m.id, &origin_path)? {
                        Some(p) => {
                            withdraw(&tx, p)?;
                            report.withdrawn += 1;
                        }
                        None => report.unchanged += 1,
                    }
                }
                MemoryStatus::Confirmed if s.revision < m.revision => report.ignored += 1,
                MemoryStatus::Confirmed => {
                    match pending_revision(&tx, project_id, &m.id, &origin_path)? {
                        Some(p) if p.origin_hash.as_deref() == Some(file.hash.as_str()) => {
                            report.unchanged += 1;
                        }
                        Some(p) => {
                            report
                                .memory_ids
                                .push(follow_file(&tx, p, &s, None, &file.hash)?);
                            report.updated += 1;
                        }
                        None => {
                            let p = super::insert(
                                &tx,
                                Draft {
                                    id: None,
                                    revision: 1,
                                    project_id,
                                    kind: s.kind,
                                    title: s.title.clone(),
                                    body: s.body.clone(),
                                    refs: s.refs.clone(),
                                    origin_path: Some(origin_path.clone()),
                                    origin_hash: Some(file.hash.clone()),
                                    supersedes: Some(m.id.clone()),
                                },
                                Actor::SharedFile,
                                false,
                                Some(&note),
                            )?;
                            report.revisions += 1;
                            report.memory_ids.push(p.id);
                        }
                    }
                }
                MemoryStatus::Proposed
                    if m.origin == MemoryOrigin::SharedFile
                        && m.origin_hash.as_deref() != Some(file.hash.as_str()) =>
                {
                    // Também uma mudança só em `supersedes` conta.
                    let wanted = resolve_supersedes(&tx, project_id, &s)?;
                    if same_content(&m, &s) && m.supersedes == wanted {
                        report.unchanged += 1;
                    } else {
                        report
                            .memory_ids
                            .push(follow_file(&tx, m, &s, Some(wanted), &file.hash)?);
                        report.updated += 1;
                    }
                }
                MemoryStatus::Proposed => report.unchanged += 1,
                MemoryStatus::Archived | MemoryStatus::Superseded => report.ignored += 1,
            },
        }
        tx.commit()?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SharedMemory {
        SharedMemory {
            id: "0190f0aa-1111-7222-8333-444455556666".into(),
            kind: MemoryKind::Decision,
            title: "Sessões no servidor: \"cookie\" HttpOnly".into(),
            revision: 3,
            created_at: 1_767_261_600_000,
            updated_at: 1_767_261_600_123,
            supersedes: Some("0190f0aa-0000-7222-8333-444455556666".into()),
            refs: vec![
                MemoryRef {
                    rel_path: "src/auth/session.ts".into(),
                    start_line: Some(10),
                    end_line: Some(20),
                    content_hash: "ab12".into(),
                },
                MemoryRef {
                    rel_path: "docs/x.md".into(),
                    start_line: None,
                    end_line: None,
                    content_hash: "cd34".into(),
                },
            ],
            body: "Usar sessões.\n\n---\n\nSem JWT.".into(),
        }
    }

    #[test]
    fn render_and_parse_round_trip() {
        let m = sample();
        let text = render(&m);
        assert!(text.starts_with("---\nformat: nexus-memory/1\n"));
        assert!(text.contains("createdAt: 2026-01-01T10:00:00Z"));
        assert_eq!(parse(&text).unwrap(), m);
        // CRLF (checkout no Windows) e BOM continuam legíveis.
        let crlf = format!("\u{feff}{}", text.replace('\n', "\r\n"));
        assert_eq!(parse(&crlf).unwrap(), m);
    }

    #[test]
    fn refuses_bad_files() {
        let good = render(&sample());
        for (bad, why) in [
            ("sem front-matter".to_owned(), "início"),
            (good.replace("nexus-memory/1", "nexus-memory/9"), "formato"),
            (good.replace("kind: decision", "kind: feitiço"), "tipo"),
            (good.replace("revision: 3", "revision: 0"), "revisão"),
            (
                good.replace("\"src/auth/session.ts\"", "\"../fora.ts\""),
                "caminho",
            ),
            (good.replace("id: 0190", "id: ../0190"), "id"),
            (good.replace("startLine: 10", "startLine: 30"), "intervalo"),
            (good.replace("title: ", "titulo: "), "campo"),
            (
                good.replace("revision: 3", "revision: 2000000000"),
                "revisão enorme",
            ),
            (
                good.replace("contentHash: \"cd34\"", "contentHash: \"\""),
                "hash vazio",
            ),
        ] {
            assert!(parse(&bad).is_err(), "{why}");
        }
    }

    #[test]
    fn file_names_are_readable_and_unique() {
        let m = sample();
        let name = file_name(&m, &HashSet::new());
        assert_eq!(name, "sessoes-no-servidor-cookie-httponly-55556666.md");
        let taken = HashSet::from([name.clone()]);
        assert_eq!(
            file_name(&m, &taken),
            "sessoes-no-servidor-cookie-httponly-55556666-2.md"
        );
    }
}
