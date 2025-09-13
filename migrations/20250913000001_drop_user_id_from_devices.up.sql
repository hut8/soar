-- Drop user_id column from devices table
ALTER TABLE devices DROP COLUMN IF EXISTS user_id;