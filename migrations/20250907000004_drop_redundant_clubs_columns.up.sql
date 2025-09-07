-- Add up migration script here
-- =========================================================
-- Drop redundant address columns from clubs table
-- =========================================================

-- Drop redundant address fields from clubs table
-- These are now normalized in the locations table via location_id foreign key

ALTER TABLE clubs DROP COLUMN IF EXISTS street1;
ALTER TABLE clubs DROP COLUMN IF EXISTS street2;
ALTER TABLE clubs DROP COLUMN IF EXISTS city;
ALTER TABLE clubs DROP COLUMN IF EXISTS state;
ALTER TABLE clubs DROP COLUMN IF EXISTS zip_code;
ALTER TABLE clubs DROP COLUMN IF EXISTS region_code;
ALTER TABLE clubs DROP COLUMN IF EXISTS county_mail_code;
ALTER TABLE clubs DROP COLUMN IF EXISTS country_mail_code;

-- Also drop the base_location column since this is now in locations.geolocation
ALTER TABLE clubs DROP COLUMN IF EXISTS base_location;