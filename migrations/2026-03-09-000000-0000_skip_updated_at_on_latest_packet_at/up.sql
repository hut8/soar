-- Replace the generic update_updated_at_column trigger on receivers
-- with one that skips updated_at when only latest_packet_at changed.
-- latest_packet_at is updated every ~60 seconds per receiver and is not
-- a meaningful "record modification", so it should not bump updated_at.

CREATE OR REPLACE FUNCTION update_receivers_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- Skip updated_at bump if the only change is to latest_packet_at
    IF NEW IS DISTINCT FROM OLD
       AND (OLD.callsign, OLD.description, OLD.contact, OLD.email,
            OLD.ogn_db_country, OLD.from_ogn_db, OLD.location,
            OLD.latitude, OLD.longitude, OLD.street_address,
            OLD.city, OLD.region, OLD.country, OLD.postal_code,
            OLD.geocoded, OLD.software)
        IS NOT DISTINCT FROM
           (NEW.callsign, NEW.description, NEW.contact, NEW.email,
            NEW.ogn_db_country, NEW.from_ogn_db, NEW.location,
            NEW.latitude, NEW.longitude, NEW.street_address,
            NEW.city, NEW.region, NEW.country, NEW.postal_code,
            NEW.geocoded, NEW.software)
    THEN
        -- Only latest_packet_at (or updated_at itself) changed — preserve old updated_at
        NEW.updated_at = OLD.updated_at;
    ELSE
        NEW.updated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$;

-- Replace the trigger to use the new function
DROP TRIGGER IF EXISTS update_receivers_updated_at ON receivers;
CREATE TRIGGER update_receivers_updated_at
    BEFORE UPDATE ON receivers
    FOR EACH ROW
    EXECUTE FUNCTION update_receivers_updated_at_column();
