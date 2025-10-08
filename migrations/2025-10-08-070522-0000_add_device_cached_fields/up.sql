-- Add cached fields to devices table for quick lookups

-- Cache the aircraft type from most recent fix
ALTER TABLE devices ADD COLUMN aircraft_type_ogn aircraft_type_ogn;

-- Track when the last fix was received for this device
ALTER TABLE devices ADD COLUMN last_fix_at TIMESTAMPTZ;

-- Create index for finding active devices (devices with recent fixes)
CREATE INDEX idx_devices_last_fix_at ON devices(last_fix_at) WHERE last_fix_at IS NOT NULL;
