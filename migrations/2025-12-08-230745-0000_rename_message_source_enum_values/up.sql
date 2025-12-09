-- Rename message_source enum values from 'aprs'/'beast' to 'ogn'/'adsb'
--
-- This migration renames the enum values to better reflect what they represent:
-- - 'aprs' -> 'ogn' (OGN network messages)
-- - 'beast' -> 'adsb' (ADS-B messages)

-- Step 1: Create new enum type with desired values
CREATE TYPE message_source_new AS ENUM ('ogn', 'adsb');

-- Step 2: Add temporary column with new enum type
ALTER TABLE raw_messages ADD COLUMN source_new message_source_new;

-- Step 3: Migrate data from old enum to new enum
UPDATE raw_messages
SET source_new = CASE
    WHEN source = 'aprs' THEN 'ogn'::message_source_new
    WHEN source = 'beast' THEN 'adsb'::message_source_new
END;

-- Step 4: Make new column NOT NULL
ALTER TABLE raw_messages ALTER COLUMN source_new SET NOT NULL;

-- Step 5: Drop old column and enum
ALTER TABLE raw_messages DROP COLUMN source;
DROP TYPE message_source;

-- Step 6: Rename new column and enum to original names
ALTER TABLE raw_messages RENAME COLUMN source_new TO source;
ALTER TYPE message_source_new RENAME TO message_source;

-- Step 7: Set default for new rows to 'ogn' (most common)
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'ogn'::message_source;
