-- Rollback fixes deduplication

-- Drop the unique index
DROP INDEX IF EXISTS idx_fixes_unique_key;
