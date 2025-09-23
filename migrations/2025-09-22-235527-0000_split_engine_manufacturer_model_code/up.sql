-- Split eng_mfr_mdl_code into engine_manufacturer_code and engine_model_code
-- Positions 46-48: engine_manufacturer_code (3 characters)
-- Positions 49-50: engine_model_code (2 characters)

-- Add the new columns
ALTER TABLE aircraft_registrations
ADD COLUMN engine_manufacturer_code VARCHAR(3),
ADD COLUMN engine_model_code VARCHAR(2);

-- Migrate existing data
UPDATE aircraft_registrations
SET
    engine_manufacturer_code = CASE
        WHEN eng_mfr_mdl_code IS NOT NULL AND LENGTH(eng_mfr_mdl_code) >= 3
        THEN SUBSTRING(eng_mfr_mdl_code FROM 1 FOR 3)
        ELSE NULL
    END,
    engine_model_code = CASE
        WHEN eng_mfr_mdl_code IS NOT NULL AND LENGTH(eng_mfr_mdl_code) >= 5
        THEN SUBSTRING(eng_mfr_mdl_code FROM 4 FOR 2)
        ELSE NULL
    END;

-- Drop the old column
ALTER TABLE aircraft_registrations DROP COLUMN eng_mfr_mdl_code;