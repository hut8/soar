-- Rollback: Remove the raw_message_hash column
ALTER TABLE aprs_messages
DROP COLUMN raw_message_hash;

-- Note: We don't drop pgcrypto extension as other code may depend on it
