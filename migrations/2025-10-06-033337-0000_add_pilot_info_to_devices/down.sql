-- Remove pilot information and radio frequency columns from devices table

ALTER TABLE devices
DROP COLUMN IF EXISTS frequency_mhz,
DROP COLUMN IF EXISTS pilot_name,
DROP COLUMN IF EXISTS home_base_airport_ident;
