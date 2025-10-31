-- Rename country column to ogn_db_country
ALTER TABLE receivers RENAME COLUMN country TO ogn_db_country;

-- Add new location fields
ALTER TABLE receivers ADD COLUMN latitude DOUBLE PRECISION;
ALTER TABLE receivers ADD COLUMN longitude DOUBLE PRECISION;
ALTER TABLE receivers ADD COLUMN street_address TEXT;
ALTER TABLE receivers ADD COLUMN city TEXT;
ALTER TABLE receivers ADD COLUMN region TEXT;
ALTER TABLE receivers ADD COLUMN country TEXT;
ALTER TABLE receivers ADD COLUMN postal_code TEXT;
ALTER TABLE receivers ADD COLUMN geocoded BOOLEAN NOT NULL DEFAULT FALSE;

-- Update the existing index on country to use the new column name
DROP INDEX IF EXISTS idx_receivers_country;
CREATE INDEX idx_receivers_ogn_db_country ON receivers (ogn_db_country);

-- Add indexes for the new location fields
CREATE INDEX idx_receivers_city ON receivers (city);
CREATE INDEX idx_receivers_region ON receivers (region);
CREATE INDEX idx_receivers_country ON receivers (country);
CREATE INDEX idx_receivers_lat_lng ON receivers (latitude, longitude) WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
CREATE INDEX idx_receivers_geocoded ON receivers (geocoded) WHERE geocoded = FALSE;
