-- Remove aprs_message_id column from receiver_statuses
DROP INDEX IF EXISTS idx_receiver_statuses_aprs_message_id;
ALTER TABLE receiver_statuses DROP COLUMN IF EXISTS aprs_message_id;

-- Remove aprs_message_id column from fixes
DROP INDEX IF EXISTS idx_fixes_aprs_message_id;
ALTER TABLE fixes DROP COLUMN IF EXISTS aprs_message_id;

-- Drop aprs_messages table
DROP TRIGGER IF EXISTS set_aprs_messages_updated_at ON aprs_messages;
DROP INDEX IF EXISTS idx_aprs_messages_received_at;
DROP TABLE IF EXISTS aprs_messages;
