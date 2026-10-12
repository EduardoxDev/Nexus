-- Migration 5: análise de código (Fase 6). Símbolos e imports são derivados do CONTEÚDO
-- (endereçado por hash, como os chunks): arquivos idênticos compartilham a análise e somem
-- em cascata com o conteúdo. As arestas do grafo dependem do CAMINHO (imports relativos),
-- então são recalculadas por arquivo quando o conjunto de arquivos muda.

CREATE TABLE symbols (
    id           INTEGER PRIMARY KEY,
    project_id   INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    name         TEXT NOT NULL,
    -- minúsculas, sem `_`/`$`: `validate_session` ≈ `validateSession`
    name_norm    TEXT NOT NULL,
    kind         TEXT NOT NULL
                 CHECK (kind IN ('function', 'method', 'class', 'struct', 'enum', 'interface',
                                 'trait', 'type', 'const', 'variable', 'module', 'macro')),
    line         INTEGER NOT NULL,
    end_line     INTEGER NOT NULL,
    exported     INTEGER NOT NULL CHECK (exported IN (0, 1)),
    container    TEXT,
    FOREIGN KEY (project_id, content_hash) REFERENCES contents (project_id, content_hash)
        ON DELETE CASCADE
) STRICT;

CREATE INDEX symbols_name ON symbols (project_id, name_norm);
CREATE INDEX symbols_content ON symbols (project_id, content_hash);

CREATE TABLE imports (
    project_id   INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    line         INTEGER NOT NULL,
    specifier    TEXT NOT NULL,
    kind         TEXT NOT NULL CHECK (kind IN ('import', 'reexport', 'require', 'dynamic', 'module')),
    PRIMARY KEY (project_id, content_hash, ordinal),
    FOREIGN KEY (project_id, content_hash) REFERENCES contents (project_id, content_hash)
        ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- Grafo de dependências: uma aresta por import de cada arquivo, com a evidência (linha e
-- especificador). `to_path` = arquivo do projeto; `external` = pacote/crate; ambos nulos =
-- não resolvido.
CREATE TABLE edges (
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    from_path  TEXT NOT NULL,
    line       INTEGER NOT NULL,
    specifier  TEXT NOT NULL,
    kind       TEXT NOT NULL,
    to_path    TEXT,
    external   TEXT,
    PRIMARY KEY (project_id, from_path, line, specifier)
) STRICT, WITHOUT ROWID;

CREATE INDEX edges_to ON edges (project_id, to_path);

CREATE TABLE graph_state (
    project_id       INTEGER PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    index_generation INTEGER NOT NULL,
    built_at         INTEGER NOT NULL,
    duration_ms      INTEGER NOT NULL,
    files            INTEGER NOT NULL,
    edges            INTEGER NOT NULL,
    resolved         INTEGER NOT NULL,
    external         INTEGER NOT NULL,
    unresolved       INTEGER NOT NULL
) STRICT;
