-- Create airports table based on OurAirports data dictionary
-- https://ourairports.com/help/data-dictionary.html

-- Enable PostGIS extension for spatial queries
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE airports (
    id INTEGER PRIMARY KEY,
    ident VARCHAR(7) NOT NULL UNIQUE,
    type VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    latitude_deg DECIMAL(10, 8),
    longitude_deg DECIMAL(11, 8),
    location GEOGRAPHY(POINT, 4326), -- PostGIS point for efficient spatial queries
    elevation_ft INTEGER,
    continent VARCHAR(2),
    iso_country VARCHAR(2),
    iso_region VARCHAR(7),
    municipality VARCHAR(255),
    scheduled_service BOOLEAN NOT NULL DEFAULT FALSE,
    gps_code VARCHAR(4),
    icao_code VARCHAR(4),
    iata_code VARCHAR(3),
    local_code VARCHAR(7),
    home_link TEXT,
    wikipedia_link TEXT,
    keywords TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for common queries
CREATE INDEX idx_airports_ident ON airports(ident);
CREATE INDEX idx_airports_type ON airports(type);
CREATE INDEX idx_airports_iso_country ON airports(iso_country);
CREATE INDEX idx_airports_iso_region ON airports(iso_region);
CREATE INDEX idx_airports_iata_code ON airports(iata_code) WHERE iata_code IS NOT NULL;
CREATE INDEX idx_airports_icao_code ON airports(icao_code) WHERE icao_code IS NOT NULL;
CREATE INDEX idx_airports_gps_code ON airports(gps_code) WHERE gps_code IS NOT NULL;
CREATE INDEX idx_airports_scheduled_service ON airports(scheduled_service) WHERE scheduled_service = TRUE;
CREATE INDEX idx_airports_municipality ON airports(municipality);

-- Spatial index for efficient nearest-neighbor queries
CREATE INDEX idx_airports_location_gist ON airports USING GIST(location) WHERE location IS NOT NULL;

-- Function to automatically populate location from lat/lng coordinates
CREATE OR REPLACE FUNCTION update_airport_location()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.latitude_deg IS NOT NULL AND NEW.longitude_deg IS NOT NULL THEN
        NEW.location = ST_SetSRID(ST_MakePoint(NEW.longitude_deg, NEW.latitude_deg), 4326)::geography;
    ELSE
        NEW.location = NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically update location when lat/lng changes
CREATE TRIGGER update_airport_location_trigger
    BEFORE INSERT OR UPDATE OF latitude_deg, longitude_deg ON airports
    FOR EACH ROW
    EXECUTE FUNCTION update_airport_location();

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_airports_updated_at
    BEFORE UPDATE ON airports
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
