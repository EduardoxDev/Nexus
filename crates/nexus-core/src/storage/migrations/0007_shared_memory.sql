-- v0.3: memória compartilhada pelo repositório (`.nexus/memory/`, ADR-0014).
-- Nova origem `shared_file` (memória lida de um arquivo do repositório) e novo autor de
-- revisão `shared_file`. O SQLite não altera um CHECK no lugar: as duas tabelas são recriadas
-- com os mesmos dados (procedimento da documentação do SQLite, com as chaves estrangeiras
-- desligadas pelo migrador e conferidas com `foreign_key_check` antes do commit).

CREATE TABLE memories_v7 (
    id            TEXT PRIMARY KEY NOT NULL,
    project_id    INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL
                  CHECK (kind IN ('decision', 'convention', 'requirement', 'constraint', 'bug',
                                  'solution', 'dependency', 'task', 'lesson')),
    status        TEXT NOT NULL
                  CHECK (status IN ('proposed', 'confirmed', 'archived', 'superseded')),
    title         TEXT NOT NULL,
    body          TEXT NOT NULL,
    origin        TEXT NOT NULL
                  CHECK (origin IN ('user', 'cli', 'adr_import', 'agent', 'shared_file')),
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

INSERT INTO memories_v7 (id, project_id, kind, status, title, body, origin, origin_path,
                         origin_hash, supersedes, superseded_by, revision, created_at,
                         updated_at, confirmed_at)
SELECT id, project_id, kind, status, title, body, origin, origin_path, origin_hash, supersedes,
       superseded_by, revision, created_at, updated_at, confirmed_at
FROM memories;

DROP TABLE memories;
ALTER TABLE memories_v7 RENAME TO memories;

CREATE INDEX memories_project_status ON memories (project_id, status, updated_at DESC);
CREATE INDEX memories_origin_path ON memories (project_id, origin_path);

-- Os gatilhos do índice textual são da tabela antiga e saem com ela.
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

CREATE TABLE memory_revisions_v7 (
    memory_id   TEXT NOT NULL REFERENCES memories (id) ON DELETE CASCADE,
    revision    INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    status      TEXT NOT NULL,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    refs_json   TEXT NOT NULL CHECK (json_valid(refs_json)),
    actor       TEXT NOT NULL
                CHECK (actor IN ('user', 'cli', 'adr_import', 'agent', 'system', 'shared_file')),
    change      TEXT NOT NULL,
    note        TEXT,
    changed_at  INTEGER NOT NULL,
    PRIMARY KEY (memory_id, revision)
) STRICT;

INSERT INTO memory_revisions_v7 (memory_id, revision, kind, status, title, body, refs_json, actor,
                                 change, note, changed_at)
SELECT memory_id, revision, kind, status, title, body, refs_json, actor, change, note, changed_at
FROM memory_revisions;

DROP TABLE memory_revisions;
ALTER TABLE memory_revisions_v7 RENAME TO memory_revisions;
