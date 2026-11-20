-- NULL = live. Trashing a folder stamps its whole subtree with one value, so restoring it undoes exactly that
-- action and leaves items that were trashed separately in the trash.
ALTER TABLE folders ADD COLUMN deleted_at TEXT;
ALTER TABLE files ADD COLUMN deleted_at TEXT;
