-- Revert foreign key constraint back to ON DELETE SET NULL
-- Note: This down migration will fail if receiver_id is still NOT NULL
-- You would need to make receiver_id nullable first

-- Drop the CASCADE constraint
ALTER TABLE aprs_messages
    DROP CONSTRAINT IF EXISTS aprs_messages_receiver_id_fkey;

-- Add back the SET NULL constraint
-- WARNING: This will fail if receiver_id is NOT NULL
ALTER TABLE aprs_messages
    ADD CONSTRAINT aprs_messages_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL;
