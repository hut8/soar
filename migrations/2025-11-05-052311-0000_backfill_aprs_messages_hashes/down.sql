-- Rollback: Clear all hashes (set back to NULL)
UPDATE aprs_messages
SET raw_message_hash = NULL;
