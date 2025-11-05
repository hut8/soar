-- Rollback: Allow NULL values again
ALTER TABLE aprs_messages
ALTER COLUMN raw_message_hash DROP NOT NULL;
