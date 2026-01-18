-- Migration: Update aircraft_types table for CSV data source
-- Adds manufacturer, wing_type, aircraft_category columns
-- Changes primary key from icao_code to (icao_code, iata_code) composite key

-- Create wing_type enum
CREATE TYPE wing_type AS ENUM ('fixed_wing', 'rotary_wing');

-- Create icao_aircraft_category enum (distinct from existing aircraft_category)
CREATE TYPE icao_aircraft_category AS ENUM ('airplane', 'helicopter');

-- Add new columns to aircraft_types table
ALTER TABLE aircraft_types
    ADD COLUMN manufacturer TEXT,
    ADD COLUMN wing_type wing_type,
    ADD COLUMN aircraft_category icao_aircraft_category;

-- Drop old primary key constraint (icao_code only)
ALTER TABLE aircraft_types DROP CONSTRAINT aircraft_types_pkey;

-- Handle NULL iata_codes: convert to empty string for composite key
UPDATE aircraft_types SET iata_code = '' WHERE iata_code IS NULL;

-- Make iata_code NOT NULL with empty string default
ALTER TABLE aircraft_types ALTER COLUMN iata_code SET NOT NULL;
ALTER TABLE aircraft_types ALTER COLUMN iata_code SET DEFAULT '';

-- Add composite primary key (icao_code, iata_code)
ALTER TABLE aircraft_types ADD PRIMARY KEY (icao_code, iata_code);
