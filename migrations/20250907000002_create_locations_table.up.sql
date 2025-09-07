-- Add up migration script here
-- =========================================================
-- Create locations table to normalize address data
-- =========================================================

-- 1. Create the locations table
CREATE TABLE locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    street1 TEXT,
    street2 TEXT,
    city TEXT,
    state TEXT,
    zip_code TEXT,
    region_code TEXT,
    county_mail_code TEXT,
    country_mail_code TEXT,
    geolocation POINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Create unique constraint on address fields (treating NULL values as equal for uniqueness)
-- Create the index after we insert the data to avoid conflicts
-- CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
--     COALESCE(street1, ''),
--     COALESCE(street2, ''),
--     COALESCE(city, ''),
--     COALESCE(state, ''),
--     COALESCE(zip_code, ''),
--     COALESCE(country_mail_code, 'US')
-- );

-- 3. Create index on geolocation for spatial queries
CREATE INDEX locations_geolocation_idx ON locations USING GIST (geolocation);

-- 4. Insert unique address combinations from aircraft_registrations
-- First, create a temporary table to hold unique addresses
CREATE TEMPORARY TABLE temp_unique_addresses AS
SELECT 
    street1, street2, city, state, zip_code, region_code,
    county_mail_code, country_mail_code,
    MIN(registration_number) as first_registration
FROM aircraft_registrations
WHERE street1 IS NOT NULL 
   OR street2 IS NOT NULL 
   OR city IS NOT NULL 
   OR state IS NOT NULL 
   OR zip_code IS NOT NULL
GROUP BY 
    street1, street2, city, state, zip_code, region_code,
    county_mail_code, country_mail_code;

-- Then insert into locations, joining back to get registered_location
INSERT INTO locations (
    street1, street2, city, state, zip_code, region_code,
    county_mail_code, country_mail_code, geolocation
)
SELECT 
    ua.street1, ua.street2, ua.city, ua.state, ua.zip_code, ua.region_code,
    ua.county_mail_code, ua.country_mail_code, 
    COALESCE(
        (SELECT registered_location FROM aircraft_registrations ar1 
         WHERE COALESCE(ar1.street1, '') = COALESCE(ua.street1, '') 
           AND COALESCE(ar1.street2, '') = COALESCE(ua.street2, '') 
           AND COALESCE(ar1.city, '') = COALESCE(ua.city, '') 
           AND COALESCE(ar1.state, '') = COALESCE(ua.state, '') 
           AND COALESCE(ar1.zip_code, '') = COALESCE(ua.zip_code, '') 
           AND COALESCE(ar1.country_mail_code, 'US') = COALESCE(ua.country_mail_code, 'US')
           AND ar1.registered_location IS NOT NULL
         LIMIT 1),
        (SELECT registered_location FROM aircraft_registrations ar2
         WHERE ar2.registration_number = ua.first_registration)
    ) as geolocation
FROM temp_unique_addresses ua;

-- 5. Add location_id foreign key column to aircraft_registrations
ALTER TABLE aircraft_registrations 
ADD COLUMN location_id UUID REFERENCES locations(id);

-- 6. Add location_id foreign key column to clubs
ALTER TABLE clubs
ADD COLUMN location_id UUID REFERENCES locations(id);

-- 7. Update aircraft_registrations to reference the appropriate location
UPDATE aircraft_registrations 
SET location_id = l.id
FROM locations l
WHERE COALESCE(aircraft_registrations.street1, '') = COALESCE(l.street1, '')
  AND COALESCE(aircraft_registrations.street2, '') = COALESCE(l.street2, '')
  AND COALESCE(aircraft_registrations.city, '') = COALESCE(l.city, '')
  AND COALESCE(aircraft_registrations.state, '') = COALESCE(l.state, '')
  AND COALESCE(aircraft_registrations.zip_code, '') = COALESCE(l.zip_code, '')
  AND COALESCE(aircraft_registrations.country_mail_code, 'US') = COALESCE(l.country_mail_code, 'US');

-- 8. Update clubs to reference the appropriate location
UPDATE clubs 
SET location_id = l.id
FROM locations l
WHERE COALESCE(clubs.street1, '') = COALESCE(l.street1, '')
  AND COALESCE(clubs.street2, '') = COALESCE(l.street2, '')
  AND COALESCE(clubs.city, '') = COALESCE(l.city, '')
  AND COALESCE(clubs.state, '') = COALESCE(l.state, '')
  AND COALESCE(clubs.zip_code, '') = COALESCE(l.zip_code, '')
  AND COALESCE(clubs.country_mail_code, 'US') = COALESCE(l.country_mail_code, 'US');

-- 9. Create indexes for foreign key lookups
CREATE INDEX aircraft_registrations_location_id_idx ON aircraft_registrations (location_id);
CREATE INDEX clubs_location_id_idx ON clubs (location_id);

-- 10. Now create the unique constraint on address fields
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    COALESCE(street1, ''),
    COALESCE(street2, ''),
    COALESCE(city, ''),
    COALESCE(state, ''),
    COALESCE(zip_code, ''),
    COALESCE(country_mail_code, 'US')
);