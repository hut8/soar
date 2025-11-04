-- Create GIST index on fixes.location_geom column for fast spatial queries
-- This is split into a separate migration from the column creation
-- to separate the fast schema changes from the slow index build
CREATE INDEX fixes_location_geom_idx ON fixes USING GIST (location_geom);
