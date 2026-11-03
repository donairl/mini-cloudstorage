-- NULL = use the server default (QUOTA_MB).
ALTER TABLE users ADD COLUMN quota_bytes INTEGER;
