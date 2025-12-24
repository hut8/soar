-- Clean up any icao_model_code values that are not 3 or 4 characters
-- Set invalid values to NULL before adding constraint
UPDATE aircraft
SET icao_model_code = NULL
WHERE icao_model_code IS NOT NULL
  AND LENGTH(icao_model_code) NOT IN (3, 4);

-- Change column type from TEXT to VARCHAR(4)
ALTER TABLE aircraft
ALTER COLUMN icao_model_code TYPE VARCHAR(4);

-- Add constraint that icao_model_code is either NULL, or 3-4 characters
ALTER TABLE aircraft
ADD CONSTRAINT icao_model_code_length_check
CHECK (icao_model_code IS NULL OR LENGTH(icao_model_code) IN (3, 4));

COMMENT ON COLUMN aircraft.icao_model_code IS 'ICAO aircraft type designator (e.g., C172, PA28, B737) - must be 3 or 4 characters';
