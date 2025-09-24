-- Truncate existing flights data since we're adding a mandatory column
TRUNCATE TABLE flights CASCADE;

-- Rename aircraft_id to device_address and add device_address_type column to flights table
ALTER TABLE flights RENAME COLUMN aircraft_id TO device_address;
ALTER TABLE flights ADD COLUMN device_address_type address_type NOT NULL;

-- Update the device_address column to allow for longer hex addresses
ALTER TABLE flights ALTER COLUMN device_address TYPE VARCHAR(20);