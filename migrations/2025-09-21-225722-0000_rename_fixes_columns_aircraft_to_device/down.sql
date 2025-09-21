-- Revert the column renames in fixes table
ALTER TABLE fixes RENAME COLUMN device_address TO aircraft_id;
ALTER TABLE fixes RENAME COLUMN address_type TO device_type;
