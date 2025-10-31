-- Drop new indexes
DROP INDEX IF EXISTS idx_receivers_geocoded;
DROP INDEX IF EXISTS idx_receivers_lat_lng;
DROP INDEX IF EXISTS idx_receivers_country;
DROP INDEX IF EXISTS idx_receivers_region;
DROP INDEX IF EXISTS idx_receivers_city;
DROP INDEX IF EXISTS idx_receivers_ogn_db_country;

-- Drop new location columns
ALTER TABLE receivers DROP COLUMN IF EXISTS geocoded;
ALTER TABLE receivers DROP COLUMN IF EXISTS postal_code;
ALTER TABLE receivers DROP COLUMN IF EXISTS country;
ALTER TABLE receivers DROP COLUMN IF EXISTS region;
ALTER TABLE receivers DROP COLUMN IF EXISTS city;
ALTER TABLE receivers DROP COLUMN IF EXISTS street_address;
ALTER TABLE receivers DROP COLUMN IF EXISTS longitude;
ALTER TABLE receivers DROP COLUMN IF EXISTS latitude;

-- Rename ogn_db_country back to country
ALTER TABLE receivers RENAME COLUMN ogn_db_country TO country;

-- Recreate the original index
CREATE INDEX idx_receivers_country ON receivers (country);
