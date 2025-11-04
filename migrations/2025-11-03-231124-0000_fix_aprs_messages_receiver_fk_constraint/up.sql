-- Fix foreign key constraint on aprs_messages.receiver_id
-- The constraint was ON DELETE SET NULL but receiver_id is now NOT NULL
-- This caused errors when deleting receivers
-- Change to ON DELETE CASCADE since receiver_id is required

-- Drop the old constraint
ALTER TABLE aprs_messages
    DROP CONSTRAINT IF EXISTS aprs_messages_receiver_id_fkey;

-- Add new constraint with ON DELETE CASCADE
ALTER TABLE aprs_messages
    ADD CONSTRAINT aprs_messages_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;
