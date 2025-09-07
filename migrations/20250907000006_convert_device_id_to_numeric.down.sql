-- Add down migration script here
-- =========================================================
-- Revert device_id from INTEGER back to TEXT
-- =========================================================

-- Drop the primary key constraint temporarily
ALTER TABLE devices DROP CONSTRAINT devices_pkey;

-- Change device_id from INTEGER back to TEXT
ALTER TABLE devices ALTER COLUMN device_id TYPE TEXT USING device_id::TEXT;

-- Recreate the primary key constraint
ALTER TABLE devices ADD CONSTRAINT devices_pkey PRIMARY KEY (device_id);

-- Note: Data will be lost during rollback since we truncated in the up migration