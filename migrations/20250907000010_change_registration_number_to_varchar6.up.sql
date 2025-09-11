-- Change registration_number column size from varchar(5) to varchar(6)
-- This accommodates longer aircraft registration numbers

ALTER TABLE aircraft_registrations 
ALTER COLUMN registration_number TYPE VARCHAR(6);