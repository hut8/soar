-- Create index on fixes.device_id for faster device-related queries
-- This improves performance when looking up all fixes for a specific device
CREATE INDEX IF NOT EXISTS fixes_device_id_idx ON fixes (device_id);
