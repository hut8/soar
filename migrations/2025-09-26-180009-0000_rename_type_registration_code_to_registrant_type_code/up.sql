-- Rename type_registration_code column to registrant_type_code in aircraft_registrations table
ALTER TABLE aircraft_registrations RENAME COLUMN type_registration_code TO registrant_type_code;
