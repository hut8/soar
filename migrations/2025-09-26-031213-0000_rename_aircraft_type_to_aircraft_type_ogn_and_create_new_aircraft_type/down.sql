-- Reverse the changes from the up migration

-- Step 1: Drop the index
DROP INDEX IF EXISTS aircraft_registrations_aw_class_idx;

-- Step 2: Add back the type_aircraft_code column to aircraft_registrations
ALTER TABLE aircraft_registrations ADD COLUMN type_aircraft_code character(1);

-- Step 3: Recreate the index with type_aircraft_code
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations USING btree (airworthiness_class, type_aircraft_code);

-- Step 4: Recreate the foreign key constraint
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_aircraft_code_fkey
    FOREIGN KEY (type_aircraft_code) REFERENCES type_aircraft(code);

-- Step 5: Remove aircraft_type column from aircraft_registrations table
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS aircraft_type;

-- Step 6: Drop the new aircraft_type enum
DROP TYPE IF EXISTS aircraft_type;

-- Step 7: Rename the column in fixes table back
ALTER TABLE fixes RENAME COLUMN aircraft_type_ogn TO aircraft_type;

-- Step 8: Rename the aircraft_type_ogn enum back to aircraft_type
ALTER TYPE aircraft_type_ogn RENAME TO aircraft_type;
