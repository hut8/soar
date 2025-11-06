-- Make raw_message_hash NOT NULL with trigger-based default
-- This assumes that production database has no more NULL values in this column

-- Step 1: Create a trigger function that computes hash if NULL
-- This acts as a safety net if hash is not provided by application
CREATE OR REPLACE FUNCTION compute_aprs_message_hash()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.raw_message_hash IS NULL THEN
        NEW.raw_message_hash := digest(NEW.raw_message, 'sha256');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Step 2: Create trigger to run before insert
CREATE TRIGGER ensure_aprs_message_hash
    BEFORE INSERT ON aprs_messages
    FOR EACH ROW
    EXECUTE FUNCTION compute_aprs_message_hash();

-- Step 3: Make the column NOT NULL
-- This will fail if there are any remaining NULL values
ALTER TABLE aprs_messages
ALTER COLUMN raw_message_hash SET NOT NULL;

-- Update the column comment
COMMENT ON COLUMN aprs_messages.raw_message_hash IS
'SHA-256 hash of raw_message for efficient deduplication (computed by application or trigger if not provided)';
