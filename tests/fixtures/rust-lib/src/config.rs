use std::collections::BTreeMap;

use crate::parser::{parse_line, Line, ParseError};

#[derive(Debug, Default)]
pub struct Config {
    values: BTreeMap<String, String>,
}

impl Config {
    pub fn from_str(text: &str) -> Result<Self, ParseError> {
        let mut values = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            if let Line::Pair { key, value } = parse_line(raw, index + 1)? {
                if values.insert(key.to_owned(), value.to_owned()).is_some() {
                    return Err(ParseError { line: index + 1, message: format!("chave duplicada: {key}") });
                }
            }
        }
        Ok(Self { values })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}
