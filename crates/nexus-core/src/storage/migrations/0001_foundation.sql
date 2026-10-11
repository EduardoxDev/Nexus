-- Migration 1: fundação (configurações e histórico local de operações).

CREATE TABLE settings (
    key         TEXT PRIMARY KEY NOT NULL,
    value_json  TEXT NOT NULL CHECK (json_valid(value_json)),
    updated_at  INTEGER NOT NULL
) STRICT;

CREATE TABLE operations (
    id            TEXT PRIMARY KEY NOT NULL,
    kind          TEXT NOT NULL,
    project_id    INTEGER,
    status        TEXT NOT NULL
                  CHECK (status IN ('running', 'succeeded', 'failed', 'cancelled', 'interrupted')),
    started_at    INTEGER NOT NULL,
    heartbeat_at  INTEGER NOT NULL,
    finished_at   INTEGER,
    duration_ms   INTEGER,
    detail_json   TEXT CHECK (detail_json IS NULL OR json_valid(detail_json)),
    error_code    TEXT,
    error_message TEXT
) STRICT;

CREATE INDEX operations_started_at ON operations (started_at DESC);
CREATE INDEX operations_kind_started_at ON operations (kind, started_at DESC);
CREATE INDEX operations_running ON operations (status) WHERE status = 'running';
