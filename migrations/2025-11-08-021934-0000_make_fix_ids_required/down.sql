-- Revert receiver_id and aprs_message_id back to nullable
ALTER TABLE fixes ALTER COLUMN receiver_id DROP NOT NULL;
ALTER TABLE fixes ALTER COLUMN aprs_message_id DROP NOT NULL;
