-- Validate the fixes_aircraft_id_fkey constraint which was added as NOT VALID.
--
-- VALIDATE CONSTRAINT is blocked on TimescaleDB hypertables with compression enabled,
-- even if no chunks are currently compressed. Instead, we drop the NOT VALID constraint
-- and re-add it without NOT VALID, which validates all existing rows at creation time.

-- safety-assured:start
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aircraft_id_fkey;
-- safety-assured:end

ALTER TABLE fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id);
