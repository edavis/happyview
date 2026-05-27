-- SQLite doesn't support ADD CONSTRAINT on existing tables, so we
-- recreate with the check inline.

CREATE TABLE scripts_new (
    id          TEXT PRIMARY KEY,
    body        TEXT NOT NULL,
    description TEXT CHECK (length(description) <= 300),
    script_type TEXT NOT NULL DEFAULT 'lua',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO scripts_new (id, body, description, script_type, created_at, updated_at)
SELECT id, body, substr(description, 1, 300), script_type, created_at, updated_at
FROM scripts;

DROP TABLE scripts;

ALTER TABLE scripts_new RENAME TO scripts;
