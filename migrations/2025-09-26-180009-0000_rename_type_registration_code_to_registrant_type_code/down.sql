-- Revert renaming of type_registration_code column back to original name
ALTER TABLE aircraft_registrations RENAME COLUMN registrant_type_code TO type_registration_code;
