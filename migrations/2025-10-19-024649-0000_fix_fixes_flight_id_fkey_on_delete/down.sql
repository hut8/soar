-- Revert to the original foreign key constraint without ON DELETE SET NULL
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_flight_id_fkey;

-- Re-add the original foreign key constraint (no ON DELETE action)
ALTER TABLE fixes
    ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id)
    REFERENCES flights(id);
