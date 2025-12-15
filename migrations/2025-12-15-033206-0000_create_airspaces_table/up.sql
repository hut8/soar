-- OpenAIP Airspace Integration
-- Data Source: OpenAIP (https://www.openaip.net)
-- License: CC BY-NC 4.0 (Attribution-NonCommercial 4.0 International)
-- https://creativecommons.org/licenses/by-nc/4.0/

-- ICAO Airspace Classifications
CREATE TYPE airspace_class AS ENUM (
    'A',  -- Class A: IFR only, clearance required
    'B',  -- Class B: IFR and VFR, clearance required for all
    'C',  -- Class C: IFR and VFR, clearance required for VFR
    'D',  -- Class D: IFR and VFR, radio contact required
    'E',  -- Class E: IFR and VFR, controlled airspace
    'F',  -- Class F: IFR and VFR (advisory services)
    'G',  -- Class G: IFR and VFR (uncontrolled)
    'SUA' -- Special Use Airspace (unclassified)
);

-- OpenAIP Airspace Types (37 types as per API documentation)
CREATE TYPE airspace_type AS ENUM (
    'Restricted',      -- R: Restricted Area
    'Danger',          -- D: Danger Area
    'Prohibited',      -- P: Prohibited Area
    'CTR',             -- Control Zone
    'TMZ',             -- Transponder Mandatory Zone
    'RMZ',             -- Radio Mandatory Zone
    'TMA',             -- Terminal Control Area
    'ATZ',             -- Aerodrome Traffic Zone
    'MATZ',            -- Military Aerodrome Traffic Zone
    'Airway',          -- Airways
    'MTR',             -- Military Training Route
    'AlertArea',       -- Alert Area
    'WarningArea',     -- Warning Area
    'ProtectedArea',   -- Protected Area
    'HTZ',             -- Helicopter Traffic Zone
    'GliderProhibited',-- Glider Prohibited Area
    'GliderSector',    -- Glider Sector
    'NoGliders',       -- No Gliders Zone
    'WaveWindow',      -- Wave Window
    'Other',           -- Other/Unspecified
    'FIR',             -- Flight Information Region
    'UIR',             -- Upper Information Region
    'ADIZ',            -- Air Defense Identification Zone
    'ATZ_P',           -- ATZ Penetrable
    'ATZ_MBZ',         -- ATZ/MBZ
    'TFR',             -- Temporary Flight Restriction
    'TRA',             -- Temporary Reserved Area
    'TSA',             -- Temporary Segregated Area
    'FIS',             -- Flight Information Service
    'UAS',             -- Unmanned Aircraft System Zone
    'RFFS',            -- Radio Free Flight Service
    'Sport',           -- Sport Aviation Area
    'DropZone',        -- Parachute Drop Zone
    'Gliding',         -- Gliding Area
    'MilitaryOps',     -- Military Operations Area
    'NotAssigned'      -- Not Assigned/Unknown
);

-- Altitude Reference Systems
CREATE TYPE altitude_reference AS ENUM (
    'MSL',  -- Mean Sea Level
    'AGL',  -- Above Ground Level
    'STD',  -- Standard (Flight Level)
    'GND',  -- Ground
    'UNL'   -- Unlimited
);

-- Main airspaces table
CREATE TABLE airspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    openaip_id INTEGER NOT NULL UNIQUE,

    -- Basic information
    name TEXT NOT NULL,
    airspace_class airspace_class,
    airspace_type airspace_type NOT NULL,
    country_code CHAR(2),  -- ISO 3166-1 alpha-2

    -- Altitude limits (lower boundary)
    lower_value INTEGER,              -- Altitude value (feet for MSL/AGL, flight level for STD)
    lower_unit TEXT,                  -- 'FT' or 'FL'
    lower_reference altitude_reference,

    -- Altitude limits (upper boundary)
    upper_value INTEGER,
    upper_unit TEXT,
    upper_reference altitude_reference,

    -- PostGIS Geometry - CRITICAL: First polygon type in SOAR!
    -- Using GEOGRAPHY for consistency with existing airport/fix patterns
    -- MultiPolygon handles airspaces with multiple boundaries
    geometry GEOGRAPHY(MultiPolygon, 4326) NOT NULL,

    -- Generated GEOMETRY column for faster bounding box queries
    -- Pattern from fixes table: allows use of && operator with GIST index
    geometry_geom GEOMETRY(MultiPolygon, 4326) GENERATED ALWAYS AS (
        geometry::geometry
    ) STORED,

    -- Metadata from OpenAIP
    remarks TEXT,                     -- Additional information
    activity_type TEXT,               -- Activity type (e.g., "MILITARY", "CIVIL")

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    openaip_updated_at TIMESTAMPTZ   -- Last update time from OpenAIP (for incremental sync)
);

-- GIST spatial indexes for efficient queries
CREATE INDEX idx_airspaces_geometry ON airspaces USING GIST (geometry);
CREATE INDEX idx_airspaces_geometry_geom ON airspaces USING GIST (geometry_geom);

-- Regular indexes for common queries
CREATE INDEX idx_airspaces_country ON airspaces (country_code);
CREATE INDEX idx_airspaces_type ON airspaces (airspace_type);
CREATE INDEX idx_airspaces_class ON airspaces (airspace_class);
CREATE INDEX idx_airspaces_openaip_updated ON airspaces (openaip_updated_at);

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_airspaces_updated_at
    BEFORE UPDATE ON airspaces
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Table and column comments for documentation
COMMENT ON TABLE airspaces IS 'Airspace boundaries from OpenAIP - CC BY-NC 4.0 license (non-commercial use only)';
COMMENT ON COLUMN airspaces.geometry IS 'Airspace boundary as GEOGRAPHY MultiPolygon (WGS84/EPSG:4326)';
COMMENT ON COLUMN airspaces.geometry_geom IS 'Cached GEOMETRY for faster bounding box queries using && operator';
COMMENT ON COLUMN airspaces.openaip_id IS 'OpenAIP internal ID - used for upserts during sync';
COMMENT ON COLUMN airspaces.openaip_updated_at IS 'Last modification time from OpenAIP - used for incremental sync';
