-- Rollback: Remove pilot qualification fields from users table
-- WARNING: This will lose pilot qualification data if not backed up

-- Drop the soft delete index
DROP INDEX IF EXISTS idx_users_deleted_at;

-- Remove pilot qualification columns
ALTER TABLE users DROP COLUMN IF EXISTS deleted_at;
ALTER TABLE users DROP COLUMN IF EXISTS is_examiner;
ALTER TABLE users DROP COLUMN IF EXISTS is_tow_pilot;
ALTER TABLE users DROP COLUMN IF EXISTS is_instructor;
ALTER TABLE users DROP COLUMN IF EXISTS is_licensed;
