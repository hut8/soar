-- Add up migration script here
-- =========================================================
-- Drop redundant address columns from aircraft_registrations table
-- =========================================================

-- Drop redundant address fields from aircraft_registrations table
-- These are now normalized in the locations table via location_id foreign key

ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS street1;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS street2;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS city;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS state;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS zip_code;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS region_code;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS county_mail_code;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS country_mail_code;

-- Also drop the registered_location column since this is now in locations.geolocation
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS registered_location;