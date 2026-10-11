use tinyconf::Config;

#[test]
fn parses_pairs_and_ignores_comments() {
    let config = Config::from_str("# comentário\nport = 8080\nhost = localhost\n").unwrap();
    assert_eq!(config.get("port"), Some("8080"));
}

#[test]
fn rejects_duplicate_keys() {
    assert!(Config::from_str("a = 1\na = 2").is_err());
}
