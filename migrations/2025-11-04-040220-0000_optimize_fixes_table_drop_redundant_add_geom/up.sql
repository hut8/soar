-- Optimize fixes table by dropping redundant columns and adding geometry column
-- This migration drops 4 columns that are redundant (they exist on devices table)
-- and adds a generated geometry column for faster spatial queries using the && operator

-- Drop redundant columns that exist on devices table
-- These are all available via device_id foreign key
ALTER TABLE fixes
    DROP COLUMN IF EXISTS address_type,
    DROP COLUMN IF EXISTS aircraft_type_ogn,
    DROP COLUMN IF EXISTS device_address,
    DROP COLUMN IF EXISTS registration;

-- Add generated geometry column for fast spatial queries
-- This allows using the && operator which is much faster than ST_Intersects with geography
ALTER TABLE fixes
    ADD COLUMN location_geom geometry(Point, 4326)
        GENERATED ALWAYS AS (
            ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)
        ) STORED;

-- Note: GIST index on location_geom column is created in a separate migration (2025-11-04-042950-0000_create_fixes_geom_index)
-- to separate the fast schema changes from the slow index build
