-- safety-assured:start
ALTER TABLE receivers ADD COLUMN protocols TEXT[] NOT NULL DEFAULT '{}';
UPDATE receivers SET protocols = ARRAY[software] WHERE software IS NOT NULL;
ALTER TABLE receivers DROP COLUMN software;
-- safety-assured:end

-- Update the trigger function to reference protocols instead of the dropped software column.
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
            OLD.geocoded, OLD.protocols)
        IS NOT DISTINCT FROM
           (NEW.callsign, NEW.description, NEW.contact, NEW.email,
            NEW.ogn_db_country, NEW.from_ogn_db, NEW.location,
            NEW.latitude, NEW.longitude, NEW.street_address,
            NEW.city, NEW.region, NEW.country, NEW.postal_code,
            NEW.geocoded, NEW.protocols)
    THEN
        -- Only latest_packet_at (or updated_at itself) changed — preserve old updated_at
        NEW.updated_at = OLD.updated_at;
    ELSE
        NEW.updated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$;
