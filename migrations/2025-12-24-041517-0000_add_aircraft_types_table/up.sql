-- Create aircraft_types table to store ICAO/IATA type code definitions
CREATE TABLE aircraft_types (
    icao_code TEXT PRIMARY KEY,
    iata_code TEXT,
    description TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Add index on IATA code for lookups
CREATE INDEX idx_aircraft_types_iata_code ON aircraft_types(iata_code);

-- Add index on description for search
CREATE INDEX idx_aircraft_types_description ON aircraft_types(description);
