-- Step 5: Create unique index on (receiver_id, received_at, raw_message_hash)
-- WARNING: This will be SLOW on large tables
-- This prevents duplicate APRS messages from being inserted

CREATE UNIQUE INDEX idx_aprs_messages_unique_key
ON aprs_messages (receiver_id, received_at, raw_message_hash);

-- Add helpful comment
COMMENT ON INDEX idx_aprs_messages_unique_key IS
'Prevents duplicate APRS messages on JetStream redelivery after crashes. Natural key: (receiver, timestamp, message hash)';
