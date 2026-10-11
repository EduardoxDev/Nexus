//! Classificação de arquivos por caminho: linguagem, tipo (código, teste, doc…) e binários conhecidos.
//! Heurística por nome/extensão — não lê conteúdo. A cobertura é declarada em `docs/architecture.md`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Code,
    Test,
    Doc,
    Config,
    Manifest,
    Lockfile,
    Ci,
    Data,
    Other,
}

impl FileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Test => "test",
            Self::Doc => "doc",
            Self::Config => "config",
            Self::Manifest => "manifest",
            Self::Lockfile => "lockfile",
            Self::Ci => "ci",
            Self::Data => "data",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "code" => Self::Code,
            "test" => Self::Test,
            "doc" => Self::Doc,
            "config" => Self::Config,
            "manifest" => Self::Manifest,
            "lockfile" => Self::Lockfile,
            "ci" => Self::Ci,
            "data" => Self::Data,
            _ => Self::Other,
        }
    }
}

/// Diretórios ignorados por padrão (além de `.gitignore`/`.nexusignore`).
pub const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "bower_components",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    ".cache",
    "coverage",
    ".nyc_output",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "venv",
    ".gradle",
    ".idea",
    ".vs",
    "obj",
    ".terraform",
];

/// Extensões tratadas como binárias sem precisar ler o arquivo.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "icns", "webp", "avif", "tif", "tiff", "psd", "pdf",
    "zip", "gz", "tgz", "bz2", "xz", "7z", "rar", "tar", "zst", "jar", "war", "exe", "dll", "so",
    "dylib", "a", "lib", "o", "obj", "pdb", "wasm", "class", "pyc", "pyo", "woff", "woff2", "ttf",
    "otf", "eot", "mp3", "wav", "ogg", "flac", "mp4", "mov", "avi", "mkv", "webm", "db", "sqlite",
    "sqlite3", "bin", "dat", "iso", "dmg", "msi", "node", "lockb", "pack", "idx",
];

pub fn is_binary_extension(rel_path: &str) -> bool {
    extension(rel_path).is_some_and(|ext| BINARY_EXTENSIONS.contains(&ext.as_str()))
}

fn file_name(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

fn extension(rel_path: &str) -> Option<String> {
    let name = file_name(rel_path);
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None; // ".gitignore" não tem extensão
    }
    Some(ext.to_ascii_lowercase())
}

/// Linguagem pelo nome/extensão. `None` quando desconhecida.
pub fn language(rel_path: &str) -> Option<&'static str> {
    let name = file_name(rel_path).to_ascii_lowercase();
    let by_name = match name.as_str() {
        "dockerfile" | "containerfile" => Some("dockerfile"),
        "makefile" | "gnumakefile" => Some("makefile"),
        "cmakelists.txt" => Some("cmake"),
        "jenkinsfile" => Some("groovy"),
        "gemfile" | "rakefile" => Some("ruby"),
        _ => None,
    };
    if by_name.is_some() {
        return by_name;
    }
    if name.starts_with("dockerfile.") {
        return Some("dockerfile");
    }
    Some(match extension(rel_path)?.as_str() {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "py" | "pyi" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "cs" => "csharp",
        "fs" | "fsx" => "fsharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "scala" => "scala",
        "dart" => "dart",
        "lua" => "lua",
        "r" => "r",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "clj" | "cljs" => "clojure",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" | "psm1" => "powershell",
        "bat" | "cmd" => "batch",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "md" | "markdown" | "mdx" => "markdown",
        "rst" => "rst",
        "adoc" => "asciidoc",
        "json" | "jsonc" | "json5" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" | "xsd" | "xsl" => "xml",
        "ini" | "cfg" | "conf" => "ini",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "tf" | "hcl" => "hcl",
        "gradle" => "groovy",
        "groovy" => "groovy",
        "csv" | "tsv" => "csv",
        "txt" => "text",
        _ => return None,
    })
}

const MANIFESTS: &[&str] = &[
    "cargo.toml",
    "package.json",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "pipfile",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "gemfile",
    "composer.json",
    "deno.json",
    "deno.jsonc",
    "pubspec.yaml",
    "mix.exs",
    "cmakelists.txt",
    "pnpm-workspace.yaml",
];

const LOCKFILES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "poetry.lock",
    "pipfile.lock",
    "uv.lock",
    "gemfile.lock",
    "composer.lock",
    "go.sum",
    "pubspec.lock",
    "mix.lock",
];

