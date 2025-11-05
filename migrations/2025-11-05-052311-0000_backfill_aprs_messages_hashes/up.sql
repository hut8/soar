-- Step 2: Backfill SHA-256 hashes for existing records
-- WARNING: This will be SLOW on large tables (computes SHA-256 for every row)
-- Consider running this during low-traffic periods or in batches manually

-- Compute hashes for all existing records
UPDATE aprs_messages
SET raw_message_hash = digest(raw_message, 'sha256')
WHERE raw_message_hash IS NULL;
