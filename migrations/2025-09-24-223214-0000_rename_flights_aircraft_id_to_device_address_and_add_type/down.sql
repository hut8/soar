-- Reverse the changes: remove device_address_type column and rename device_address back to aircraft_id
ALTER TABLE flights DROP COLUMN device_address_type;
ALTER TABLE flights RENAME COLUMN device_address TO aircraft_id;

-- Revert the device_address column back to original size
ALTER TABLE flights ALTER COLUMN aircraft_id TYPE VARCHAR(10);
