-- Fix message_source enum: rename ogn->aprs, adsb->beast, and add sbs
--
-- The previous migration incorrectly renamed the values. This migration:
-- - Renames 'ogn' back to 'aprs' (OGN uses APRS protocol)
-- - Renames 'adsb' to 'beast' (Beast is the binary protocol format)
-- - Adds 'sbs' (SBS-1 BaseStation CSV format)

-- Step 1: Create new enum type with correct values
CREATE TYPE message_source_new AS ENUM ('aprs', 'beast', 'sbs');

-- Step 2: Add temporary column with new enum type
ALTER TABLE raw_messages ADD COLUMN source_new message_source_new;

-- Step 3: Migrate data from old enum to new enum
UPDATE raw_messages
SET source_new = CASE
    WHEN source = 'ogn' THEN 'aprs'::message_source_new
    WHEN source = 'adsb' THEN 'beast'::message_source_new
END;

-- Step 4: Make new column NOT NULL
ALTER TABLE raw_messages ALTER COLUMN source_new SET NOT NULL;

-- Step 5: Drop old column and enum
ALTER TABLE raw_messages DROP COLUMN source;
DROP TYPE message_source;

-- Step 6: Rename new column and enum to original names
ALTER TABLE raw_messages RENAME COLUMN source_new TO source;
ALTER TYPE message_source_new RENAME TO message_source;

-- Step 7: Set default for new rows to 'aprs' (most common)
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'aprs'::message_source;

-- Step 8: Recreate index on source column
CREATE INDEX idx_raw_messages_source ON raw_messages (source);

COMMENT ON COLUMN raw_messages.source IS 'Protocol source: aprs (APRS/OGN text), beast (ADS-B Beast binary), or sbs (SBS-1 BaseStation CSV)';
