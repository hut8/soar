-- Add up migration script here
-- Add flight_id foreign key to fixes table
ALTER TABLE fixes 
ADD COLUMN flight_id UUID REFERENCES flights(id);

-- Index for flight_id lookups
CREATE INDEX fixes_flight_id_idx ON fixes (flight_id);
