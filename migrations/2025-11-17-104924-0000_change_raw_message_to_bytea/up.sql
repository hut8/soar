-- Change raw_message column from TEXT to BYTEA to support both ASCII APRS and binary Beast messages
-- APRS messages will be UTF-8 encoded to bytes
-- Beast messages will be stored as raw binary

-- Step 1: Add new BYTEA column
ALTER TABLE raw_messages ADD COLUMN raw_message_binary BYTEA;

-- Step 2: Migrate existing TEXT data to BYTEA (UTF-8 encode APRS messages)
-- This converts all existing APRS text messages to their UTF-8 byte representation
UPDATE raw_messages SET raw_message_binary = convert_to(raw_message, 'UTF8');

-- Step 3: Make new column NOT NULL (all data has been migrated)
ALTER TABLE raw_messages ALTER COLUMN raw_message_binary SET NOT NULL;

-- Step 4: Drop old TEXT column
ALTER TABLE raw_messages DROP COLUMN raw_message;

-- Step 5: Rename new column to original name
ALTER TABLE raw_messages RENAME COLUMN raw_message_binary TO raw_message;

COMMENT ON COLUMN raw_messages.raw_message IS 'Raw message data as bytes. For APRS: UTF-8 encoded ASCII text. For Beast: Raw binary ADS-B frames.';
