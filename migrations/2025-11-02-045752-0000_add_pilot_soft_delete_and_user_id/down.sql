-- Remove soft delete and user_id from pilots table
DROP INDEX IF EXISTS idx_pilots_deleted_at;
ALTER TABLE pilots DROP COLUMN user_id;
ALTER TABLE pilots DROP COLUMN deleted_at;
