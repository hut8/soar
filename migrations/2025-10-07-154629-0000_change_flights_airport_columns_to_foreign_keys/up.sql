-- Add new foreign key columns for airports
ALTER TABLE flights ADD COLUMN departure_airport_id INTEGER REFERENCES airports(id);
ALTER TABLE flights ADD COLUMN arrival_airport_id INTEGER REFERENCES airports(id);

-- Populate new columns from existing string identifiers
UPDATE flights
SET departure_airport_id = airports.id
FROM airports
WHERE flights.departure_airport = airports.ident;

UPDATE flights
SET arrival_airport_id = airports.id
FROM airports
WHERE flights.arrival_airport = airports.ident;

-- Drop old string identifier columns
ALTER TABLE flights DROP COLUMN departure_airport;
ALTER TABLE flights DROP COLUMN arrival_airport;
