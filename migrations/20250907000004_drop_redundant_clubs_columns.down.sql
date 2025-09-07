-- Add down migration script here
-- =========================================================
-- Restore redundant address columns to clubs table
-- =========================================================

-- Re-add the address columns that were removed
ALTER TABLE clubs ADD COLUMN street1 TEXT;
ALTER TABLE clubs ADD COLUMN street2 TEXT;
ALTER TABLE clubs ADD COLUMN city TEXT;
ALTER TABLE clubs ADD COLUMN state TEXT;
ALTER TABLE clubs ADD COLUMN zip_code TEXT;
ALTER TABLE clubs ADD COLUMN region_code TEXT;
ALTER TABLE clubs ADD COLUMN county_mail_code TEXT;
ALTER TABLE clubs ADD COLUMN country_mail_code TEXT;
ALTER TABLE clubs ADD COLUMN base_location POINT;

-- Repopulate the columns from the locations table
UPDATE clubs 
SET 
    street1 = l.street1,
    street2 = l.street2,
    city = l.city,
    state = l.state,
    zip_code = l.zip_code,
    region_code = l.region_code,
    county_mail_code = l.county_mail_code,
    country_mail_code = l.country_mail_code,
    base_location = l.geolocation
FROM locations l
WHERE clubs.location_id = l.id;