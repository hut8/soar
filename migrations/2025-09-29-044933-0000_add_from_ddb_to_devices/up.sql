-- Add from_ddb column to devices table
-- This boolean column indicates if the device entry is from the "real" devices database (DDB URLs)
ALTER TABLE devices ADD COLUMN from_ddb BOOLEAN NOT NULL DEFAULT true;

-- Create an index for potential filtering queries
CREATE INDEX idx_devices_from_ddb ON devices(from_ddb);
