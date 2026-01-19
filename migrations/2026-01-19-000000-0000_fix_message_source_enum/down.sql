-- Revert message_source enum: aprs->ogn, beast->adsb, remove sbs

-- Step 1: Create old enum type (without sbs)
CREATE TYPE message_source_old AS ENUM ('ogn', 'adsb');

-- Step 2: Add temporary column with old enum type
ALTER TABLE raw_messages ADD COLUMN source_old message_source_old;

-- Step 3: Migrate data back from new enum to old enum
-- Note: Any 'sbs' rows will be converted to 'adsb' (data loss)
UPDATE raw_messages
SET source_old = CASE
    WHEN source = 'aprs' THEN 'ogn'::message_source_old
    WHEN source = 'beast' THEN 'adsb'::message_source_old
    WHEN source = 'sbs' THEN 'adsb'::message_source_old
END;

-- Step 4: Make old column NOT NULL
ALTER TABLE raw_messages ALTER COLUMN source_old SET NOT NULL;

-- Step 5: Drop new column and enum
ALTER TABLE raw_messages DROP COLUMN source;
DROP TYPE message_source;

-- Step 6: Rename old column and enum back
ALTER TABLE raw_messages RENAME COLUMN source_old TO source;
ALTER TYPE message_source_old RENAME TO message_source;

-- Step 7: Set default back to 'ogn'
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'ogn'::message_source;

-- Step 8: Recreate index on source column
CREATE INDEX idx_raw_messages_source ON raw_messages (source);

COMMENT ON COLUMN raw_messages.source IS 'Protocol source: ogn or adsb';
