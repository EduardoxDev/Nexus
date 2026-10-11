use std::time::{SystemTime, UNIX_EPOCH};

/// Instante atual em milissegundos Unix.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Carimbo compacto e ordenável (`20261010T153012Z`) para nomes de arquivo.
pub fn file_stamp(ms: i64) -> String {
    let format = time::macros::format_description!("[year][month][day]T[hour][minute][second]Z");
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .ok()
        .and_then(|t| t.format(format).ok())
        .unwrap_or_else(|| ms.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stamp_is_utc_and_sortable() {
        assert_eq!(file_stamp(0), "19700101T000000Z");
        assert!(file_stamp(1_000) < file_stamp(2_000_000));
    }
}
