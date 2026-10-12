-- Migration 4: memória do projeto (com revisões completas) e alertas de atualidade.
-- A memória é texto escrito ou confirmado pelo usuário; referências a arquivos guardam caminho,
-- linhas e o hash do arquivo no momento do vínculo — nunca cópia do código (ADR-0003, ADR-0011).

CREATE TABLE memories (
    id            TEXT PRIMARY KEY NOT NULL,
    project_id    INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL
                  CHECK (kind IN ('decision', 'convention', 'requirement', 'constraint', 'bug',
                                  'solution', 'dependency', 'task', 'lesson')),
    status        TEXT NOT NULL
                  CHECK (status IN ('proposed', 'confirmed', 'archived', 'superseded')),
    title         TEXT NOT NULL,
    body          TEXT NOT NULL,
    origin        TEXT NOT NULL CHECK (origin IN ('user', 'cli', 'adr_import', 'agent')),
    -- Documento de origem (importação) e hash dele no momento da importação.
    origin_path   TEXT,
    origin_hash   TEXT,
    -- Memória confirmada que esta proposta pretende substituir.
    supersedes    TEXT REFERENCES memories (id) ON DELETE SET NULL,
    superseded_by TEXT REFERENCES memories (id) ON DELETE SET NULL,
    revision      INTEGER NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    confirmed_at  INTEGER
) STRICT;

CREATE INDEX memories_project_status ON memories (project_id, status, updated_at DESC);
CREATE INDEX memories_origin_path ON memories (project_id, origin_path);

CREATE TABLE memory_refs (
    memory_id    TEXT NOT NULL REFERENCES memories (id) ON DELETE CASCADE,
    ordinal      INTEGER NOT NULL,
    rel_path     TEXT NOT NULL,
    start_line   INTEGER,
    end_line     INTEGER,
    -- Hash do arquivo indexado quando a referência foi criada ou revisada.
    content_hash TEXT NOT NULL,
    PRIMARY KEY (memory_id, ordinal)
) STRICT;

CREATE INDEX memory_refs_path ON memory_refs (rel_path);

-- Histórico completo: cada revisão é um retrato integral (título, corpo, estado, referências).
CREATE TABLE memory_revisions (
    memory_id   TEXT NOT NULL REFERENCES memories (id) ON DELETE CASCADE,
    revision    INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    status      TEXT NOT NULL,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    refs_json   TEXT NOT NULL CHECK (json_valid(refs_json)),
    actor       TEXT NOT NULL CHECK (actor IN ('user', 'cli', 'adr_import', 'agent', 'system')),
    change      TEXT NOT NULL,
    note        TEXT,
    changed_at  INTEGER NOT NULL,
    PRIMARY KEY (memory_id, revision)
) STRICT;

-- Busca textual na memória (o texto da memória já vive em `memories`; esta cópia é só o índice).
CREATE VIRTUAL TABLE memories_fts USING fts5 (
    memory_id UNINDEXED,
    title,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER memories_after_insert AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts (memory_id, title, body) VALUES (new.id, new.title, new.body);
END;

CREATE TRIGGER memories_after_update AFTER UPDATE OF title, body ON memories BEGIN
    DELETE FROM memories_fts WHERE memory_id = old.id;
    INSERT INTO memories_fts (memory_id, title, body) VALUES (new.id, new.title, new.body);
END;

CREATE TRIGGER memories_after_delete AFTER DELETE ON memories BEGIN
    DELETE FROM memories_fts WHERE memory_id = old.id;
END;

-- Alertas de atualidade. `fingerprint` identifica o mesmo problema entre verificações, para que
-- um alerta dispensado continue dispensado e um alerta corrigido seja marcado como resolvido.
CREATE TABLE freshness_issues (
    id                 INTEGER PRIMARY KEY,
    project_id         INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    kind               TEXT NOT NULL
                       CHECK (kind IN ('missing_path', 'missing_symbol', 'memory_ref_missing',
                                       'memory_ref_changed', 'memory_source_changed',
                                       'superseded_source', 'duplicate_memory', 'files_changed')),
    fingerprint        TEXT NOT NULL,
    severity           TEXT NOT NULL CHECK (severity IN ('info', 'warning')),
    status             TEXT NOT NULL CHECK (status IN ('open', 'dismissed', 'resolved')),
    needs_review       INTEGER NOT NULL CHECK (needs_review IN (0, 1)),
    subject_path       TEXT,
    memory_id          TEXT REFERENCES memories (id) ON DELETE CASCADE,
    related_memory_id  TEXT REFERENCES memories (id) ON DELETE CASCADE,
    detail_json        TEXT NOT NULL CHECK (json_valid(detail_json)),
    first_detected_at  INTEGER NOT NULL,
    last_detected_at   INTEGER NOT NULL,
    resolved_at        INTEGER,
    dismissed_at       INTEGER,
    dismiss_note       TEXT,
    UNIQUE (project_id, fingerprint)
) STRICT;

CREATE INDEX freshness_issues_project_status ON freshness_issues (project_id, status, kind);
CREATE INDEX freshness_issues_memory ON freshness_issues (memory_id);

-- Última verificação por projeto (para a interface e para decidir se vale reverificar).
CREATE TABLE freshness_scans (
    project_id       INTEGER PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    scanned_at       INTEGER NOT NULL,
    index_generation INTEGER NOT NULL,
    duration_ms      INTEGER NOT NULL,
    docs_checked     INTEGER NOT NULL,
    memories_checked INTEGER NOT NULL,
    open_issues      INTEGER NOT NULL,
    -- Identificadores citados em documentos que existiam no código na última verificação:
    -- permite distinguir "removido do código" de "nunca encontrado" (ex.: API externa).
    seen_symbols_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(seen_symbols_json))
) STRICT;
