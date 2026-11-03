CREATE TABLE shares (
    id            INTEGER PRIMARY KEY,
    token         TEXT NOT NULL UNIQUE,
    file_id       INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT,
    expires_at    TEXT, -- UTC 'YYYY-MM-DD HH:MM:SS', NULL = never
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX shares_file ON shares(file_id);