fn is_test_path(lower_path: &str, name: &str) -> bool {
    let in_test_dir = lower_path.split('/').rev().skip(1).any(|segment| {
        matches!(
            segment,
            "test" | "tests" | "__tests__" | "spec" | "specs" | "e2e" | "testing"
        )
    });
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    in_test_dir
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
        || stem.ends_with("_test")
        || stem.ends_with("_spec")
        || (stem.starts_with("test_") && name.ends_with(".py"))
        || (name.ends_with(".java") && (stem.ends_with("test") || stem.ends_with("tests")))
        || (name.ends_with(".cs") && stem.ends_with("tests"))
}

/// Tipo do arquivo para filtros, ranking e visão geral.
pub fn kind(rel_path: &str) -> FileKind {
    let lower = rel_path.to_ascii_lowercase();
    let name = file_name(&lower);

    if lower.starts_with(".github/workflows/")
        || lower.starts_with(".circleci/")
        || lower.starts_with(".gitlab/")
        || matches!(
            name,
            ".gitlab-ci.yml"
                | "azure-pipelines.yml"
                | "jenkinsfile"
                | ".travis.yml"
                | "bitbucket-pipelines.yml"
        )
    {
        return FileKind::Ci;
    }
    if LOCKFILES.contains(&name) {
        return FileKind::Lockfile;
    }
    if MANIFESTS.contains(&name)
        || name.ends_with(".csproj")
        || name.ends_with(".fsproj")
        || name.ends_with(".sln")
        || (name.starts_with("requirements") && name.ends_with(".txt"))
    {
        return FileKind::Manifest;
    }

    let lang = language(rel_path);
    let is_doc_lang = matches!(lang, Some("markdown" | "rst" | "asciidoc"));
    let is_code_lang = lang.is_some_and(|l| {
        !matches!(
            l,
            "markdown"
                | "rst"
                | "asciidoc"
                | "json"
                | "yaml"
                | "toml"
                | "xml"
                | "ini"
                | "csv"
                | "text"
        )
    });

    if is_code_lang && is_test_path(&lower, name) {
        return FileKind::Test;
    }
    if is_doc_lang
        || matches!(
            name,
            "license" | "licence" | "copying" | "authors" | "notice"
        )
        || (lang == Some("text") && (lower.starts_with("docs/") || name.starts_with("readme")))
    {
        return FileKind::Doc;
    }
    if matches!(lang, Some("json" | "yaml" | "toml" | "xml" | "ini"))
        || name.starts_with('.')
        || matches!(lang, Some("dockerfile" | "makefile" | "cmake" | "hcl"))
    {
        return FileKind::Config;
    }
    if matches!(lang, Some("csv")) {
        return FileKind::Data;
    }
    if is_code_lang {
        return FileKind::Code;
    }
    FileKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages() {
        assert_eq!(language("src/main.rs"), Some("rust"));
        assert_eq!(language("web/App.tsx"), Some("tsx"));
        assert_eq!(language("Dockerfile"), Some("dockerfile"));
        assert_eq!(language(".gitignore"), None);
        assert_eq!(language("README"), None);
    }

    #[test]
    fn kinds() {
        assert_eq!(kind("src/auth/login.ts"), FileKind::Code);
        assert_eq!(kind("src/auth/login.test.ts"), FileKind::Test);
        assert_eq!(kind("tests/test_payments.py"), FileKind::Test);
        assert_eq!(kind("pkg/cart/cart_test.go"), FileKind::Test);
        assert_eq!(kind("docs/adr/0001.md"), FileKind::Doc);
        assert_eq!(kind("README.md"), FileKind::Doc);
        assert_eq!(kind("package.json"), FileKind::Manifest);
        assert_eq!(kind("requirements-dev.txt"), FileKind::Manifest);
        assert_eq!(kind("Cargo.lock"), FileKind::Lockfile);
        assert_eq!(kind(".github/workflows/ci.yml"), FileKind::Ci);
        assert_eq!(kind("tsconfig.json"), FileKind::Config);
        assert_eq!(kind(".editorconfig"), FileKind::Config);
        assert_eq!(kind("data/users.csv"), FileKind::Data);
        assert_eq!(kind("assets/logo.svg"), FileKind::Other);
        // Documentação dentro de pasta de testes continua sendo doc.
        assert_eq!(kind("tests/README.md"), FileKind::Doc);
    }

    #[test]
    fn binary_extensions() {
        assert!(is_binary_extension("assets/logo.PNG"));
        assert!(is_binary_extension("vendor/lib.dll"));
        assert!(!is_binary_extension("src/lib.rs"));
        assert!(!is_binary_extension(".bin"));
    }
}
