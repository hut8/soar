-- Rename aircraft_id to device_address and device_type to address_type in fixes table
ALTER TABLE fixes RENAME COLUMN aircraft_id TO device_address;
ALTER TABLE fixes RENAME COLUMN device_type TO address_type;
