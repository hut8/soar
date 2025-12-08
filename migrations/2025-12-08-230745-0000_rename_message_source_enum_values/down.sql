-- Revert message_source enum values from 'ogn'/'adsb' back to 'aprs'/'beast'

-- Step 1: Create old enum type
CREATE TYPE message_source_old AS ENUM ('aprs', 'beast');

-- Step 2: Add temporary column with old enum type
ALTER TABLE raw_messages ADD COLUMN source_old message_source_old;

-- Step 3: Migrate data back from new enum to old enum
UPDATE raw_messages
SET source_old = CASE
    WHEN source = 'ogn' THEN 'aprs'::message_source_old
    WHEN source = 'adsb' THEN 'beast'::message_source_old
END;

-- Step 4: Make old column NOT NULL
ALTER TABLE raw_messages ALTER COLUMN source_old SET NOT NULL;

-- Step 5: Drop new column and enum
ALTER TABLE raw_messages DROP COLUMN source;
DROP TYPE message_source;

-- Step 6: Rename old column and enum back to original names
ALTER TABLE raw_messages RENAME COLUMN source_old TO source;
ALTER TYPE message_source_old RENAME TO message_source;

-- Step 7: Set default back to 'aprs'
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'aprs'::message_source;
