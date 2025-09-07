-- Add down migration script here
-- =========================================================
-- Restore redundant address columns to aircraft_registrations table
-- =========================================================

-- Re-add the address columns that were removed
ALTER TABLE aircraft_registrations ADD COLUMN street1 TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN street2 TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN city TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN state TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN zip_code TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN region_code TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN county_mail_code TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN country_mail_code TEXT;
ALTER TABLE aircraft_registrations ADD COLUMN registered_location POINT;

-- Repopulate the columns from the locations table
UPDATE aircraft_registrations 
SET 
    street1 = l.street1,
    street2 = l.street2,
    city = l.city,
    state = l.state,
    zip_code = l.zip_code,
    region_code = l.region_code,
    county_mail_code = l.county_mail_code,
    country_mail_code = l.country_mail_code,
    registered_location = l.geolocation
FROM locations l
WHERE aircraft_registrations.location_id = l.id;