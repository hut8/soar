-- Drop the new geometry columns and indexes
DROP INDEX IF EXISTS idx_runways_le_location_geom;
DROP INDEX IF EXISTS idx_runways_he_location_geom;
ALTER TABLE runways DROP COLUMN IF EXISTS le_location_geom;
ALTER TABLE runways DROP COLUMN IF EXISTS he_location_geom;

-- Restore the old geography columns
ALTER TABLE runways
    ADD COLUMN le_location geography(Point, 4326),
    ADD COLUMN he_location geography(Point, 4326);

-- Restore the trigger function
CREATE FUNCTION public.update_runway_locations() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- Update low end location
    IF NEW.le_latitude_deg IS NOT NULL AND NEW.le_longitude_deg IS NOT NULL THEN
        NEW.le_location = ST_SetSRID(ST_MakePoint(NEW.le_longitude_deg, NEW.le_latitude_deg), 4326)::geography;
    ELSE
        NEW.le_location = NULL;
    END IF;

    -- Update high end location
    IF NEW.he_latitude_deg IS NOT NULL AND NEW.he_longitude_deg IS NOT NULL THEN
        NEW.he_location = ST_SetSRID(ST_MakePoint(NEW.he_longitude_deg, NEW.he_latitude_deg), 4326)::geography;
    ELSE
        NEW.he_location = NULL;
    END IF;

    RETURN NEW;
END;
$$;

-- Restore the trigger
CREATE TRIGGER update_runway_locations_trigger
    BEFORE INSERT OR UPDATE OF le_latitude_deg, le_longitude_deg, he_latitude_deg, he_longitude_deg
    ON public.runways
    FOR EACH ROW
    EXECUTE FUNCTION public.update_runway_locations();

-- Restore the indexes
CREATE INDEX idx_runways_le_location_gist ON public.runways USING gist (le_location) WHERE (le_location IS NOT NULL);
CREATE INDEX idx_runways_he_location_gist ON public.runways USING gist (he_location) WHERE (he_location IS NOT NULL);

-- Populate the geography columns from existing lat/lon data
UPDATE runways SET
    le_latitude_deg = le_latitude_deg,
    he_latitude_deg = he_latitude_deg
WHERE le_latitude_deg IS NOT NULL OR he_latitude_deg IS NOT NULL;
