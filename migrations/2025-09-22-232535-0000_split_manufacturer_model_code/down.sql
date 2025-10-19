-- Reverse the migration

-- Rename aircraft_models table back to aircraft_model
ALTER TABLE aircraft_models RENAME TO aircraft_model;

-- Add back the mfr_mdl_code column
ALTER TABLE aircraft_registrations
ADD COLUMN mfr_mdl_code VARCHAR(7);

-- Migrate data back from the split columns to mfr_mdl_code
UPDATE aircraft_registrations
SET mfr_mdl_code = CONCAT(
    COALESCE(manufacturer_code, ''),
    COALESCE(model_code, ''),
    COALESCE(series_code, '')
)
WHERE manufacturer_code IS NOT NULL OR model_code IS NOT NULL OR series_code IS NOT NULL;

-- Set to NULL if all parts are empty
UPDATE aircraft_registrations
SET mfr_mdl_code = NULL
WHERE mfr_mdl_code = '';

-- Drop the split columns
ALTER TABLE aircraft_registrations
DROP COLUMN manufacturer_code,
DROP COLUMN model_code,
DROP COLUMN series_code;
