-- Step 3: Make raw_message_hash NOT NULL
-- This is fast - just adds a constraint after backfill is complete

ALTER TABLE aprs_messages
ALTER COLUMN raw_message_hash SET NOT NULL;
