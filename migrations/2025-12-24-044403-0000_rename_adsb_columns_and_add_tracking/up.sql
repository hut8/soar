-- Rename from_ddb to from_ogn_ddb for clarity
ALTER TABLE aircraft RENAME COLUMN from_ddb TO from_ogn_ddb;

-- Add new column to track if aircraft came from ADS-B Exchange database
ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS from_adsbx_ddb BOOLEAN NOT NULL DEFAULT false;

-- Add comments
COMMENT ON COLUMN aircraft.from_ogn_ddb IS 'Indicates if this aircraft record came from OGN/FlarmNet DDB';
COMMENT ON COLUMN aircraft.from_adsbx_ddb IS 'Indicates if this aircraft record came from ADS-B Exchange database';
