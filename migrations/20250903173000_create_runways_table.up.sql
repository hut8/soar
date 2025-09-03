-- Create runways table based on OurAirports data dictionary
-- https://ourairports.com/help/data-dictionary.html

-- Enable PostGIS extension for spatial queries (if not already enabled)
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE runways (
    id INTEGER PRIMARY KEY,
    airport_ref INTEGER NOT NULL,
    airport_ident VARCHAR(7) NOT NULL,
    length_ft INTEGER,
    width_ft INTEGER,
    surface VARCHAR(10),
    lighted BOOLEAN NOT NULL DEFAULT FALSE,
    closed BOOLEAN NOT NULL DEFAULT FALSE,

    -- Low-numbered end of runway
    le_ident VARCHAR(7),
    le_latitude_deg DECIMAL(10, 8),
    le_longitude_deg DECIMAL(11, 8),
    le_location GEOGRAPHY(POINT, 4326), -- PostGIS point for low end
    le_elevation_ft INTEGER,
    le_heading_degt DECIMAL(5, 2), -- Heading in degrees true
    le_displaced_threshold_ft INTEGER,

    -- High-numbered end of runway
    he_ident VARCHAR(7),
    he_latitude_deg DECIMAL(10, 8),
    he_longitude_deg DECIMAL(11, 8),
    he_location GEOGRAPHY(POINT, 4326), -- PostGIS point for high end
    he_elevation_ft INTEGER,
    he_heading_degt DECIMAL(5, 2), -- Heading in degrees true
    he_displaced_threshold_ft INTEGER,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Foreign key to airports table
    CONSTRAINT fk_runway_airport_ref FOREIGN KEY (airport_ref) REFERENCES airports(id),
    CONSTRAINT fk_runway_airport_ident FOREIGN KEY (airport_ident) REFERENCES airports(ident)
);

-- Create indexes for common queries
CREATE INDEX idx_runways_airport_ref ON runways(airport_ref);
CREATE INDEX idx_runways_airport_ident ON runways(airport_ident);
CREATE INDEX idx_runways_surface ON runways(surface);
CREATE INDEX idx_runways_lighted ON runways(lighted) WHERE lighted = TRUE;
CREATE INDEX idx_runways_closed ON runways(closed);
CREATE INDEX idx_runways_length ON runways(length_ft) WHERE length_ft IS NOT NULL;

-- Spatial indexes for runway endpoints
CREATE INDEX idx_runways_le_location_gist ON runways USING GIST(le_location) WHERE le_location IS NOT NULL;
CREATE INDEX idx_runways_he_location_gist ON runways USING GIST(he_location) WHERE he_location IS NOT NULL;

-- Function to automatically populate location from lat/lng coordinates for runway ends
CREATE OR REPLACE FUNCTION update_runway_locations()
RETURNS TRIGGER AS $$
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
$$ LANGUAGE plpgsql;

-- Trigger to automatically update locations when lat/lng changes
CREATE TRIGGER update_runway_locations_trigger
    BEFORE INSERT OR UPDATE OF le_latitude_deg, le_longitude_deg, he_latitude_deg, he_longitude_deg ON runways
    FOR EACH ROW
    EXECUTE FUNCTION update_runway_locations();

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_runways_updated_at
    BEFORE UPDATE ON runways
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
