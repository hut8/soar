-- Reverse the changes from up.sql

-- Drop the index
DROP INDEX IF EXISTS idx_flights_device_id;

-- Remove the foreign key constraint
ALTER TABLE flights DROP CONSTRAINT IF EXISTS flights_device_id_fkey;

-- Remove the device_id column
ALTER TABLE flights DROP COLUMN IF EXISTS device_id;

-- Make takeoff_time NOT NULL again (note: this will fail if there are null values)
-- In practice, this rollback may require data cleanup first
ALTER TABLE flights ALTER COLUMN takeoff_time SET NOT NULL;
