-- Rename existing aircraft_type enum to aircraft_type_ogn
-- and create new aircraft_type enum for FAA aircraft types

-- Step 1: Rename the existing aircraft_type enum to aircraft_type_ogn
ALTER TYPE aircraft_type RENAME TO aircraft_type_ogn;

-- Step 2: Rename the column in fixes table
ALTER TABLE fixes RENAME COLUMN aircraft_type TO aircraft_type_ogn;

-- Step 3: Create new aircraft_type enum for FAA aircraft types
CREATE TYPE aircraft_type AS ENUM (
    'glider',
    'balloon',
    'blimp_dirigible',
    'fixed_wing_single_engine',
    'fixed_wing_multi_engine',
    'rotorcraft',
    'weight_shift_control',
    'powered_parachute',
    'gyroplane',
    'hybrid_lift',
    'other'
);

-- Step 4: Add aircraft_type column to aircraft_registrations table
ALTER TABLE aircraft_registrations ADD COLUMN aircraft_type aircraft_type;

-- Step 5: Remove the type_aircraft_code column from aircraft_registrations
-- (First drop the foreign key constraint and index)
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_type_aircraft_code_fkey;
DROP INDEX IF EXISTS aircraft_registrations_aw_class_idx;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS type_aircraft_code;

-- Step 6: Recreate the index without type_aircraft_code
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations USING btree (airworthiness_class);