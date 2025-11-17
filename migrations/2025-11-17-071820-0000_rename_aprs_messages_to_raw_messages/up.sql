-- Rename aprs_messages table to raw_messages to support both APRS and Beast protocols
-- Add source enum to distinguish between protocol types

-- Step 1: Create message_source enum type
CREATE TYPE message_source AS ENUM ('aprs', 'beast');

-- Step 2: Rename the table and its constraints
ALTER TABLE aprs_messages RENAME TO raw_messages;

-- Step 3: Rename indexes
ALTER INDEX IF EXISTS idx_aprs_messages_received_at RENAME TO idx_raw_messages_received_at;
ALTER INDEX IF EXISTS idx_aprs_messages_receiver_id RENAME TO idx_raw_messages_receiver_id;
ALTER INDEX IF EXISTS idx_aprs_messages_raw_message_hash RENAME TO idx_raw_messages_raw_message_hash;

-- Step 4: Rename sequences (if any UUID defaults)
-- (aprs_messages uses gen_random_uuid(), no sequence to rename)

-- Step 5: Add source column with default 'aprs' for existing data
ALTER TABLE raw_messages ADD COLUMN source message_source NOT NULL DEFAULT 'aprs';

-- Step 6: Rename foreign key column in fixes table
ALTER TABLE fixes RENAME COLUMN aprs_message_id TO raw_message_id;

-- Step 7: Rename foreign key constraint in fixes
ALTER TABLE fixes RENAME CONSTRAINT fixes_aprs_message_id_fkey TO fixes_raw_message_id_fkey;

-- Step 8: Rename index in fixes
ALTER INDEX IF EXISTS idx_fixes_aprs_message_id RENAME TO idx_fixes_raw_message_id;

-- Step 9: Rename foreign key column in receiver_statuses table
ALTER TABLE receiver_statuses RENAME COLUMN aprs_message_id TO raw_message_id;

-- Step 10: Rename foreign key constraint in receiver_statuses
ALTER TABLE receiver_statuses RENAME CONSTRAINT receiver_statuses_aprs_message_id_fkey TO receiver_statuses_raw_message_id_fkey;

-- Step 11: Rename index in receiver_statuses
ALTER INDEX IF EXISTS idx_receiver_statuses_aprs_message_id RENAME TO idx_receiver_statuses_raw_message_id;

-- Step 12: Update partman configuration for the renamed table
UPDATE partman.part_config
SET parent_table = 'public.raw_messages'
WHERE parent_table = 'public.aprs_messages';

-- Step 13: Create index on source column for filtering
CREATE INDEX idx_raw_messages_source ON raw_messages (source);

COMMENT ON TABLE raw_messages IS 'Stores raw protocol messages from both APRS and ADS-B Beast sources';
COMMENT ON COLUMN raw_messages.source IS 'Protocol source: aprs or beast';
