-- Rename airworthiness_class enum values from PascalCase to lowercase
-- to match diesel-derive-enum v3.0.0-beta.1 serialization behavior

-- First, create a new enum with lowercase values
CREATE TYPE airworthiness_class_new AS ENUM (
    'standard',
    'limited',
    'restricted',
    'experimental',
    'provisional',
    'multiple',
    'primary',
    'special_flight_permit',
    'light_sport'
);

-- Update the table to use the new enum, converting values
ALTER TABLE aircraft_registrations
ADD COLUMN airworthiness_class_new airworthiness_class_new;

UPDATE aircraft_registrations
SET airworthiness_class_new = CASE
    WHEN airworthiness_class = 'Standard' THEN 'standard'::airworthiness_class_new
    WHEN airworthiness_class = 'Limited' THEN 'limited'::airworthiness_class_new
    WHEN airworthiness_class = 'Restricted' THEN 'restricted'::airworthiness_class_new
    WHEN airworthiness_class = 'Experimental' THEN 'experimental'::airworthiness_class_new
    WHEN airworthiness_class = 'Provisional' THEN 'provisional'::airworthiness_class_new
    WHEN airworthiness_class = 'Multiple' THEN 'multiple'::airworthiness_class_new
    WHEN airworthiness_class = 'Primary' THEN 'primary'::airworthiness_class_new
    WHEN airworthiness_class = 'Special Flight Permit' THEN 'special_flight_permit'::airworthiness_class_new
    WHEN airworthiness_class = 'Light Sport' THEN 'light_sport'::airworthiness_class_new
END;

-- Drop the old column and rename the new one
ALTER TABLE aircraft_registrations DROP COLUMN airworthiness_class;
ALTER TABLE aircraft_registrations RENAME COLUMN airworthiness_class_new TO airworthiness_class;

-- Drop the old enum type and rename the new one
DROP TYPE airworthiness_class;
ALTER TYPE airworthiness_class_new RENAME TO airworthiness_class;

-- Recreate the index
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations USING btree (airworthiness_class, type_aircraft_code);
