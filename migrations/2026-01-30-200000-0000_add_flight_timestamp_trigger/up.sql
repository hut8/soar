-- Trigger to update flight timestamps when a fix is inserted.
-- This eliminates a separate DB round trip from the application:
-- the flight's created_at and last_fix_at are updated atomically
-- as part of the fix INSERT transaction.
--
-- Uses LEAST/GREATEST to handle out-of-order fixes:
-- - created_at = earliest fix timestamp
-- - last_fix_at = latest fix timestamp
--
-- The fixes table is a TimescaleDB hypertable. TimescaleDB automatically
-- propagates row-level triggers from the parent to all existing and
-- future chunks.

CREATE OR REPLACE FUNCTION update_flight_timestamp_on_fix_insert()
RETURNS trigger AS $$
BEGIN
    IF NEW.flight_id IS NOT NULL THEN
        UPDATE flights
        SET created_at = LEAST(created_at, NEW.received_at),
            last_fix_at = GREATEST(last_fix_at, NEW.received_at),
            updated_at = NOW()
        WHERE id = NEW.flight_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_flight_timestamp
AFTER INSERT ON fixes
FOR EACH ROW
EXECUTE FUNCTION update_flight_timestamp_on_fix_insert();
