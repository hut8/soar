-- Add up migration script here
-- =========================================================
-- Replace airworthiness_classes table with enum type
-- =========================================================

-- 1. Create the enum type with the current values
CREATE TYPE airworthiness_class AS ENUM (
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

-- 2. Add new column with enum type to aircraft_registrations table
ALTER TABLE aircraft_registrations
ADD COLUMN airworthiness_class airworthiness_class;

-- 3. Update the new column based on current foreign key relationships
UPDATE aircraft_registrations
SET airworthiness_class = CASE
    WHEN ac.code = '1' THEN 'Standard'::airworthiness_class
    WHEN ac.code = '2' THEN 'Limited'::airworthiness_class
    WHEN ac.code = '3' THEN 'Restricted'::airworthiness_class
    WHEN ac.code = '4' THEN 'Experimental'::airworthiness_class
    WHEN ac.code = '5' THEN 'Provisional'::airworthiness_class
    WHEN ac.code = '6' THEN 'Multiple'::airworthiness_class
    WHEN ac.code = '7' THEN 'Primary'::airworthiness_class
    WHEN ac.code = '8' THEN 'Special Flight Permit'::airworthiness_class
    WHEN ac.code = '9' THEN 'Light Sport'::airworthiness_class
END
FROM airworthiness_classes ac
WHERE aircraft_registrations.airworthiness_class_code = ac.code;

-- 4. Drop the foreign key constraint
ALTER TABLE aircraft_registrations
DROP CONSTRAINT aircraft_registrations_airworthiness_class_code_fkey;

-- 5. Drop the old column
ALTER TABLE aircraft_registrations
DROP COLUMN airworthiness_class_code;

-- 6. Drop the airworthiness_classes table
DROP TABLE airworthiness_classes;

-- 7. Update the index to use the new column
DROP INDEX IF EXISTS aircraft_registrations_aw_class_idx;
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations (airworthiness_class, type_aircraft_code);
