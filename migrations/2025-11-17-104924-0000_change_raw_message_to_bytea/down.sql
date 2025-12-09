-- Revert raw_message column from BYTEA back to TEXT

-- Step 1: Add new TEXT column
ALTER TABLE raw_messages ADD COLUMN raw_message_text TEXT;

-- Step 2: Migrate BYTEA data back to TEXT (UTF-8 decode)
-- This converts byte data back to text (works for APRS, Beast will be garbled but that's expected on rollback)
UPDATE raw_messages SET raw_message_text = convert_from(raw_message, 'UTF8');

-- Step 3: Make new column NOT NULL
ALTER TABLE raw_messages ALTER COLUMN raw_message_text SET NOT NULL;

-- Step 4: Drop BYTEA column
ALTER TABLE raw_messages DROP COLUMN raw_message;

-- Step 5: Rename text column back to original name
ALTER TABLE raw_messages RENAME COLUMN raw_message_text TO raw_message;
