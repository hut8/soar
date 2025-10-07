-- Add back string identifier columns
ALTER TABLE flights ADD COLUMN departure_airport VARCHAR(10);
ALTER TABLE flights ADD COLUMN arrival_airport VARCHAR(10);

-- Populate from foreign keys
UPDATE flights
SET departure_airport = airports.ident
FROM airports
WHERE flights.departure_airport_id = airports.id;

UPDATE flights
SET arrival_airport = airports.ident
FROM airports
WHERE flights.arrival_airport_id = airports.id;

-- Drop foreign key columns
ALTER TABLE flights DROP COLUMN departure_airport_id;
ALTER TABLE flights DROP COLUMN arrival_airport_id;
