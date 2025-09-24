-- Create RegistrantType enum
CREATE TYPE registrant_type AS ENUM (
    'individual',
    'partnership',
    'corporation',
    'co_owned',
    'government',
    'llc',
    'non_citizen_corporation',
    'non_citizen_co_owned',
    'unknown'
);

-- Add new column with the enum type (nullable)
ALTER TABLE aircraft_registrations
ADD COLUMN type_registration_enum registrant_type;

-- Migrate existing data from single character codes to enum values
UPDATE aircraft_registrations
SET type_registration_enum = CASE
    WHEN type_registration_code = '1' THEN 'individual'::registrant_type
    WHEN type_registration_code = '2' THEN 'partnership'::registrant_type
    WHEN type_registration_code = '3' THEN 'corporation'::registrant_type
    WHEN type_registration_code = '4' THEN 'co_owned'::registrant_type
    WHEN type_registration_code = '5' THEN 'government'::registrant_type
    WHEN type_registration_code = '7' THEN 'llc'::registrant_type
    WHEN type_registration_code = '8' THEN 'non_citizen_corporation'::registrant_type
    WHEN type_registration_code = '9' THEN 'non_citizen_co_owned'::registrant_type
    ELSE 'unknown'::registrant_type
END
WHERE type_registration_code IS NOT NULL;

-- Drop the old character column
ALTER TABLE aircraft_registrations
DROP COLUMN type_registration_code;

-- Rename the new column to the original name
ALTER TABLE aircraft_registrations
RENAME COLUMN type_registration_enum TO type_registration_code;
