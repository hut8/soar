-- Step 1: Enable pgcrypto extension and add raw_message_hash column
-- This is a fast operation that only modifies schema
-- Made idempotent to handle case where old migration already ran

-- Enable pgcrypto extension for SHA-256 hashing (if not already enabled)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Add hash column for raw message content (nullable, stays nullable)
-- IF NOT EXISTS for idempotency
ALTER TABLE aprs_messages
ADD COLUMN IF NOT EXISTS raw_message_hash BYTEA;

-- Add helpful comment (safe to run multiple times)
COMMENT ON COLUMN aprs_messages.raw_message_hash IS
'SHA-256 hash of raw_message for efficient deduplication';
