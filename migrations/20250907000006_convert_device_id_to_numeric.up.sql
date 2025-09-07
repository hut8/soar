-- Add up migration script here
-- =========================================================
-- Convert device_id from TEXT to INTEGER and truncate table
-- =========================================================

-- Truncate the devices table first as requested
TRUNCATE TABLE devices;

-- Drop the primary key constraint temporarily
ALTER TABLE devices DROP CONSTRAINT devices_pkey;

-- Change device_id from TEXT to INTEGER
-- 6-digit hex max value is 0xFFFFFF = 16777215, so INTEGER (max ~2.1B) is sufficient
ALTER TABLE devices ALTER COLUMN device_id TYPE INTEGER USING device_id::INTEGER;

-- Recreate the primary key constraint
ALTER TABLE devices ADD CONSTRAINT devices_pkey PRIMARY KEY (device_id);

-- Update the index name if needed (PostgreSQL should handle this automatically)
-- The existing indexes should be preserved