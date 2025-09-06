-- Add up migration script here
-- =========================================================
-- Update clubs table with additional fields
-- =========================================================

-- Add is_soaring boolean field
ALTER TABLE clubs ADD COLUMN is_soaring BOOLEAN DEFAULT FALSE;

-- Add home_base_airport_id as foreign key to airports table
ALTER TABLE clubs ADD COLUMN home_base_airport_id UUID REFERENCES airports(id) ON DELETE SET NULL;

-- Add base_location as WGS84 point
ALTER TABLE clubs ADD COLUMN base_location POINT;

-- Add address columns (matching aircraft registration table)
ALTER TABLE clubs ADD COLUMN street1 VARCHAR(255);
ALTER TABLE clubs ADD COLUMN street2 VARCHAR(255);
ALTER TABLE clubs ADD COLUMN city VARCHAR(100);
ALTER TABLE clubs ADD COLUMN state VARCHAR(10);
ALTER TABLE clubs ADD COLUMN zip_code VARCHAR(20);
ALTER TABLE clubs ADD COLUMN region_code VARCHAR(10);
ALTER TABLE clubs ADD COLUMN county_mail_code VARCHAR(10);
ALTER TABLE clubs ADD COLUMN country_mail_code VARCHAR(10);

-- Set is_soaring to true for clubs containing soaring-related keywords
UPDATE clubs 
SET is_soaring = TRUE 
WHERE UPPER(name) LIKE '%SOAR%' 
   OR UPPER(name) LIKE '%GLIDING%' 
   OR UPPER(name) LIKE '%SAILPLANE%' 
   OR UPPER(name) LIKE '%GLIDER%';

-- Create index on is_soaring for faster lookups
CREATE INDEX clubs_is_soaring_idx ON clubs (is_soaring);

-- Create index on home_base_airport_id
CREATE INDEX clubs_home_base_airport_id_idx ON clubs (home_base_airport_id);

-- Create spatial index on base_location
CREATE INDEX clubs_base_location_idx ON clubs USING GIST (base_location);