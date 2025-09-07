-- Add up migration script here
-- =========================================================
-- Create locations table to normalize address data - SIMPLIFIED
-- =========================================================

-- 1. Create the locations table
CREATE TABLE IF NOT EXISTS locations (
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

-- 2. Create index on geolocation for spatial queries
CREATE INDEX IF NOT EXISTS locations_geolocation_idx ON locations USING GIST (geolocation);

-- 3. Add location_id foreign key column to aircraft_registrations (if not exists)
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'aircraft_registrations' AND column_name = 'location_id') THEN
        ALTER TABLE aircraft_registrations 
        ADD COLUMN location_id UUID REFERENCES locations(id);
    END IF;
END $$;

-- 4. Add location_id foreign key column to clubs (if not exists)
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'clubs' AND column_name = 'location_id') THEN
        ALTER TABLE clubs
        ADD COLUMN location_id UUID REFERENCES locations(id);
    END IF;
END $$;

-- 5. Create indexes for foreign key lookups
CREATE INDEX IF NOT EXISTS aircraft_registrations_location_id_idx ON aircraft_registrations (location_id);
CREATE INDEX IF NOT EXISTS clubs_location_id_idx ON clubs (location_id);