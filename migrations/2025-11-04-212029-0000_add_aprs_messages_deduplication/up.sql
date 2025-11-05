-- Add deduplication to aprs_messages to prevent duplicate messages on JetStream redelivery after crashes
-- Uses SHA-256 hash of raw_message for efficient unique constraint

-- Enable pgcrypto extension for SHA-256 hashing (if not already enabled)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Add hash column for raw message content
ALTER TABLE aprs_messages
ADD COLUMN raw_message_hash BYTEA;

-- Compute hashes for existing records
-- This uses SHA-256 hash of the raw message content
UPDATE aprs_messages
SET raw_message_hash = digest(raw_message, 'sha256')
WHERE raw_message_hash IS NULL;

-- Make column NOT NULL after backfill
ALTER TABLE aprs_messages
ALTER COLUMN raw_message_hash SET NOT NULL;

-- Delete any existing duplicates before creating unique index
-- Keep the oldest record (smallest id) for each unique combination
DELETE FROM aprs_messages a
USING aprs_messages b
WHERE a.id > b.id
  AND a.receiver_id = b.receiver_id
  AND a.received_at = b.received_at
  AND a.raw_message = b.raw_message;

-- Create unique index on natural key (receiver_id, received_at, message hash)
-- This prevents the same message from being inserted twice
-- Uses regular CREATE INDEX (not CONCURRENTLY) since this runs in a transaction
CREATE UNIQUE INDEX idx_aprs_messages_unique_key
ON aprs_messages (receiver_id, received_at, raw_message_hash);

-- Add helpful comment
COMMENT ON INDEX idx_aprs_messages_unique_key IS
'Prevents duplicate APRS messages on JetStream redelivery after crashes. Natural key: (receiver, timestamp, message hash)';

COMMENT ON COLUMN aprs_messages.raw_message_hash IS
'SHA-256 hash of raw_message for efficient deduplication';
