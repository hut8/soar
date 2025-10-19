-- Drop the existing foreign key constraint
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_flight_id_fkey;

-- Re-add the foreign key constraint with ON DELETE SET NULL
ALTER TABLE fixes
    ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id)
    REFERENCES flights(id)
    ON DELETE SET NULL;
