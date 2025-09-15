-- Revert airworthiness_class enum values from lowercase back to PascalCase

-- First, create the original enum with PascalCase values
CREATE TYPE airworthiness_class_original AS ENUM (
    'Standard',
    'Limited',
    'Restricted',
    'Experimental',
    'Provisional',
    'Multiple',
    'Primary',
    'Special Flight Permit',
    'Light Sport'
);

-- Update the table to use the original enum, converting values back
ALTER TABLE aircraft_registrations
ADD COLUMN airworthiness_class_original airworthiness_class_original;

UPDATE aircraft_registrations
SET airworthiness_class_original = CASE
    WHEN airworthiness_class = 'standard' THEN 'Standard'::airworthiness_class_original
    WHEN airworthiness_class = 'limited' THEN 'Limited'::airworthiness_class_original
    WHEN airworthiness_class = 'restricted' THEN 'Restricted'::airworthiness_class_original
    WHEN airworthiness_class = 'experimental' THEN 'Experimental'::airworthiness_class_original
    WHEN airworthiness_class = 'provisional' THEN 'Provisional'::airworthiness_class_original
    WHEN airworthiness_class = 'multiple' THEN 'Multiple'::airworthiness_class_original
    WHEN airworthiness_class = 'primary' THEN 'Primary'::airworthiness_class_original
    WHEN airworthiness_class = 'special_flight_permit' THEN 'Special Flight Permit'::airworthiness_class_original
    WHEN airworthiness_class = 'light_sport' THEN 'Light Sport'::airworthiness_class_original
END;

-- Drop the current column and rename the original one
ALTER TABLE aircraft_registrations DROP COLUMN airworthiness_class;
ALTER TABLE aircraft_registrations RENAME COLUMN airworthiness_class_original TO airworthiness_class;

-- Drop the current enum type and rename the original one
DROP TYPE airworthiness_class;
ALTER TYPE airworthiness_class_original RENAME TO airworthiness_class;

-- Recreate the index
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations USING btree (airworthiness_class, type_aircraft_code);
