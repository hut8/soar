-- Make receiver_id nullable in raw_messages table
-- ADS-B messages don't have a receiver concept, so receiver_id should be NULL for them

-- Drop the foreign key constraint
ALTER TABLE raw_messages
DROP CONSTRAINT IF EXISTS raw_messages_receiver_id_fkey;

-- Make receiver_id nullable
ALTER TABLE raw_messages
ALTER COLUMN receiver_id DROP NOT NULL;

-- Re-add the foreign key constraint (now allows NULL)
ALTER TABLE raw_messages
ADD CONSTRAINT raw_messages_receiver_id_fkey
FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;
