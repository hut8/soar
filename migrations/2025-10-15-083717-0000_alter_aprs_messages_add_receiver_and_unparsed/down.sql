-- Restore created_at and updated_at columns
ALTER TABLE aprs_messages
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Recreate the updated_at trigger
CREATE TRIGGER set_aprs_messages_updated_at
    BEFORE UPDATE ON aprs_messages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Remove unparsed column
ALTER TABLE aprs_messages
    DROP COLUMN IF EXISTS unparsed;

-- Remove receiver_id index and column
DROP INDEX IF EXISTS idx_aprs_messages_receiver_id;

ALTER TABLE aprs_messages
    DROP COLUMN IF EXISTS receiver_id;
