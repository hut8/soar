-- Remove receiver_id column from fixes table
DROP INDEX IF EXISTS idx_fixes_receiver_id;
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_receiver_id_fkey;
ALTER TABLE fixes DROP COLUMN IF EXISTS receiver_id;
