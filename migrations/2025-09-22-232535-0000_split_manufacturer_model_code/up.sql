-- Split mfr_mdl_code into three separate columns
-- Positions (38-40) - manufacturer_code (3 chars)
-- Positions (41-42) - model_code (2 chars)
-- Positions (43-44) - series_code (2 chars)

-- Add the new columns first
ALTER TABLE aircraft_registrations
ADD COLUMN manufacturer_code VARCHAR(3),
ADD COLUMN model_code VARCHAR(2),
ADD COLUMN series_code VARCHAR(2);

-- Migrate existing data from mfr_mdl_code to the new columns
UPDATE aircraft_registrations
SET
    manufacturer_code = CASE
        WHEN LENGTH(mfr_mdl_code) >= 3 THEN SUBSTRING(mfr_mdl_code FROM 1 FOR 3)
        ELSE NULL
    END,
    model_code = CASE
        WHEN LENGTH(mfr_mdl_code) >= 5 THEN SUBSTRING(mfr_mdl_code FROM 4 FOR 2)
        ELSE NULL
    END,
    series_code = CASE
        WHEN LENGTH(mfr_mdl_code) >= 7 THEN SUBSTRING(mfr_mdl_code FROM 6 FOR 2)
        ELSE NULL
    END
WHERE mfr_mdl_code IS NOT NULL;

-- Drop the old column
ALTER TABLE aircraft_registrations DROP COLUMN mfr_mdl_code;

-- Rename aircraft_model table to aircraft_models for consistency
ALTER TABLE aircraft_model RENAME TO aircraft_models;
