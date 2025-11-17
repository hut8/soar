-- Rollback: Drop source_metadata column and indexes

DROP INDEX IF EXISTS idx_fixes_protocol;
DROP INDEX IF EXISTS idx_fixes_source_metadata;
ALTER TABLE fixes DROP COLUMN IF EXISTS source_metadata;
