/// Uma linha interpretada.
#[derive(Debug, PartialEq, Eq)]
pub enum Line<'a> {
    Empty,
    Comment,
    Pair { key: &'a str, value: &'a str },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

pub fn parse_line(raw: &str, line: usize) -> Result<Line<'_>, ParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Line::Empty);
    }
    if trimmed.starts_with('#') {
        return Ok(Line::Comment);
    }
    let (key, value) = trimmed.split_once('=').ok_or_else(|| ParseError {
        line,
        message: "esperado `chave = valor`".to_owned(),
    })?;
    Ok(Line::Pair { key: key.trim(), value: value.trim() })
}
