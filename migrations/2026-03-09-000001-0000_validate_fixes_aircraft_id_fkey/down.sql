-- There is no way to "un-validate" a constraint, so we must drop and re-add as NOT VALID.
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aircraft_id_fkey;
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) NOT VALID;
