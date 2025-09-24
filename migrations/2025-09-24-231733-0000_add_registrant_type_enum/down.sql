-- Revert RegistrantType enum migration

-- Add back the original character column
ALTER TABLE aircraft_registrations
ADD COLUMN type_registration_enum_backup char(1);

-- Migrate data back from enum to character codes
UPDATE aircraft_registrations
SET type_registration_enum_backup = CASE
    WHEN type_registration_code = 'individual' THEN '1'
    WHEN type_registration_code = 'partnership' THEN '2'
    WHEN type_registration_code = 'corporation' THEN '3'
    WHEN type_registration_code = 'co_owned' THEN '4'
    WHEN type_registration_code = 'government' THEN '5'
    WHEN type_registration_code = 'llc' THEN '7'
    WHEN type_registration_code = 'non_citizen_corporation' THEN '8'
    WHEN type_registration_code = 'non_citizen_co_owned' THEN '9'
    ELSE NULL
END
WHERE type_registration_code IS NOT NULL;

-- Drop the enum column
ALTER TABLE aircraft_registrations
DROP COLUMN type_registration_code;

-- Rename the backup column to the original name
ALTER TABLE aircraft_registrations
RENAME COLUMN type_registration_enum_backup TO type_registration_code;

-- Drop the enum type
DROP TYPE registrant_type;
