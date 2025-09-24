-- Make manufacturer_code, model_code, and series_code NOT NULL in aircraft_registrations
-- First, update any NULL values to empty strings (though there shouldn't be any)
UPDATE aircraft_registrations
SET manufacturer_code = ''
WHERE manufacturer_code IS NULL;

UPDATE aircraft_registrations
SET model_code = ''
WHERE model_code IS NULL;

UPDATE aircraft_registrations
SET series_code = ''
WHERE series_code IS NULL;

-- Now alter the columns to be NOT NULL
ALTER TABLE aircraft_registrations
ALTER COLUMN manufacturer_code SET NOT NULL;

ALTER TABLE aircraft_registrations
ALTER COLUMN model_code SET NOT NULL;

ALTER TABLE aircraft_registrations
ALTER COLUMN series_code SET NOT NULL;
