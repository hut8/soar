-- Add ICAO aircraft type code and owner/operator columns to aircraft table

ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS icao_type_code TEXT,
ADD COLUMN IF NOT EXISTS owner_operator TEXT;

-- Create index for ICAO type code lookups (where not null)
CREATE INDEX IF NOT EXISTS idx_aircraft_icao_type_code ON aircraft (icao_type_code) WHERE icao_type_code IS NOT NULL;

-- Create index for owner/operator searches (where not null)
CREATE INDEX IF NOT EXISTS idx_aircraft_owner_operator ON aircraft (owner_operator) WHERE owner_operator IS NOT NULL;

COMMENT ON COLUMN aircraft.icao_type_code IS 'ICAO aircraft type designator (e.g., C172, PA28, B737)';
COMMENT ON COLUMN aircraft.owner_operator IS 'Aircraft owner/operator name';
