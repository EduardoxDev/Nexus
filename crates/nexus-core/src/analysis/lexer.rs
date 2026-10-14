//! Lexer mínimo: mascara comentários e conteúdo de strings com espaços, **preservando o
//! comprimento em bytes e as quebras de linha** — offsets e números de linha continuam valendo
//! no texto original. Não é um parser: só evita que `// function x` ou `"class Y"` virem
//! declarações e que chaves dentro de strings desequilibrem a contagem de blocos.

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub line_comment: &'static str,
    /// Segundo marcador de comentário de linha (`#` no PHP, que não vale para `#[`).
    pub alt_line_comment: Option<&'static str>,
    pub block_comment: Option<(&'static str, &'static str)>,
    /// Comentários de bloco aninháveis (Rust).
    pub nested_blocks: bool,
    pub quotes: &'static [char],
    /// Strings com três aspas (Python).
    pub triple_quotes: bool,
    /// Strings brutas `r"…"`/`r#"…"#` e distinção entre `'a'` (caractere) e `'a` (lifetime).
    pub rust: bool,
}

pub const C_LIKE: Style = Style {
    line_comment: "//",
    alt_line_comment: None,
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    quotes: &['"', '\'', '`'],
    triple_quotes: false,
    rust: false,
};

/// PHP: `//` e `#` (menos `#[`, atributo do PHP 8), `/* */`, aspas simples, duplas e crases.
pub const PHP: Style = Style {
    line_comment: "//",
    alt_line_comment: Some("#"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    quotes: &['"', '\'', '`'],
    triple_quotes: false,
    rust: false,
};

/// Ruby: `#` (blocos `=begin`/`=end` não são tratados), aspas simples, duplas e crases.
pub const RUBY: Style = Style {
    line_comment: "#",
    alt_line_comment: None,
    block_comment: None,
    nested_blocks: false,
    quotes: &['"', '\'', '`'],
    triple_quotes: false,
    rust: false,
};

pub const PYTHON: Style = Style {
    line_comment: "#",
    alt_line_comment: None,
    block_comment: None,
    nested_blocks: false,
    quotes: &['"', '\''],
    triple_quotes: true,
    rust: false,
};

pub const RUST: Style = Style {
    line_comment: "//",
    alt_line_comment: None,
    block_comment: Some(("/*", "*/")),
    nested_blocks: true,
    quotes: &['"'],
    triple_quotes: false,
    rust: true,
};

/// Duas versões do texto, ambas com o mesmo comprimento do original:
/// - `code`: comentários mascarados, strings intactas (para ler especificadores de import);
/// - `bare`: comentários **e** conteúdo de strings mascarados (para declarações e blocos).
pub struct Masked {
    pub code: String,
    pub bare: String,
}

fn blank(out: &mut Vec<u8>, bytes: &[u8]) {
    for &b in bytes {
        out.push(if b == b'\n' { b'\n' } else { b' ' });
    }
}

pub fn mask(text: &str, style: Style) -> Masked {
    let src = text.as_bytes();
    let mut code = Vec::with_capacity(src.len());
    let mut bare = Vec::with_capacity(src.len());
    let mut i = 0;
    let starts = |i: usize, pat: &str| src[i..].starts_with(pat.as_bytes());

    while i < src.len() {
        // Comentário de linha.
        let alt = style
            .alt_line_comment
            .is_some_and(|m| starts(i, m) && src.get(i + m.len()) != Some(&b'['));
        if starts(i, style.line_comment) || alt {
            let end = src[i..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(src.len(), |p| i + p);
            blank(&mut code, &src[i..end]);
            blank(&mut bare, &src[i..end]);
            i = end;
            continue;
        }
        // Comentário de bloco.
        if let Some((open, close)) = style.block_comment
            && starts(i, open)
        {
            let mut depth = 0usize;
            let mut j = i;
            while j < src.len() {
                if starts(j, open) && (style.nested_blocks || depth == 0) {
                    depth += 1;
                    j += open.len();
                } else if starts(j, close) {
                    depth -= 1;
                    j += close.len();
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            let end = j.min(src.len());
            blank(&mut code, &src[i..end]);
            blank(&mut bare, &src[i..end]);
            i = end;
            continue;
        }
        // Strings brutas do Rust: r"…", r#"…"#, br"…".
        if style.rust
            && (src[i] == b'r' || (src[i] == b'b' && src.get(i + 1) == Some(&b'r')))
            && (i == 0 || !(src[i - 1].is_ascii_alphanumeric() || src[i - 1] == b'_'))
        {
            let r = if src[i] == b'b' { i + 1 } else { i };
            let hashes = src[r + 1..].iter().take_while(|&&b| b == b'#').count();
            if src.get(r + 1 + hashes) == Some(&b'"') {
                let body = r + 2 + hashes;
                let mut closing = String::from("\"");
                closing.push_str(&"#".repeat(hashes));
                let end = src[body..]
                    .windows(closing.len())
                    .position(|w| w == closing.as_bytes())
                    .map_or(src.len(), |p| body + p + closing.len());
                code.extend_from_slice(&src[i..end]);
                bare.extend_from_slice(&src[i..body]);
                blank(
                    &mut bare,
                    &src[body..end.saturating_sub(closing.len()).max(body)],
                );
                bare.extend_from_slice(&src[end.saturating_sub(closing.len()).max(body)..end]);
                i = end;
                continue;
            }
        }
        // Rust: caractere `'x'`/`'\n'` é string; `'a` (lifetime) não.
        if style.rust && src[i] == b'\'' {
            let close = if src.get(i + 1) == Some(&b'\\') {
                src[i + 2..]
                    .iter()
                    .take(10)
                    .position(|&b| b == b'\'')
                    .map(|p| i + 2 + p)
            } else {
                // Um caractere UTF-8 (1 a 4 bytes) seguido de aspa.
                (2..=5)
                    .find(|&n| src.get(i + n) == Some(&b'\''))
                    .filter(|&n| {
                        std::str::from_utf8(&src[i + 1..i + n])
                            .is_ok_and(|s| s.chars().count() == 1)
                    })
                    .map(|n| i + n)
            };
            if let Some(c) = close {
                code.extend_from_slice(&src[i..=c]);
                bare.push(b'\'');
                blank(&mut bare, &src[i + 1..c]);
                bare.push(b'\'');
                i = c + 1;
                continue;
            }
            code.push(src[i]);
            bare.push(src[i]);
            i += 1;
            continue;
        }
        // Strings comuns (e triplas no Python).
        let c = src[i] as char;
        if style.quotes.contains(&c) {
            let triple = style.triple_quotes && src[i..].starts_with(&[src[i]; 3]);
            let delim_len = if triple { 3 } else { 1 };
            let mut j = i + delim_len;
            let mut end = src.len();
            while j < src.len() {
                if src[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if !triple && src[j] == b'\n' && c != '`' {
                    end = j; // string não terminada: para no fim da linha
                    break;
                }
                if src[j..].starts_with(&src[i..i + delim_len]) {
                    end = j + delim_len;
                    break;
                }
                j += 1;
            }
            let end = end.min(src.len());
            code.extend_from_slice(&src[i..end]);
            let inner_end =
                if end >= i + 2 * delim_len && src[end - delim_len..end] == src[i..i + delim_len] {
                    end - delim_len
                } else {
                    end
                };
            bare.extend_from_slice(&src[i..i + delim_len]);
            blank(&mut bare, &src[i + delim_len..inner_end]);
            bare.extend_from_slice(&src[inner_end..end]);
            i = end;
            continue;
        }
        code.push(src[i]);
        bare.push(src[i]);
        i += 1;
    }
    // Trechos mascarados começam e terminam em delimitadores ASCII, então caracteres multibyte
    // são copiados ou mascarados inteiros: o UTF-8 continua válido. A substituição é só defesa.
    Masked {
        code: String::from_utf8(code)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
        bare: String::from_utf8(bare)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
    }
}

/// Número da linha (1-based) de um offset em bytes.
pub fn line_of(text: &str, offset: usize) -> i64 {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as i64
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_comments_and_strings_keeping_lengths() {
        let src = "a // function x\nb /* class\nY */ c \"str { }\" 'ç'\n";
        let m = mask(src, C_LIKE);
        assert_eq!(m.code.len(), src.len());
        assert_eq!(m.bare.len(), src.len());
        assert!(!m.code.contains("function"));
        assert!(!m.bare.contains("class"));
        assert!(m.code.contains("\"str { }\""));
        assert!(!m.bare.contains('{'));
        assert_eq!(m.bare.lines().count(), src.lines().count());
    }

    #[test]
    fn python_triple_quotes_and_hash_comments() {
        let src = "def f():\n    \"\"\"doc\nclass X:\n\"\"\"\n    # def g\n    return 'x'\n";
        let m = mask(src, PYTHON);
        assert!(m.bare.contains("def f"));
        assert!(!m.bare.contains("class X"));
        assert!(!m.bare.contains("def g"));
        assert_eq!(m.bare.len(), src.len());
    }

    #[test]
    fn rust_raw_strings_chars_and_lifetimes() {
        let src = "fn a<'a>(x: &'a str) -> char { let s = r#\"fn fake() { \"#; '{' }\n/* /* nested */ fn no() */ fn yes() {}\n";
        let m = mask(src, RUST);
        assert!(m.bare.contains("fn a<'a>(x: &'a str)"));
        assert!(!m.bare.contains("fn fake"));
        assert!(m.code.contains("fn fake"), "strings ficam em `code`");
        assert!(!m.bare.contains("fn no"));
        assert!(m.bare.contains("fn yes"));
        assert_eq!(m.bare.matches('{').count(), 2);
        assert_eq!(m.bare.len(), src.len());
    }

    #[test]
    fn php_hash_comments_but_not_attributes() {
        let src = "# class A {}\n#[Route('/x')]\nclass B {}\n";
        let m = mask(src, PHP);
        assert!(!m.bare.contains("class A"));
        assert!(m.bare.contains("#[Route"));
        assert!(m.bare.contains("class B"));
    }

    #[test]
    fn template_literals_span_lines() {
        let src = "const q = `\nfunction inside() {}\n`;\nfunction outside() {}\n";
        let m = mask(src, C_LIKE);
        assert!(!m.bare.contains("inside"));
        assert!(m.bare.contains("function outside"));
        assert_eq!(line_of(src, src.find("function outside").unwrap_or(0)), 4);
    }
}
