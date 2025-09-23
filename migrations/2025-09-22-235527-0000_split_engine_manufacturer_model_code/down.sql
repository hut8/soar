-- Reverse the split of engine manufacturer model code
-- Recreate the eng_mfr_mdl_code column

-- Add back the original column
ALTER TABLE aircraft_registrations
ADD COLUMN eng_mfr_mdl_code VARCHAR(5);

-- Migrate data back by concatenating the split columns
UPDATE aircraft_registrations
SET eng_mfr_mdl_code = CASE
    WHEN engine_manufacturer_code IS NOT NULL AND engine_model_code IS NOT NULL
    THEN engine_manufacturer_code || engine_model_code
    WHEN engine_manufacturer_code IS NOT NULL
    THEN engine_manufacturer_code
    ELSE NULL
END;

-- Drop the split columns
ALTER TABLE aircraft_registrations
DROP COLUMN engine_manufacturer_code,
DROP COLUMN engine_model_code;