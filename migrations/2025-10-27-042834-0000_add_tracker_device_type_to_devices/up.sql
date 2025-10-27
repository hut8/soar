-- Add tracker_device_type to devices table
-- This is the APRS packet destination (e.g., "OGFLR", "OGADSB") that indicates
-- what type of tracker device is transmitting the position data
ALTER TABLE devices ADD COLUMN tracker_device_type TEXT;

-- Create index for lookups by tracker_device_type
CREATE INDEX idx_devices_tracker_device_type ON devices(tracker_device_type) WHERE tracker_device_type IS NOT NULL;
