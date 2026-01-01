-- Remove constraint
ALTER TABLE aircraft
DROP CONSTRAINT IF EXISTS icao_model_code_length_check;

-- Change column type back to TEXT
ALTER TABLE aircraft
ALTER COLUMN icao_model_code TYPE TEXT;

COMMENT ON COLUMN aircraft.icao_model_code IS 'ICAO aircraft type designator (e.g., C172, PA28, B737)';
