-- A share now targets exactly one of file_id / folder_id. SQLite can't drop NOT NULL, so rebuild the table.
CREATE TABLE shares_new (
    id             INTEGER PRIMARY KEY,
    token          TEXT NOT NULL UNIQUE,
    file_id        INTEGER REFERENCES files(id) ON DELETE CASCADE,
    folder_id      INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_hash  TEXT,
    expires_at     TEXT, -- UTC 'YYYY-MM-DD HH:MM:SS', NULL = never
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    download_count INTEGER NOT NULL DEFAULT 0,
    CHECK ((file_id IS NULL) <> (folder_id IS NULL))
);
INSERT INTO shares_new (id, token, file_id, user_id, password_hash, expires_at, created_at, download_count)
    SELECT id, token, file_id, user_id, password_hash, expires_at, created_at, download_count FROM shares;
DROP TABLE shares;
ALTER TABLE shares_new RENAME TO shares;
CREATE INDEX shares_file ON shares(file_id);
CREATE INDEX shares_folder ON shares(folder_id);
