-- Add composite index on device_id and timestamp to improve query performance
CREATE INDEX IF NOT EXISTS idx_fixes_device_id_timestamp
ON fixes(device_id, timestamp);
