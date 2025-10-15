-- Add receiver_id foreign key to aprs_messages table
ALTER TABLE aprs_messages
    ADD COLUMN receiver_id UUID REFERENCES receivers(id) ON DELETE SET NULL;

-- Create index on receiver_id for joins
CREATE INDEX idx_aprs_messages_receiver_id ON aprs_messages(receiver_id);

-- Add unparsed column to store raw unparsed message fragments
ALTER TABLE aprs_messages
    ADD COLUMN unparsed TEXT;

-- Drop the updated_at trigger since we're removing that column
DROP TRIGGER IF EXISTS set_aprs_messages_updated_at ON aprs_messages;

-- Remove created_at and updated_at columns
ALTER TABLE aprs_messages
    DROP COLUMN IF EXISTS created_at,
    DROP COLUMN IF EXISTS updated_at;
