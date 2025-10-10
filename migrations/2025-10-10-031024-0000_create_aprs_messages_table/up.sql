-- Create aprs_messages table to store raw APRS message data
CREATE TABLE aprs_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    raw_message TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on received_at for time-based queries
CREATE INDEX idx_aprs_messages_received_at ON aprs_messages(received_at);

-- Create updated_at trigger
CREATE TRIGGER set_aprs_messages_updated_at
    BEFORE UPDATE ON aprs_messages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add aprs_message_id foreign key to fixes table
ALTER TABLE fixes
    ADD COLUMN aprs_message_id UUID REFERENCES aprs_messages(id) ON DELETE SET NULL;

-- Create index on aprs_message_id in fixes for joins
CREATE INDEX idx_fixes_aprs_message_id ON fixes(aprs_message_id);

-- Add aprs_message_id foreign key to receiver_statuses table
ALTER TABLE receiver_statuses
    ADD COLUMN aprs_message_id UUID REFERENCES aprs_messages(id) ON DELETE SET NULL;

-- Create index on aprs_message_id in receiver_statuses for joins
CREATE INDEX idx_receiver_statuses_aprs_message_id ON receiver_statuses(aprs_message_id);
