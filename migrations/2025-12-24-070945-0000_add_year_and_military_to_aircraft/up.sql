-- Add year manufactured and military status columns to aircraft table
ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS year SMALLINT,
ADD COLUMN IF NOT EXISTS is_military BOOLEAN;

-- Create index for year queries (where not null)
CREATE INDEX IF NOT EXISTS idx_aircraft_year ON aircraft (year) WHERE year IS NOT NULL;

-- Create index for military aircraft queries (where true)
CREATE INDEX IF NOT EXISTS idx_aircraft_is_military ON aircraft (is_military) WHERE is_military = true;

-- Add comments
COMMENT ON COLUMN aircraft.year IS 'Year manufactured (FAA data is canonical, ADS-B Exchange used if FAA unavailable)';
COMMENT ON COLUMN aircraft.is_military IS 'Indicates if aircraft is military (from ADS-B Exchange or FAA data)';
