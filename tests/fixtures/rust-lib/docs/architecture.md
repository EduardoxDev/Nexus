# Arquitetura

`parser::parse_line` interpreta uma linha; `config::Config::from_str` agrega as linhas e
rejeita chaves duplicadas. Comentários começam com `#`.
