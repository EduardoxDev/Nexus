-- v0.2: um import pode apontar para vários arquivos (pacote Go, `package.*` Java, namespace C#).
-- O destino entra na chave. As arestas são sempre recalculadas a partir dos imports já
-- analisados, então a tabela é recriada vazia e o grafo é refeito na próxima indexação.
DROP TABLE edges;

CREATE TABLE edges (
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    from_path  TEXT NOT NULL,
    line       INTEGER NOT NULL,
    specifier  TEXT NOT NULL,
    kind       TEXT NOT NULL,
    to_path    TEXT,
    external   TEXT,
    -- to_path, external ou '' (não resolvido): parte da chave.
    target     TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (project_id, from_path, line, specifier, target)
) STRICT, WITHOUT ROWID;

CREATE INDEX edges_to ON edges (project_id, to_path);

-- Sem estado, o grafo é reconstruído na próxima indexação de cada projeto.
DELETE FROM graph_state;
