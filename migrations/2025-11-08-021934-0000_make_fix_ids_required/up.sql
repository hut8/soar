-- Make receiver_id and aprs_message_id NOT NULL in fixes table
-- These are always provided by the generic processor before Fix creation

-- First, delete any rows with NULL receiver_id or aprs_message_id
-- (There shouldn't be any in production, but we need to clean up just in case)
DELETE FROM fixes WHERE receiver_id IS NULL OR aprs_message_id IS NULL;

-- Now make the columns NOT NULL
ALTER TABLE fixes ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE fixes ALTER COLUMN aprs_message_id SET NOT NULL;
