-- Re-add the foreign key constraint (NOT VALID to avoid scanning existing rows)
ALTER TABLE fixes ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id) REFERENCES flights(id) NOT VALID;
