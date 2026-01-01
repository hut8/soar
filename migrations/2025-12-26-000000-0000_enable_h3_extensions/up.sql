-- Enable H3 extension for hierarchical hexagonal geospatial indexing
-- This extension provides functions for working with Uber's H3 spatial index system
CREATE EXTENSION IF NOT EXISTS h3 CASCADE;

-- Enable h3_postgis extension for PostGIS integration with H3
-- This provides functions like h3_polygon_to_cells() for efficient spatial queries
-- CASCADE automatically installs dependencies (postgis_raster, etc.)
CREATE EXTENSION IF NOT EXISTS h3_postgis CASCADE;
