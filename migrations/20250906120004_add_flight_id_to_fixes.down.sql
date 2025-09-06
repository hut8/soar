-- Add down migration script here
-- Remove flight_id foreign key from fixes table
ALTER TABLE fixes 
DROP COLUMN IF EXISTS flight_id;
