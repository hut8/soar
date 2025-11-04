-- Rollback aprs_messages deduplication

-- Drop the unique index
DROP INDEX IF EXISTS idx_aprs_messages_unique_key;

-- Drop the hash column
ALTER TABLE aprs_messages
DROP COLUMN IF EXISTS raw_message_hash;

-- Note: We intentionally don't drop the pgcrypto extension as other parts
-- of the system may depend on it
