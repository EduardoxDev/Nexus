-- Migration 2: projetos autorizados, arquivos, conteúdo endereçado por hash, FTS e jobs de indexação.
-- O código indexado NÃO é armazenado (ADR-0003): o FTS é contentless; trechos são lidos do disco.

CREATE TABLE projects (
    id               INTEGER PRIMARY KEY,
    name             TEXT NOT NULL,
    root_path        TEXT NOT NULL UNIQUE,
    created_at       INTEGER NOT NULL,
    last_opened_at   INTEGER NOT NULL,
    last_indexed_at  INTEGER,
    index_generation INTEGER NOT NULL DEFAULT 0,
    head_ref         TEXT,
    head_commit      TEXT
) STRICT;

-- Conteúdo único por projeto (deduplicação: arquivos idênticos compartilham a mesma linha).
CREATE TABLE contents (
    project_id      INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    content_hash    TEXT NOT NULL,
    line_count      INTEGER NOT NULL,
    chunk_count     INTEGER NOT NULL,
    indexer_version INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (project_id, content_hash)
) STRICT, WITHOUT ROWID;

CREATE TABLE files (
    id              INTEGER PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    rel_path        TEXT NOT NULL,
    size            INTEGER NOT NULL,
    mtime_ns        INTEGER NOT NULL,
    content_hash    TEXT,
    language        TEXT,
    kind            TEXT NOT NULL
                    CHECK (kind IN ('code', 'test', 'doc', 'config', 'manifest', 'lockfile', 'ci', 'data', 'other')),
    status          TEXT NOT NULL
                    CHECK (status IN ('indexed', 'skipped_sensitive', 'skipped_binary', 'skipped_large',
                                      'skipped_encoding', 'error')),
    skip_reason     TEXT,
    line_count      INTEGER,
    first_seen_at   INTEGER NOT NULL,
    indexed_at      INTEGER NOT NULL,
    change_count    INTEGER NOT NULL DEFAULT 0,
    indexer_version INTEGER NOT NULL,
    UNIQUE (project_id, rel_path),
    FOREIGN KEY (project_id, content_hash) REFERENCES contents (project_id, content_hash)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE INDEX files_project_status ON files (project_id, status);
CREATE INDEX files_project_hash ON files (project_id, content_hash);

CREATE TABLE chunks (
    id           INTEGER PRIMARY KEY,
    project_id   INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    start_line   INTEGER NOT NULL,
    end_line     INTEGER NOT NULL,
    chunk_hash   TEXT NOT NULL,
    UNIQUE (project_id, content_hash, ordinal),
    FOREIGN KEY (project_id, content_hash) REFERENCES contents (project_id, content_hash)
        ON DELETE CASCADE
) STRICT;

-- Índice textual sem cópia do conteúdo. rowid = chunks.id.
CREATE VIRTUAL TABLE chunks_fts USING fts5 (
    body,
    content = '',
    contentless_delete = 1,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Mantém o FTS consistente quando chunks somem (inclusive por cascata).
CREATE TRIGGER chunks_after_delete AFTER DELETE ON chunks BEGIN
    DELETE FROM chunks_fts WHERE rowid = old.id;
END;

CREATE TABLE indexing_jobs (
    id              TEXT PRIMARY KEY NOT NULL,
    project_id      INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    mode            TEXT NOT NULL CHECK (mode IN ('full', 'incremental')),
    trigger         TEXT NOT NULL CHECK (trigger IN ('user', 'watcher', 'reconcile', 'cli', 'resume')),
    status          TEXT NOT NULL
                    CHECK (status IN ('running', 'paused', 'succeeded', 'failed', 'cancelled', 'interrupted')),
    phase           TEXT NOT NULL CHECK (phase IN ('scanning', 'indexing', 'finalizing', 'done')),
    pid             INTEGER NOT NULL,
    started_at      INTEGER NOT NULL,
    heartbeat_at    INTEGER NOT NULL,
    finished_at     INTEGER,
    discovered      INTEGER NOT NULL DEFAULT 0,
    to_process      INTEGER NOT NULL DEFAULT 0,
    processed       INTEGER NOT NULL DEFAULT 0,
    unchanged       INTEGER NOT NULL DEFAULT 0,
    new_contents    INTEGER NOT NULL DEFAULT 0,
    reused_contents INTEGER NOT NULL DEFAULT 0,
    skipped         INTEGER NOT NULL DEFAULT 0,
    removed         INTEGER NOT NULL DEFAULT 0,
    renamed         INTEGER NOT NULL DEFAULT 0,
    failed          INTEGER NOT NULL DEFAULT 0,
    bytes_read      INTEGER NOT NULL DEFAULT 0,
    generation      INTEGER,
    error_message   TEXT
) STRICT;

-- Trava entre processos: no máximo um job ativo por projeto.
CREATE UNIQUE INDEX indexing_jobs_one_active ON indexing_jobs (project_id)
    WHERE status IN ('running', 'paused');
CREATE INDEX indexing_jobs_project_started ON indexing_jobs (project_id, started_at DESC);

CREATE TABLE index_errors (
    id         INTEGER PRIMARY KEY,
    job_id     TEXT NOT NULL REFERENCES indexing_jobs (id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    rel_path   TEXT NOT NULL,
    code       TEXT NOT NULL,
    message    TEXT NOT NULL,
    at         INTEGER NOT NULL
) STRICT;

CREATE INDEX index_errors_job ON index_errors (job_id);
