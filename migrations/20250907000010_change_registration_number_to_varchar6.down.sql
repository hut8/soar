-- Revert registration_number column size from varchar(6) back to varchar(5)
-- WARNING: This may cause data loss if any registration numbers exceed 5 characters

ALTER TABLE aircraft_registrations 
ALTER COLUMN registration_number TYPE VARCHAR(5);