-- Make takeoff_time nullable and add device_id foreign key to flights table
-- Also make departure_airport nullable since we don't have this data when program starts

-- Add device_id column as nullable UUID with foreign key to devices table
ALTER TABLE flights ADD COLUMN device_id UUID;

-- Add foreign key constraint with SET NULL on delete (so deleting a device doesn't delete flights)
ALTER TABLE flights ADD CONSTRAINT flights_device_id_fkey
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE SET NULL;

-- Make takeoff_time nullable since we don't have takeoff data for aircraft already airborne when program starts
ALTER TABLE flights ALTER COLUMN takeoff_time DROP NOT NULL;

-- Create index on device_id for performance
CREATE INDEX idx_flights_device_id ON flights(device_id);
