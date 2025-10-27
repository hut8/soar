-- Drop the index first
DROP INDEX IF EXISTS idx_devices_tracker_device_type;

-- Drop the tracker_device_type column from devices table
ALTER TABLE devices DROP COLUMN tracker_device_type;
