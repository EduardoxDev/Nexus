//! Divisão do conteúdo em chunks por linhas e texto de indexação (conteúdo + subpalavras).

use std::collections::BTreeSet;

/// Versão do indexador (chunking + texto do FTS + análise de código). Mudou ⇒ arquivos são
/// reprocessados. 2: símbolos e imports (Fase 6).
pub const INDEXER_VERSION: i64 = 2;

const MAX_CHUNK_LINES: usize = 60;
const MAX_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Linhas 1-based, inclusivas.
    pub start_line: i64,
    pub end_line: i64,
    pub text: String,
}

/// Divide em blocos de até 60 linhas ou 8 KiB (o que vier primeiro), sem cortar linhas.
pub fn chunk_lines(text: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut start = 1i64;
    let mut lines_in_chunk = 0usize;
    let mut line_no = 0i64;

    for line in text.split_inclusive('\n') {
        line_no += 1;
        if lines_in_chunk > 0
            && (lines_in_chunk >= MAX_CHUNK_LINES || current.len() + line.len() > MAX_CHUNK_BYTES)
        {
            chunks.push(Chunk {
                start_line: start,
                end_line: line_no - 1,
                text: std::mem::take(&mut current),
            });
            start = line_no;
            lines_in_chunk = 0;
        }
        current.push_str(line);
        lines_in_chunk += 1;
    }
    if lines_in_chunk > 0 {
        chunks.push(Chunk {
            start_line: start,
            end_line: line_no,
            text: current,
        });
    }
    chunks
}

pub fn line_count(text: &str) -> i64 {
    i64::try_from(text.split_inclusive('\n').count()).unwrap_or(i64::MAX)
}

/// Texto enviado ao FTS: o chunk original seguido das subpalavras de identificadores compostos
/// (`getUserName` → `get user name`, `MAX_RETRY_COUNT` → `max retry count`), para que buscas por
/// partes do nome encontrem o símbolo.
pub fn fts_body(chunk: &str) -> String {
    let mut extra = BTreeSet::new();
    for ident in identifiers(chunk) {
        let parts = split_identifier(ident);
        if parts.len() > 1 {
            extra.insert(parts.join(" "));
        }
    }
    if extra.is_empty() {
        return chunk.to_owned();
    }
    let mut body = String::with_capacity(chunk.len() + extra.len() * 16);
    body.push_str(chunk);
    body.push('\n');
    for (i, words) in extra.iter().enumerate() {
        if i > 0 {
            body.push(' ');
        }
        body.push_str(words);
    }
    body
}

fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|s| {
            s.len() >= 4
                && s.len() <= 80
                && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        })
}

/// Divide camelCase, PascalCase, snake_case e SCREAMING_CASE em palavras minúsculas.
pub fn split_identifier(ident: &str) -> Vec<String> {
    let mut words = Vec::new();
    for part in ident.split('_').filter(|p| !p.is_empty()) {
        let chars: Vec<char> = part.chars().collect();
        let mut word = String::new();
        for (i, &c) in chars.iter().enumerate() {
            let boundary = i > 0
                && ((c.is_ascii_uppercase()
                    && (chars[i - 1].is_ascii_lowercase()
                        || chars[i - 1].is_ascii_digit()
                        || chars.get(i + 1).is_some_and(char::is_ascii_lowercase)
                            && chars[i - 1].is_ascii_uppercase()))
                    || (c.is_ascii_digit() && chars[i - 1].is_ascii_alphabetic()));
            if boundary && !word.is_empty() {
                words.push(std::mem::take(&mut word).to_ascii_lowercase());
            }
            word.push(c);
        }
        if !word.is_empty() {
            words.push(word.to_ascii_lowercase());
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_identifiers() {
        assert_eq!(split_identifier("getUserName"), ["get", "user", "name"]);
        assert_eq!(
            split_identifier("HTTPServerError"),
            ["http", "server", "error"]
        );
        assert_eq!(
            split_identifier("MAX_RETRY_COUNT"),
            ["max", "retry", "count"]
        );
        assert_eq!(
            split_identifier("parse_v2_config"),
            ["parse", "v", "2", "config"]
        );
        assert_eq!(split_identifier("simple"), ["simple"]);
    }

    #[test]
    fn fts_body_adds_subwords_once() {
        let body = fts_body("fn getUserName() { getUserName() }");
        assert!(body.ends_with("\nget user name"));
        assert_eq!(fts_body("nada composto aqui"), "nada composto aqui");
    }

    #[test]
    fn chunks_by_line_count_and_keeps_ranges() {
        let text: String = (1..=130).map(|i| format!("linha {i}\n")).collect();
        let chunks = chunk_lines(&text);
        assert_eq!(chunks.len(), 3);
        assert_eq!((chunks[0].start_line, chunks[0].end_line), (1, 60));
        assert_eq!((chunks[1].start_line, chunks[1].end_line), (61, 120));
        assert_eq!((chunks[2].start_line, chunks[2].end_line), (121, 130));
        assert!(chunks[1].text.starts_with("linha 61\n"));
        assert_eq!(line_count(&text), 130);
    }

    #[test]
    fn chunks_by_size_without_splitting_lines() {
        let long = "x".repeat(5000);
        let text = format!("{long}\n{long}\n{long}\n");
        let chunks = chunk_lines(&text);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|c| c.start_line == c.end_line));
    }

    #[test]
    fn handles_missing_trailing_newline_and_empty() {
        assert_eq!(chunk_lines("a\nb").last().unwrap().end_line, 2);
        assert!(chunk_lines("").is_empty());
        assert_eq!(line_count(""), 0);
    }
}
