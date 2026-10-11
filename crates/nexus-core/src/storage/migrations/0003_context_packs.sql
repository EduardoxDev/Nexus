-- Migration 3: Context Packs. Guardam o plano e as referências (caminho, linhas, hash), nunca o
-- texto do código: os trechos são lidos do disco e verificados ao abrir ou exportar (ADR-0003).

CREATE TABLE context_packs (
    id               TEXT PRIMARY KEY NOT NULL,
    project_id       INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    task             TEXT NOT NULL,
    created_at       INTEGER NOT NULL,
    index_generation INTEGER NOT NULL,
    head_commit      TEXT,
    token_budget     INTEGER NOT NULL,
    estimated_tokens INTEGER NOT NULL,
    file_count       INTEGER NOT NULL,
    plan_json        TEXT NOT NULL CHECK (json_valid(plan_json))
) STRICT;

CREATE INDEX context_packs_project_created ON context_packs (project_id, created_at DESC);

CREATE TABLE context_sources (
    pack_id      TEXT NOT NULL REFERENCES context_packs (id) ON DELETE CASCADE,
    ordinal      INTEGER NOT NULL,
    rel_path     TEXT NOT NULL,
    role         TEXT NOT NULL,
    start_line   INTEGER NOT NULL,
    end_line     INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    score        REAL NOT NULL,
    PRIMARY KEY (pack_id, ordinal)
) STRICT;

CREATE INDEX context_sources_path ON context_sources (rel_path);
