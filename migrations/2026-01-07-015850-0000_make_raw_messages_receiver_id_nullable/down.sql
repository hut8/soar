-- Reverse the migration: make receiver_id NOT NULL again
-- WARNING: This will fail if there are any rows with NULL receiver_id

-- Drop the foreign key constraint
ALTER TABLE raw_messages
DROP CONSTRAINT IF EXISTS raw_messages_receiver_id_fkey;

-- Make receiver_id NOT NULL (will fail if there are NULL values)
ALTER TABLE raw_messages
ALTER COLUMN receiver_id SET NOT NULL;

-- Re-add the foreign key constraint
ALTER TABLE raw_messages
ADD CONSTRAINT raw_messages_receiver_id_fkey
FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;
