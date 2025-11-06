-- Rollback: Make raw_message_hash nullable again and remove trigger

-- Step 1: Drop the NOT NULL constraint
ALTER TABLE aprs_messages
ALTER COLUMN raw_message_hash DROP NOT NULL;

-- Step 2: Drop the trigger
DROP TRIGGER IF EXISTS ensure_aprs_message_hash ON aprs_messages;

-- Step 3: Drop the trigger function
DROP FUNCTION IF EXISTS compute_aprs_message_hash();

-- Restore original comment
COMMENT ON COLUMN aprs_messages.raw_message_hash IS
'SHA-256 hash of raw_message for efficient deduplication';
